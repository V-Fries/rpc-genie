use syn::{Item, ItemStruct};

use super::ServiceBuilder;

pub fn parse_struct_item(
    service: &mut ServiceBuilder,
    struct_item: ItemStruct,
    _errors: &mut Option<syn::Error>,
) {
    if struct_item.ident == "Server" {
        service.server = Some(struct_item);
    } else if struct_item.ident == "Client" {
        service.client = Some(struct_item);
    } else {
        service.rest.push(Item::Struct(struct_item));
    }
}
