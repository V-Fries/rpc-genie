use syn::{GenericParam, Ident, Item, ItemStruct};

use crate::service::service_parser::error_helpers::combine_errors;

use super::ServiceBuilder;

pub fn parse_struct_item(
    service: &mut ServiceBuilder,
    struct_item: ItemStruct,
    errors: &mut Option<syn::Error>,
) {
    if struct_item.ident == "Server" {
        check_generics(&struct_item, errors);
        if service.server.is_some() {
            combine_errors(
                errors,
                syn::Error::new_spanned(struct_item, "Server struct was already defined"),
            );
            return;
        }
        service.server = Some(struct_item);
    } else if struct_item.ident == "Client" {
        check_generics(&struct_item, errors);
        if service.client.is_some() {
            combine_errors(
                errors,
                syn::Error::new_spanned(struct_item, "Client struct was already defined"),
            );
            return;
        }
        service.client = Some(struct_item);
    } else {
        service.rest.push(Item::Struct(struct_item));
    }
}

fn check_generics(struct_item: &ItemStruct, errors: &mut Option<syn::Error>) {
    let mut has_request_sender_type = false;

    for generic_param in struct_item.generics.params.iter() {
        check_generic_param(
            &struct_item.ident,
            generic_param,
            &mut has_request_sender_type,
            errors,
        );
    }

    if !has_request_sender_type {
        combine_errors(
            errors,
            syn::Error::new_spanned(
                &struct_item.ident,
                format!(
                    "{} struct requires a generic param named RequestSender.",
                    struct_item.ident,
                ),
            ),
        );
    }

    if let Some(ref where_clause) = struct_item.generics.where_clause {
        combine_errors(
            errors,
            syn::Error::new_spanned(
                where_clause,
                format!(
                    "{} struct is not allowed to have a where clause.",
                    struct_item.ident
                ),
            ),
        );
    }
}

fn check_generic_param(
    struct_name: &Ident,
    generic_param: &GenericParam,
    has_request_sender_type: &mut bool,
    errors: &mut Option<syn::Error>,
) {
    match generic_param {
        GenericParam::Lifetime(lifetime) => {
            combine_errors(
                errors,
                syn::Error::new_spanned(
                    lifetime,
                    format!("{struct_name} struct is not allowed to have generic lifetimes"),
                ),
            );
        }
        GenericParam::Type(type_param) => {
            if type_param.ident == "RequestSender" {
                // No need to make an error if the bool is already at true. The compiler will make
                // one on it's own
                *has_request_sender_type = true;
            } else {
                combine_errors(
                    errors,
                    syn::Error::new_spanned(
                        type_param,
                        format!(
                            "{struct_name} struct is not allowed to have generic type parameters \
                             other than RequestSender."
                        ),
                    ),
                );
            }
        }
        GenericParam::Const(const_param) => {
            combine_errors(
                errors,
                syn::Error::new_spanned(
                    const_param,
                    format!("{struct_name} struct is not allowed to have generic cont parameters."),
                ),
            );
        }
    }
}
