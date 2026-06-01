use std::ops::Deref;

use syn::{Attribute, FnArg, ImplItem, ImplItemFn, Item, ItemImpl, Type};

use crate::service::RemoteMethod;

use super::{ServiceBuilder, combine_errors};

enum ImplForServerOrClient {
    Server,
    Client,
    Neither,
}

pub fn parse_impl_item(
    service: &mut ServiceBuilder,
    impl_item: ItemImpl,
    errors: &mut Option<syn::Error>,
) {
    let has_remote_methods_attr = has_remote_methods_attr(&impl_item, errors);

    match is_impl_block_for_server_or_client(&impl_item) {
        ImplForServerOrClient::Server => {
            if has_remote_methods_attr {
                push_remote_methods_block_methods(
                    &mut service.server_remote_methods,
                    &impl_item,
                    errors,
                )
            }
        }
        ImplForServerOrClient::Client => {
            if has_remote_methods_attr {
                push_remote_methods_block_methods(
                    &mut service.client_remote_methods,
                    &impl_item,
                    errors,
                )
            }
        }
        ImplForServerOrClient::Neither => {
            if has_remote_methods_attr {
                combine_errors(
                    errors,
                    syn::Error::new_spanned(
                        impl_item,
                        "Only the Server and Client structs may use the #[remote_methods] attribute",
                    ),
                );
            } else {
                service.rest.push(Item::Impl(impl_item));
            }
        }
    }
}

fn has_remote_methods_attr(impl_item: &ItemImpl, errors: &mut Option<syn::Error>) -> bool {
    let maybe_remote_method_attr = impl_item
        .attrs
        .iter()
        .filter(|attr| attr.path().is_ident("remote_methods"))
        .collect::<Vec<_>>();

    match maybe_remote_method_attr.as_slice() {
        [] => return false,
        [remote_method_attr] => check_remote_methods_attr_args(remote_method_attr, errors),
        _ => {
            combine_errors(
                errors,
                syn::Error::new_spanned(
                    impl_item,
                    "#[remote_methods] attribute should only be present once",
                ),
            );
        }
    };

    if impl_item.attrs.len() != 1 {
        combine_errors(
            errors,
            syn::Error::new_spanned(
                impl_item,
                "When #[remote_methods] attribute is used, other attributes are not allowed",
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

/// remote_methods attributes should not have any arguments so this returns an error if there are
/// any
fn check_remote_methods_attr_args(attr: &Attribute, errors: &mut Option<syn::Error>) {
    match attr.meta {
        syn::Meta::Path(_) => (),
        syn::Meta::List(ref list) => combine_errors(
            errors,
            syn::Error::new_spanned(list, "#[remote_methods] does not take any arguments"),
        ),
        syn::Meta::NameValue(ref name_value) => combine_errors(
            errors,
            syn::Error::new_spanned(name_value, "#[remote_methods] does not take any arguments"),
        ),
    }
}

fn push_remote_methods_block_methods(
    dst: &mut Vec<RemoteMethod>,
    impl_item: &ItemImpl,
    errors: &mut Option<syn::Error>,
) {
    for item in impl_item.items.iter() {
        if let ImplItem::Fn(function) = item {
            match push_remote_methods_block_method(function) {
                Err(err) => combine_errors(errors, err),
                Ok(remote_method) => dst.push(remote_method),
            }
        }
    }
}

fn push_remote_methods_block_method(function: &ImplItemFn) -> syn::Result<RemoteMethod> {
    if !function.attrs.is_empty() {
        return Err(syn::Error::new_spanned(
            function,
            "Remote methods are not allowed to use attributes",
        ));
    }

    let mut remote_method = RemoteMethod {
        ident: function.sig.ident.clone(),
        receiver: None,
        args: Vec::new(),
        method: function.clone(),
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
                return Err(syn::Error::new_spanned(
                    input,
                    "rpc_genie crate has a bug, please create an issue with the prototype of your \
                     method so that we can look into it",
                ));
            }
            FnArg::Typed(typed) => remote_method.args.push(typed.clone()),
        }
    }

    Ok(remote_method)
}
