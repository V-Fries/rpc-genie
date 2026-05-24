mod module_parser;

use proc_macro::TokenStream;

use crate::module_parser::Service;

#[proc_macro_attribute]
pub fn service(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let _service = syn::parse_macro_input!(tokens as Service);

    todo!()
}
