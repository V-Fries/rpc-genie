use proc_macro2::TokenStream;
use quote::quote;

use crate::service::{SubService, code_generation::utils::ident_to_lit_str};

pub fn impl_handle_request_for_sub_service(
    associated_sub_service_struct_name: &TokenStream,
    opposite_stub_struct_name: &TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let fn_content = fn_content(sub_services);

    quote! {
        impl rpc_genie::HandleRequest<#opposite_stub_struct_name>
            for #associated_sub_service_struct_name
        {
            async fn handle_request(
                &self,
                method_path: &str,
                __rpc_stub__: #opposite_stub_struct_name,
                __rpc_request_arg_reader__: rpc_genie::frame::rpc_request::RpcRequestArgReader,
                __rpc_request_id__: rpc_genie::frame::RpcRequestId,
            ) -> rpc_genie::frame::rpc_response::RpcResponse {
                #fn_content
            }
        }
    }
}

fn fn_content(sub_services: &[SubService]) -> TokenStream {
    let not_found_error_ast = quote! {
        rpc_genie::frame::rpc_response::RpcResponse::builder()
            .id(__rpc_request_id__)
            .error(rpc_genie::Error::MethodNotFound)
            .build()
    };

    if sub_services.is_empty() {
        return not_found_error_ast;
    }
    let match_branches = sub_services.iter().map(|remote_method| {
        let name = &remote_method.name;
        let name_as_lit_str = ident_to_lit_str(name);
        quote! {
            #name_as_lit_str => {
                self.#name
                    .handle_request(
                        method_path,
                        __rpc_stub__.#name,
                        __rpc_request_arg_reader__,
                        __rpc_request_id__,
                    )
                    .await
            }
        }
    });

    quote! {
        let Some((sub_service_name, method_path)) = method_path.split_once("/") else {
            return #not_found_error_ast;
        };

        match sub_service_name {
            #(#match_branches)*
            _ => #not_found_error_ast
        }
    }
}
