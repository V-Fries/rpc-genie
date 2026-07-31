mod impl_handle_request_for_request_handler;
use impl_handle_request_for_request_handler::impl_handle_request_for_request_handler;

mod impl_handle_request_for_sub_service;
use impl_handle_request_for_sub_service::impl_handle_request_for_sub_service;

use proc_macro2::TokenStream;
use quote::quote;

use crate::service::{RemoteMethod, SubService};

use super::Service;

impl Service {
    /// Generate requests handlers structs and implementations
    pub fn request_handlers(&self) -> TokenStream {
        let server_request_handler = request_handler(
            quote!(ServerRequestHandler),
            quote!(Server),
            quote!(ServerSubServices),
            &self.server_remote_methods,
            quote!(ClientStub),
            &self.sub_services,
        );
        let client_request_handler = request_handler(
            quote!(ClientRequestHandler),
            quote!(Client),
            quote!(ClientSubServices),
            &self.client_remote_methods,
            quote!(ServerStub),
            &self.sub_services,
        );

        quote! {
            #server_request_handler
            #client_request_handler
        }
    }
}

/// Generate a request handler struct and implementation
fn request_handler(
    request_handler_struct_name: TokenStream,
    associated_state_struct_name: TokenStream,
    associated_sub_service_struct_name: TokenStream,
    associated_remote_methods: &[RemoteMethod],
    opposite_stub_struct_name: TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let request_handler_type_definition = request_handler_type_definition(
        &request_handler_struct_name,
        &associated_state_struct_name,
        &associated_sub_service_struct_name,
    );

    let impl_handle_request_for_request_handler = impl_handle_request_for_request_handler(
        &request_handler_struct_name,
        &associated_state_struct_name,
        associated_remote_methods,
        &opposite_stub_struct_name,
    );

    let impl_handle_request_for_sub_service = impl_handle_request_for_sub_service(
        &associated_sub_service_struct_name,
        &opposite_stub_struct_name,
        sub_services,
    );

    quote! {
        #request_handler_type_definition
        #impl_handle_request_for_request_handler
        #impl_handle_request_for_sub_service
    }
}

fn request_handler_type_definition(
    request_handler_struct_name: &TokenStream,
    associated_state_struct_name: &TokenStream,
    associated_sub_service_struct_name: &TokenStream,
) -> TokenStream {
    quote! {
        #[doc(hidden)]
        pub type #request_handler_struct_name =
            rpc_genie::RequestHandler<
                #associated_state_struct_name,
                #associated_sub_service_struct_name,
            >;
    }
}
