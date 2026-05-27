use proc_macro::TokenStream;

use crate::service::Service;

mod service;

#[proc_macro_attribute]
pub fn service(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let _service = syn::parse_macro_input!(tokens as Service);

    todo!()
}
