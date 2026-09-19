mod request_handlers;
mod server_and_client_structs;
mod stubs;
mod sub_services_from_state_impl_blocks;
mod utils;

use super::{Service, SubService};

use proc_macro2::TokenStream;
use quote::{ToTokens, TokenStreamExt, quote};
use syn::ItemMod;

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
        let client_handle_alias = client_handle_alias();
        let server_handle_alias = server_handle_alias();
        let rest = &self.rest;

        quote! {
            #server_and_clients_structs
            #sub_services_structs
            #request_handlers
            #stubs
            #into_request_handler_impl_blocks
            #sub_services_from_state_impl_blocks
            #client_handle_alias
            #server_handle_alias
            #(#rest)*
        }
    }

    /// Creates the ServerSubServices and ClientSubServices structs
    fn sub_services_structs(&self) -> TokenStream {
        let fields_ast_creator = |field_type| {
            if self.sub_services.is_empty() {
                return quote!(pub _request_sender: std::marker::PhantomData<RequestSender>,);
            }

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
                        #pub_keyword #name #colon std::sync::Arc<#path::#field_type<RequestSender>>,
                    }
                },
            )
        };

        let server_side_fields = fields_ast_creator(quote!(ServerRequestHandler));
        let client_side_fields = fields_ast_creator(quote!(ClientRequestHandler));

        quote! {
            #[doc(hidden)]
            pub struct ServerSubServices<RequestSender>
            where
                RequestSender: rpc_genie::SubscribableStub + rpc_genie::SendRequest,
            {
                #server_side_fields
            }

            #[doc(hidden)]
            pub struct ClientSubServices<RequestSender>
            where
                RequestSender: rpc_genie::SubscribableStub + rpc_genie::SendRequest,
            {
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
            impl<RequestSender> rpc_genie::IntoRequestHandler<
                #request_handler_struct_name<RequestSender>,
                #associated_sub_services_struct_name<RequestSender>,
            > for #state_struct_name<RequestSender>
                where
                    RequestSender: rpc_genie::SubscribableStub + rpc_genie::SendRequest,
            {
                fn into_request_handler(
                    self: std::sync::Arc<Self>,
                    service_path: Option<String>
                ) -> std::sync::Arc<#request_handler_struct_name<RequestSender>> {
                    use rpc_genie::SubServicesFromState;

                    std::sync::Arc::new(#request_handler_struct_name::<RequestSender> {
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

fn client_handle_alias() -> TokenStream {
    quote! {
        /// Type alias of an `rpc_genie::client::ClientHandle` for this service
        pub type ClientHandle<const MAX_FRAME_SIZE: usize, Stream>
            = rpc_genie::client::ClientHandle<
                ServerStubArcHandle<MAX_FRAME_SIZE, Stream>,
                Client<std::sync::Weak<rpc_genie::stream_handler::Handle<MAX_FRAME_SIZE, Stream>>>,
            >;
    }
}

fn server_handle_alias() -> TokenStream {
    quote! {
        /// Type alias of an `rpc_genie::server::ServerHandle` for this service
        pub type ServerHandle<const MAX_FRAME_SIZE: usize, Stream>
            = rpc_genie::server::ServerHandle<ClientStubArcHandle<MAX_FRAME_SIZE, Stream>>;
    }
}
