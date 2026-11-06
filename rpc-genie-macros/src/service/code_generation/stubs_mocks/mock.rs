use proc_macro2::TokenStream;
use quote::quote;

use crate::service::SubService;

pub fn mock(
    stub_name: &TokenStream,
    mock_name: &TokenStream,
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
             }| quote!(#name #colon #path #mock_name),
        );

    quote! {
        pub struct #mock_name {
            expectations: Vec<rpc_genie::mock::Expectation>,
            #(#sub_services_fields_definition,)*
        }

        impl #mock_name {
            pub fn new(expectation_setter_callback: FnOnce(&mut #mock_builder_name)) -> Self {
                let mut expectation_setter = #mock_builder_name::default();
                expectation_setter_callback(&mut expectation_setter);
                expectation_setter.build()
            }

            pub fn from_builder(builder: #mock_builder_name) -> Self {

            }
        }
    }
}

fn from_builder_method(
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
             }| quote!(#name #colon #path::#mock_name::from_builder(builder.#name)),
        );

    quote! {
        impl #mock_name {
            fn from_builder(builder: #mock_builder_name) -> Self {
                Self {
                    expectations: builder.expectations,
                    #(#sub_services_fields_creation,)*
                }
            }
        }
    }
}
