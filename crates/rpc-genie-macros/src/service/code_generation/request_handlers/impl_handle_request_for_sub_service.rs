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
        impl<'state> rpc_genie::HandleRequest<#opposite_stub_struct_name>
            for #associated_sub_service_struct_name<'state>
        {
            async fn handle_request(
                &self,
                method_path: &str,
                __rpc_client__: #opposite_stub_struct_name,
                __rpc_args__: rpc_genie::Args,
            ) -> rpc_genie::Result<rpc_genie::ReturnValue> {
                #fn_content
            }
        }
    }
}

fn fn_content(sub_services: &[SubService]) -> TokenStream {
    let not_found_error_ast = quote!(Err(rpc_genie::Error::MethodNotFound));

    if sub_services.is_empty() {
        return not_found_error_ast;
    }
    let match_branches = sub_services.iter().map(|remote_method| {
        let name = &remote_method.name;
        let ident_as_lit_str = ident_to_lit_str(name);
        quote! {
            #ident_as_lit_str => {
                self.#name
                    .handle_request(method_path, __rpc_client__.#name, __rpc_args__)
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
