use proc_macro::TokenStream;
use quote::quote;

use crate::service::Service;

mod service;

#[proc_macro_attribute]
pub fn service(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let service = syn::parse_macro_input!(tokens as Service);

    quote!(#service).into()
}
