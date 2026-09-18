use std::ops::Deref;

use syn::{Attribute, FnArg, ImplItem, ImplItemFn, Item, ItemImpl, Type};

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
            push_remote_methods(&mut service.server_remote_methods, &mut item_impl, errors);
        }
        ImplForServerOrClient::Client => {
            push_remote_methods(&mut service.client_remote_methods, &mut item_impl, errors);
        }
        ImplForServerOrClient::Neither => {
            check_non_state_struct_impl_for_remote_method_attr(&item_impl, errors)
        }
    }

    service.rest.push(Item::Impl(item_impl));
}

fn push_remote_methods(
    dst: &mut Vec<RemoteMethod>,
    item_impl: &mut ItemImpl,
    errors: &mut Option<syn::Error>,
) {
    let mut contains_remote_methods = false;

    for item in item_impl.items.iter_mut() {
        if let ImplItem::Fn(impl_item_fn) = item
            && has_remote_method_attr(impl_item_fn, errors)
        {
            contains_remote_methods = true;

            match create_remote_method(impl_item_fn) {
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

/// Returns whether the impl block concerns the Server or the Client struct, or something else
fn is_impl_block_for_server_or_client(impl_item: &ItemImpl) -> ImplForServerOrClient {
    match impl_item.self_ty.deref() {
        Type::Path(type_path) => {
            if type_path.path.is_ident("Server") {
                ImplForServerOrClient::Server
            } else if type_path.path.is_ident("Client") {
                ImplForServerOrClient::Client
            } else {
                ImplForServerOrClient::Neither
            }
        }
        _ => ImplForServerOrClient::Neither,
    }
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

fn create_remote_method(function: &ImplItemFn) -> syn::Result<RemoteMethod> {
    let mut errors = None;

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
                    &mut errors,
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

    create_result(remote_method, errors)
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
