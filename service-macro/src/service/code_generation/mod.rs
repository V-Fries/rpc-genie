mod request_handlers;
mod stubs;
mod sub_services_from_state_impl_blocks;
mod utils;

use super::{Service, SubService};

use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::{ItemMod, ItemStruct};

impl ToTokens for Service {
    fn to_tokens(&self, dst: &mut TokenStream) {
        let ItemMod {
            attrs,
            vis,
            unsafety,
            mod_token,
            ident,
            content: _,
            semi,
        } = &self.module;

        let content = self.content();

        let tokens = quote! {
            #(#attrs)*
            #vis #unsafety #mod_token #ident {
                #content
            }#semi
        };

        dst.append_all(tokens);
    }
}

impl Service {
    fn content(&self) -> TokenStream {
        let server_and_clients_structs = self.server_and_clients_structs();
        let sub_services_structs = self.sub_services_structs();
        let request_handlers = self.request_handlers();
        let stubs = self.stubs();
        let into_request_handler_impl_blocks = into_request_handler_impl_blocks();
        let sub_services_from_state_impl_blocks = self.sub_services_from_state_impl_blocks();
        let rest = &self.rest;

        quote! {
            #server_and_clients_structs
            #sub_services_structs
            #request_handlers
            #stubs
            #into_request_handler_impl_blocks
            #sub_services_from_state_impl_blocks
            #(#rest)*
        }
    }

    /// Creates the final code for the Server and Client
    fn server_and_clients_structs(&self) -> TokenStream {
        let create_final_struct = |ItemStruct {
                                       attrs,
                                       vis,
                                       struct_token,
                                       ident,
                                       generics,
                                       fields,
                                       semi_token,
                                   }: &ItemStruct,
                                   trait_to_implement,
                                   opposite_stub_type_name| {
            let fields_iter = fields.iter();

            let sub_services_fields = self.sub_services.iter().map(
                |SubService {
                     pub_keyword,
                     name,
                     colon,
                     path,
                 }| quote!(#pub_keyword #name #colon std::sync::Arc<#path::#ident>),
            );

            quote! {
                #(#attrs)*
                #vis #struct_token #ident #generics {
                    #(#fields_iter,)*
                    #(
                        // We always add #[allow(unused)] to sub services state fields as the user
                        // may never want to access the state directly from the parent
                        // (The sub-service request handler has a ref to it's state, so the compiler
                        // won't be able to see that the value is used unless it is used directly in
                        // the parent)
                        #[allow(unused)]
                        #sub_services_fields,
                    )*
                }#semi_token

                impl rpc_genie::State for #ident {}
                impl<const MAX_FRAME_SIZE: usize, Stream>
                    #trait_to_implement<MAX_FRAME_SIZE, Stream>
                    for #ident
                {
                    type #opposite_stub_type_name =
                        #opposite_stub_type_name<MAX_FRAME_SIZE, Stream>;
                }
            }
        };

        let server_state = create_final_struct(
            &self.server,
            quote!(rpc_genie::Server),
            quote!(ClientStubArcHandle),
        );
        let client_state = create_final_struct(
            &self.client,
            quote!(rpc_genie::Client),
            quote!(ServerStubArcHandle),
        );

        quote! {
            #server_state
            #client_state
        }
    }

    /// Creates the ServerSubServices and ClientSubServices structs
    fn sub_services_structs(&self) -> TokenStream {
        let fields_ast_creator = |field_type| {
            self.sub_services.iter().fold(
                TokenStream::new(),
                |acc,
                 SubService {
                     pub_keyword,
                     name,
                     colon,
                     path,
                 }| {
                    quote! {
                        #acc
                        #pub_keyword #name #colon std::sync::Arc<#path::#field_type>,
                    }
                },
            )
        };

        let server_side_fields = fields_ast_creator(quote!(ServerRequestHandler));
        let client_side_fields = fields_ast_creator(quote!(ClientRequestHandler));

        quote! {
            #[doc(hidden)]
            pub struct ServerSubServices {
                #server_side_fields
            }

            #[doc(hidden)]
            pub struct ClientSubServices {
                #client_side_fields
            }
        }
    }
}

fn into_request_handler_impl_blocks() -> TokenStream {
    fn impl_block_creator(
        state_struct_name: TokenStream,
        request_handler_struct_name: TokenStream,
        associated_sub_services_struct_name: TokenStream,
    ) -> TokenStream {
        quote! {
            impl rpc_genie::IntoRequestHandler<
                #request_handler_struct_name,
                #associated_sub_services_struct_name,
            > for #state_struct_name {
                fn into_request_handler(
                    self: std::sync::Arc<Self>,
                    service_path: Option<String>
                ) -> std::sync::Arc<#request_handler_struct_name> {
                    use rpc_genie::SubServicesFromState;

                    std::sync::Arc::new(#request_handler_struct_name {
                        state: std::sync::Arc::clone(&self),
                        sub_services: #associated_sub_services_struct_name::from_state(
                            self,
                            service_path.as_deref(),
                        ),
                        service_path,
                    })
                }
            }
        }
    }

    let impl_as_request_handler_for_server = impl_block_creator(
        quote!(Server),
        quote!(ServerRequestHandler),
        quote!(ServerSubServices),
    );
    let impl_as_request_handler_for_client = impl_block_creator(
        quote!(Client),
        quote!(ClientRequestHandler),
        quote!(ClientSubServices),
    );

    quote! {
        #impl_as_request_handler_for_server
        #impl_as_request_handler_for_client
    }
}
