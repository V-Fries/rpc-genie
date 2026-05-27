use syn::{
    Item, MacroDelimiter, Token,
    parse::{Parse, ParseStream},
    punctuated::Punctuated,
};

use crate::SubService;

use super::{ServiceBuilder, combine_errors};

pub fn parse_macro_item(
    service: &mut ServiceBuilder,
    macro_item: syn::ItemMacro,
    errors: &mut Option<syn::Error>,
) {
    if !macro_item.mac.path.is_ident("sub_services") {
        service.rest.push(Item::Macro(macro_item));
        return;
    }

    if let Some(attr) = macro_item.attrs.first() {
        combine_errors(
            errors,
            syn::Error::new_spanned(attr, "sub_services macro does not take any attributes"),
        );
    }

    match macro_item.mac.delimiter {
        MacroDelimiter::Brace(_) => {}
        _ => {
            combine_errors(
                errors,
                syn::Error::new_spanned(
                    macro_item.mac.path,
                    "sub_services macro expects curly brackets",
                ),
            );
        }
    }

    let sub_services: SubServices = match syn::parse2(macro_item.mac.tokens) {
        Err(err) => return combine_errors(errors, err),
        Ok(sub_services) => sub_services,
    };
    let iter = sub_services.entries.into_iter();
    service.sub_services.extend(iter);
}

struct SubServices {
    entries: Punctuated<SubService, Token![,]>,
}

impl Parse for SubServices {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        Ok(Self {
            entries: input.parse_terminated(SubService::parse, Token![,])?,
        })
    }
}

impl Parse for SubService {
    fn parse(input: ParseStream) -> syn::Result<Self> {
        let pub_keyword = if input.peek(Token![pub]) {
            Some(input.parse::<Token![pub]>()?)
        } else {
            None
        };
        let name = input.parse()?;
        let colon = input.parse::<Token![:]>()?;
        let path = input.parse()?;

        Ok(Self {
            pub_keyword,
            name,
            colon,
            path,
        })
    }
}
