//! Macros used by the [`rpc-genie`](https://docs.rs/rpc-genie/latest/rpc-genie/) crate
//!
//! It is unlikely you would ever want to use this crate directly. See
//! [`rpc-genie`](https://docs.rs/rpc-genie/latest/rpc-genie/) for a crate that facilitates with
//! RPC requests.

use proc_macro::TokenStream;
use quote::quote;

use crate::service::Service;

mod service;

/// Documentation can be found [here](https://docs.rs/rpc-genie/latest/rpc-genie/attr.service.html)
#[proc_macro_attribute]
pub fn service(_attr: TokenStream, tokens: TokenStream) -> TokenStream {
    let service = syn::parse_macro_input!(tokens as Service);

    quote!(#service).into()
}
