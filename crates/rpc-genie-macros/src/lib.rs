use proc_macro::TokenStream;
use quote::ToTokens;

use crate::service::Service;

mod service;

#[proc_macro_attribute]
pub fn service(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    syn::parse_macro_input!(tokens as Service)
        .into_token_stream()
        .into()
}
