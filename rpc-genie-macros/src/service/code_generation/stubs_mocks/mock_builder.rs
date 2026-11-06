use proc_macro2::TokenStream;
use quote::quote;

use crate::service::{RemoteMethod, SubService};

pub fn mock_builder(
    methods: &[RemoteMethod],
    mock_name: &TokenStream,
    mock_builder_name: &TokenStream,
    opposite_stub_alias: &str,
    sub_services: &[SubService],
) -> TokenStream {
    let struct_def = mock_builder_struct(mock_builder_name, sub_services);
    let constructor = mock_builder_constructor(mock_builder_name, sub_services);
    let build_method = mock_builder_build_method(mock_name, mock_builder_name, sub_services);

    quote! {
        #struct_def
        #constructor
        #build_method
    }
}

fn mock_builder_struct(
    mock_builder_name: &TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let sub_services_fields_definition = sub_services
        .iter()
        .filter(|sub_service| sub_service.pub_keyword.is_some())
        .map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon,
                 path,
             }| quote!(#name #colon #path::#mock_builder_name),
        );

    quote! {
        #[derive(Default)]
        struct #mock_builder_name {
            expectations: Vec<rpc_genie::mock::Expectation>,
            #(#sub_services_fields_definition,)*
        }
    }
}

fn mock_builder_constructor(
    mock_builder_name: &TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let sub_services_fields_creation = sub_services
        .iter()
        .filter(|sub_service| sub_service.pub_keyword.is_some())
        .map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon,
                 path,
             }| quote!(#name #colon #path::#mock_builder_name::new()),
        );

    quote! {
        fn new() -> Self {
            Self {
                expectations: Vec::new(),
                #(#sub_services_fields_creation,)*
            }
        }
    }
}

fn mock_builder_build_method(
    mock_name: &TokenStream,
    mock_builder_name: &TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let sub_services_fields_creation = sub_services
        .iter()
        .filter(|sub_service| sub_service.pub_keyword.is_some())
        .map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon,
                 path,
             }| quote!(#name #colon #path::#mock_name::from_builder(self.#name)),
        );

    quote! {
        impl #mock_builder_name {
            fn build(self) -> #mock_name {
                #mock_name {
                    expectations: self.expectations,
                    #(#sub_services_fields_creation,)*
                }
            }
        }
    }
}
