use proc_macro2::TokenStream;
use quote::quote;
use syn::{Fields, Ident, ItemStruct, Type, parse_quote};

use crate::{Service, service::SubService};

impl Service {
    /// Creates the final code for the Server and Client
    pub fn server_and_clients_structs(&self) -> TokenStream {
        let server_state = self.create_final_struct(
            &self.server,
            quote!(rpc_genie::Server),
            quote!(ClientStubArcHandle),
            quote!(ClientStub),
        );
        let client_state = self.create_final_struct(
            &self.client,
            quote!(rpc_genie::Client),
            quote!(ServerStubArcHandle),
            quote!(ServerStub),
        );

        quote! {
            #server_state
            #client_state
        }
    }

    fn create_final_struct(
        &self,
        item_struct: &ItemStruct,
        trait_to_implement: TokenStream,
        opposite_stub_arc_handle_type_name: TokenStream,
        opposite_stub_generic_handle_type_name: TokenStream,
    ) -> TokenStream {
        let struct_definition =
            self.struct_definition(item_struct, opposite_stub_generic_handle_type_name);

        let struct_ident = &item_struct.ident;

        quote! {
            #struct_definition

            impl<RequestSender> rpc_genie::State for #struct_ident<RequestSender>
            where
                RequestSender: rpc_genie::SubscribableStub + rpc_genie::SendRequest,
            {}
            impl<const MAX_FRAME_SIZE: usize, Stream, RequestSender>
                #trait_to_implement<MAX_FRAME_SIZE, Stream, RequestSender>
                for #struct_ident<RequestSender>
            where
                RequestSender: rpc_genie::SubscribableStub + rpc_genie::SendRequest,
            {
                type #opposite_stub_arc_handle_type_name =
                    #opposite_stub_arc_handle_type_name<MAX_FRAME_SIZE, Stream>;
            }
        }
    }

    fn struct_definition(
        &self,
        ItemStruct {
            attrs,
            vis,
            struct_token,
            ident,
            generics: _,
            fields,
            semi_token,
        }: &ItemStruct,
        opposite_stub_generic_handle_type_name: TokenStream,
    ) -> TokenStream {
        let fields =
            self.generate_struct_fields(ident, fields, opposite_stub_generic_handle_type_name);

        quote! {
            #(#attrs)*
            #vis #struct_token #ident<RequestSender>
            where
                RequestSender: rpc_genie::SubscribableStub + rpc_genie::SendRequest,
            {
                #fields
            }#semi_token
        }
    }

    fn generate_struct_fields(
        &self,
        struct_ident: &Ident,
        fields: &Fields,
        opposite_stub_generic_handle_type_name: TokenStream,
    ) -> TokenStream {
        let sub_services_fields = self.sub_services.iter().map(
            |SubService {
                 pub_keyword,
                 name,
                 colon,
                 path,
             }| {
                quote! {
                    // We always add #[allow(unused)] to sub services state fields as the user
                    // may never want to access the state directly from the parent
                    // (The sub-service request handler has a ref to it's state, so the compiler
                    // won't be able to see that the value is used unless it is used directly in
                    // the parent)
                    // #[allow(unused)]
                    #pub_keyword #name #colon std::sync::Arc<#path::#struct_ident<RequestSender>>
                }
            },
        );

        let mut contains_topic_field = false;

        let fields = fields.iter().cloned().map(|mut field| match field.ty {
            Type::Path(type_path) if type_path.path.is_ident("Topic") => {
                contains_topic_field = true;
                field.ty = parse_quote! {
                    rpc_genie::Topic<
                        #opposite_stub_generic_handle_type_name<RequestSender>
                    >
                };
                field
            }
            _ => field,
        });

        let mut fields = quote! {
            #(#fields,)*
            #(#sub_services_fields,)*
        };

        if self.sub_services.is_empty() && !contains_topic_field {
            fields = quote! {
                #fields
                pub _request_sender:
                    std::marker::PhantomData<fn() -> RequestSender>,
            };
        }

        fields
    }
}
