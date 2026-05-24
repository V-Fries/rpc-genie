#[cfg(test)]
mod tests;

mod service_builder;
use service_builder::ServiceBuilder;

mod error_helpers;
use error_helpers::*;

mod parse_impl_item;
pub use parse_impl_item::RemoteMethod;
use parse_impl_item::parse_impl_item;

mod parse_struct_item;
use parse_struct_item::parse_struct_item;

mod parse_macro_item;
pub use parse_macro_item::SubService;
use parse_macro_item::parse_macro_item;

use syn::{
    Item, ItemMod, ItemStruct,
    parse::{Parse, ParseStream},
};

// TODO remove allow(dead_code)
#[allow(dead_code)]
pub struct Service {
    pub module: ItemMod,

    pub server: ItemStruct,
    pub server_remote_methods: Vec<RemoteMethod>,
    pub client: ItemStruct,
    pub client_remote_methods: Vec<RemoteMethod>,
    pub sub_services: Vec<SubService>,
    pub rest: Vec<Item>,
}

impl Parse for Service {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let mut service_builder = ServiceBuilder::default();

        let module = ItemMod::parse(input)?;
        let module_items = get_module_items(&module)?;

        let mut errors = None;

        for item in module_items {
            match item {
                Item::Struct(struct_item) => {
                    parse_struct_item(&mut service_builder, struct_item.clone(), &mut errors)
                }
                Item::Impl(impl_item) => {
                    parse_impl_item(&mut service_builder, impl_item.clone(), &mut errors)
                }
                Item::Macro(macro_item) => {
                    parse_macro_item(&mut service_builder, macro_item.clone(), &mut errors)
                }
                other => service_builder.rest.push(other.clone()),
            }
        }

        service_builder.build(module, errors)
    }
}

fn get_module_items(module: &ItemMod) -> syn::Result<&[Item]> {
    match module.content.as_ref() {
        Some((_, items)) => Ok(items),
        None => {
            let module_name = &module.ident;

            Err(syn::Error::new_spanned(
                module,
                format!(
                    "module must have a body (e.g., `mod {module_name} {{ ... }}`) instead of just \
                 `mod {module_name};`",
                ),
            ))
        }
    }
}
