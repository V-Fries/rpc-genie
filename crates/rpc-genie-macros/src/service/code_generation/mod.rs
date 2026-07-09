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
        let as_request_handler_impl_blocks = as_request_handler_impl_blocks();
        let sub_services_from_state_impl_blocks = self.sub_services_from_state_impl_blocks();
        let rest = &self.rest;

        quote! {
            #server_and_clients_structs
            #sub_services_structs
            #request_handlers
            #stubs
            #as_request_handler_impl_blocks
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
                                   }: &ItemStruct| {
            let fields_iter = fields.iter();

            let sub_services_fields = self.sub_services.iter().map(
                |SubService {
                     pub_keyword,
                     name,
                     colon,
                     path,
                 }| quote!(#pub_keyword #name #colon #path::#ident),
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
            }
        };

        let server_state = create_final_struct(&self.server);
        let client_state = create_final_struct(&self.client);

        quote! {
            #server_state
            #client_state
        }
    }

    /// Creates the ServerSubServices and ClientSubServices structs
    fn sub_services_structs(&self) -> TokenStream {
        let fields_ast_creator = |field_type| {
            if self.sub_services.is_empty() {
                // To simplify the code we use empty structs when no sub services are present.
                // We use the PhantomData to avoid an error due to 'state not being used
                return quote!(_state_lifetime: std::marker::PhantomData<&'state ()>,);
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
                        #pub_keyword #name #colon #path::#field_type,
                    }
                },
            )
        };

        let server_side_fields = fields_ast_creator(quote!(ServerRequestHandler<'state>));
        let client_side_fields = fields_ast_creator(quote!(ClientRequestHandler<'state>));

        quote! {
            #[doc(hidden)]
            pub struct ServerSubServices<'state> {
                #server_side_fields
            }

            #[doc(hidden)]
            pub struct ClientSubServices<'state> {
                #client_side_fields
            }
        }
    }
}

fn as_request_handler_impl_blocks() -> TokenStream {
    fn impl_block_creator(
        state_struct_name: TokenStream,
        associated_sub_services_struct_name: TokenStream,
    ) -> TokenStream {
        quote! {
            impl<'state> rpc_genie::AsRequestHandler<
                'state,
                #associated_sub_services_struct_name<'state>
            > for #state_struct_name { }
        }
    }

    let impl_as_request_handler_for_server =
        impl_block_creator(quote!(Server), quote!(ServerSubServices));
    let impl_as_request_handler_for_client =
        impl_block_creator(quote!(Client), quote!(ClientSubServices));

    quote! {
        #impl_as_request_handler_for_server
        #impl_as_request_handler_for_client
    }
}
