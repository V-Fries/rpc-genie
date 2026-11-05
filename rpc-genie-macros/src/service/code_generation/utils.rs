use quote::quote;
use syn::{Ident, LitStr, ReturnType};

pub fn ident_to_lit_str(ident: &Ident) -> LitStr {
    syn::LitStr::new(&ident.to_string(), ident.span())
}

pub fn single_request_sender_output(output: &ReturnType) -> proc_macro2::TokenStream {
    match output {
        ReturnType::Default => quote!(-> rpc_genie::SingleRequestSender<RequestSender, ()>),
        ReturnType::Type(arrow, output_type) => {
            quote!(#arrow rpc_genie::SingleRequestSender<RequestSender, #output_type>)
        }
    }
}
