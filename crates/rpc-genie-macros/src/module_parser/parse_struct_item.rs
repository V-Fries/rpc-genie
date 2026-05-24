use syn::{Item, ItemStruct};

use crate::module_parser::error_helpers::combine_errors;

use super::ServiceBuilder;

pub fn parse_struct_item(
    service: &mut ServiceBuilder,
    struct_item: ItemStruct,
    errors: &mut Option<syn::Error>,
) {
    if struct_item.ident == "Server" {
        if service.server.is_some() {
            return combine_errors(
                errors,
                syn::Error::new_spanned(struct_item, "Server struct was already defined"),
            );
        }
        service.server = Some(struct_item);
    } else if struct_item.ident == "Client" {
        if service.client.is_some() {
            return combine_errors(
                errors,
                syn::Error::new_spanned(struct_item, "Client struct was already defined"),
            );
        }
        service.client = Some(struct_item);
    } else {
        service.rest.push(Item::Struct(struct_item));
    }
}
