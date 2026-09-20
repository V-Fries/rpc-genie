use std::ops::Deref;

use proc_macro2::TokenStream;
use quote::quote;
use syn::{
    Attribute, FnArg, ImplItem, ImplItemFn, Item, ItemImpl, Safety, Signature, Type, WhereClause,
    parse_quote, punctuated::Punctuated, token::Comma,
};

use crate::service::{RemoteMethod, service_parser::error_helpers::create_result};

use super::{ServiceBuilder, combine_errors};

enum ImplForServerOrClient {
    Server,
    Client,
    Neither,
}

pub fn parse_impl_item(
    service: &mut ServiceBuilder,
    mut item_impl: ItemImpl,
    errors: &mut Option<syn::Error>,
) {
    match is_impl_block_for_server_or_client(&item_impl) {
        ImplForServerOrClient::Server => {
            push_remote_methods(
                &mut service.server_remote_methods,
                &mut item_impl,
                errors,
                "ClientStub",
                quote!(ClientStub),
            );
            add_where_clause("Server", &mut item_impl.generics.where_clause, errors);
        }
        ImplForServerOrClient::Client => {
            push_remote_methods(
                &mut service.client_remote_methods,
                &mut item_impl,
                errors,
                "ServerStub",
                quote!(ServerStub),
            );
            add_where_clause("Client", &mut item_impl.generics.where_clause, errors);
        }
        ImplForServerOrClient::Neither => {
            check_non_state_struct_impl_for_remote_method_attr(&item_impl, errors)
        }
    }

    service.rest.push(Item::Impl(item_impl));
}

/// Returns whether the impl block concerns the Server or the Client struct, or something else
fn is_impl_block_for_server_or_client(item_impl: &ItemImpl) -> ImplForServerOrClient {
    let Type::Path(type_path) = item_impl.self_ty.deref() else {
        return ImplForServerOrClient::Neither;
    };

    if type_path.path.segments.len() != 1 {
        return ImplForServerOrClient::Neither;
    }

    match type_path
        .path
        .segments
        .first()
        .map(|segment| &segment.ident)
    {
        Some(ident) if ident == "Server" => ImplForServerOrClient::Server,
        Some(ident) if ident == "Client" => ImplForServerOrClient::Client,
        _ => ImplForServerOrClient::Neither,
    }
}

fn push_remote_methods(
    dst: &mut Vec<RemoteMethod>,
    item_impl: &mut ItemImpl,
    errors: &mut Option<syn::Error>,
    opposite_stub_alias: &str,
    opposite_stub_generic_handle: TokenStream,
) {
    let mut contains_remote_methods = false;

    for item in item_impl.items.iter_mut() {
        if let ImplItem::Fn(impl_item_fn) = item
            && has_remote_method_attr(impl_item_fn, errors)
        {
            contains_remote_methods = true;

            match parse_remote_method(
                impl_item_fn,
                opposite_stub_alias,
                &opposite_stub_generic_handle,
            ) {
                Err(err) => combine_errors(errors, err),
                Ok(remote_method) => dst.push(remote_method),
            }

            impl_item_fn
                .attrs
                .retain(|attr| !attr.path().is_ident("remote_method"));
        }
    }

    if contains_remote_methods {
        for attr in item_impl.attrs.iter() {
            combine_errors(
                errors,
                syn::Error::new_spanned(
                    attr,
                    "Using an attribute on an impl block which contains methods marked with \
                     #[remote_method] is not allowed",
                ),
            );
        }
    }
}

fn has_remote_method_attr(impl_item_fn: &ImplItemFn, errors: &mut Option<syn::Error>) -> bool {
    let remote_method_attrs = impl_item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("remote_method"))
        .collect::<Vec<_>>();

    match remote_method_attrs.as_slice() {
        [] => return false,
        [remote_method_attr] => check_remote_method_attr_args(remote_method_attr, errors),
        _ => {
            combine_errors(
                errors,
                syn::Error::new_spanned(
                    impl_item_fn,
                    "#[remote_method] attribute should only be present once",
                ),
            );
        }
    };

    if impl_item_fn.attrs.len() != remote_method_attrs.len() {
        combine_errors(
            errors,
            syn::Error::new_spanned(
                impl_item_fn,
                "When #[remote_method] attribute is used, other attributes are not allowed",
            ),
        );
    }

    true
}

/// #[remote_method] attributes should not have any arguments so this returns an error if there are
/// any
fn check_remote_method_attr_args(attr: &Attribute, errors: &mut Option<syn::Error>) {
    match attr.meta {
        syn::Meta::Path(_) => (),
        syn::Meta::List(ref list) => combine_errors(
            errors,
            syn::Error::new_spanned(list, "#[remote_method] does not take any arguments"),
        ),
        syn::Meta::NameValue(ref name_value) => combine_errors(
            errors,
            syn::Error::new_spanned(name_value, "#[remote_method] does not take any arguments"),
        ),
    }
}

fn parse_remote_method(
    function: &mut ImplItemFn,
    opposite_stub_alias: &str,
    opposite_stub_generic_handle: &TokenStream,
) -> syn::Result<RemoteMethod> {
    let mut errors = None;

    let remote_method = remote_method_from(function, &mut errors);

    replace_opposite_stub_alias_with_actual_type(
        &mut function.sig.inputs,
        opposite_stub_alias,
        opposite_stub_generic_handle,
    );

    check_remote_method_signature(&function.sig, &mut errors);

    create_result(remote_method, errors)
}

fn remote_method_from(function: &ImplItemFn, errors: &mut Option<syn::Error>) -> RemoteMethod {
    let mut remote_method = RemoteMethod {
        vis: function.vis.clone(),
        is_async: function.sig.asyncness.is_some(),
        ident: function.sig.ident.clone(),
        receiver: None,
        args: Vec::new(),
        output: function.sig.output.clone(),
    };

    let mut inputs = function.sig.inputs.iter();

    match inputs.next() {
        Some(FnArg::Receiver(receiver)) => remote_method.receiver = Some(receiver.clone()),
        Some(FnArg::Typed(typed)) => remote_method.args.push(typed.clone()),
        None => {}
    }

    for input in inputs {
        match input {
            FnArg::Receiver(_) => {
                combine_errors(
                    errors,
                    syn::Error::new_spanned(
                        input,
                        "rpc_genie crate has a bug, please create an issue with the prototype of \
                         your method so that we can look into it",
                    ),
                );
            }
            FnArg::Typed(typed) => remote_method.args.push(typed.clone()),
        }
    }

    remote_method
}

fn replace_opposite_stub_alias_with_actual_type(
    inputs: &mut Punctuated<FnArg, Comma>,
    opposite_stub_alias: &str,
    opposite_stub_generic_handle: &TokenStream,
) {
    *inputs = inputs
        .iter()
        .cloned()
        .map(|mut input| match input {
            FnArg::Typed(ref mut pat_type) => match pat_type.ty.deref() {
                Type::Path(type_path) if type_path.path.is_ident(opposite_stub_alias) => {
                    pat_type.ty =
                        parse_quote!(&std::sync::Arc<#opposite_stub_generic_handle<RequestSender>>);
                    input
                }
                _ => input,
            },
            _ => input,
        })
        .collect();
}

fn check_remote_method_signature(signature: &Signature, errors: &mut Option<syn::Error>) {
    if let Some(const_keyword) = signature.constness {
        combine_errors(
            errors,
            syn::Error::new_spanned(
                const_keyword,
                "Remote methods may not be const. A const function can be evaluated at compile \
                 time, so there is no reason to ever call it on a remote device",
            ),
        );
    }

    if let Safety::Unsafe(unsafe_keyword) = signature.safety {
        combine_errors(
            errors,
            syn::Error::new_spanned(unsafe_keyword, "Remote methods may not be unsafe"),
        );
    }

    if !signature.generics.params.is_empty() {
        combine_errors(
            errors,
            syn::Error::new_spanned(
                &signature.generics.params,
                "Remote methods may not have generic parameters",
            ),
        );
    }

    if let Some(ref where_clause) = signature.generics.where_clause {
        combine_errors(
            errors,
            syn::Error::new_spanned(where_clause, "Remote methods may not have a where clause"),
        );
    }

    if let Some(ref variadic) = signature.variadic {
        combine_errors(
            errors,
            syn::Error::new_spanned(variadic, "Remote methods may not have variadic arguments"),
        );
    }
}

fn check_non_state_struct_impl_for_remote_method_attr(
    item_impl: &ItemImpl,
    errors: &mut Option<syn::Error>,
) {
    for item in item_impl.items.iter() {
        if let ImplItem::Fn(impl_item_fn) = item {
            check_non_state_method_for_remote_method_attr(impl_item_fn, errors);
        }
    }
}

fn check_non_state_method_for_remote_method_attr(
    impl_item_fn: &ImplItemFn,
    errors: &mut Option<syn::Error>,
) {
    for attr in impl_item_fn
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("remote_method"))
    {
        combine_errors(
            errors,
            syn::Error::new_spanned(
                attr,
                "Only the Server and Client structs may use the #[remote_method] attribute",
            ),
        );
    }
}

fn add_where_clause(
    struct_name: &str,
    where_clause: &mut Option<WhereClause>,
    errors: &mut Option<syn::Error>,
) {
    if let Some(where_clause) = where_clause {
        combine_errors(
            errors,
            syn::Error::new_spanned(
                where_clause,
                format!("Not allowed to have a where clause when implementing {struct_name}"),
            ),
        );
        return;
    }

    *where_clause = Some(parse_quote! {
        where
            RequestSender: rpc_genie::SubscribableStub + rpc_genie::SendRequest,
    });
}
