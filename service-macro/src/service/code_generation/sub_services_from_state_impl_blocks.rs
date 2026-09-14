use proc_macro2::TokenStream;
use quote::quote;
use syn::LitStr;

use crate::{Service, service::SubService};

impl Service {
    pub fn sub_services_from_state_impl_blocks(&self) -> TokenStream {
        let sub_service_struct_creation_ast = self.sub_service_struct_creation_ast();

        let impl_sub_services_from_state_for_server = self.impl_block_creator(
            quote!(Server),
            quote!(ServerSubServices),
            &sub_service_struct_creation_ast,
        );
        let impl_sub_services_from_state_for_client = self.impl_block_creator(
            quote!(Client),
            quote!(ClientSubServices),
            &sub_service_struct_creation_ast,
        );

        quote! {
            #impl_sub_services_from_state_for_server
            #impl_sub_services_from_state_for_client
        }
    }

    fn impl_block_creator(
        &self,
        state_struct_name: TokenStream,
        associated_sub_services_struct_name: TokenStream,
        sub_service_struct_creation_ast: &TokenStream,
    ) -> TokenStream {
        quote! {
            impl rpc_genie::SubServicesFromState<
                #state_struct_name,
            > for #associated_sub_services_struct_name
            {
                fn from_state(
                    state: std::sync::Arc<#state_struct_name>,
                    service_path: Option<&str>
                ) -> Self {
                    #sub_service_struct_creation_ast
                }
            }
        }
    }

    fn sub_service_struct_creation_ast(&self) -> TokenStream {
        let field_creation_ast = self.sub_service_struct_field_creation_ast();

        quote! {
            use rpc_genie::IntoRequestHandler;

            Self {
                #field_creation_ast
            }
        }
    }

    fn sub_service_struct_field_creation_ast(&self) -> TokenStream {
        let field_creation_ast = self.sub_services.iter().map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon: _,
                 path: _str_to_add_to_service_path,
             }| {
                let str_to_add_to_service_path = LitStr::new(&format!("{}/", name), name.span());

                quote! {
                    #name: {
                        let service_path = match service_path {
                            Some(service_path) => {
                                service_path.to_owned() + #str_to_add_to_service_path
                            },
                            None => #str_to_add_to_service_path.to_owned(),
                        };

                        std::sync::Arc::clone(&state.#name)
                            .into_request_handler(Some(service_path))
                    }
                }
            },
        );

        quote!(#(#field_creation_ast,)*)
    }
}
