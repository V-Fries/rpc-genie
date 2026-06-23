use proc_macro2::TokenStream;
use quote::quote;

use crate::service::{RemoteMethod, SubService};

use super::{Service, utils::ident_to_lit_str};

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
        pub type #request_handler_struct_name<'state> =
            rpc_genie::RequestHandler<
                'state,
                #associated_state_struct_name,
                #associated_sub_service_struct_name<'state>,
            >;
    }
}

fn impl_handle_request_for_request_handler(
    request_handler_struct_name: &TokenStream,
    associated_state_struct_name: &TokenStream,
    associated_remote_methods: &[RemoteMethod],
    opposite_stub_struct_name: &TokenStream,
) -> TokenStream {
    let fn_content = impl_block_fn_content(associated_state_struct_name, associated_remote_methods);

    quote! {
        impl<'state> rpc_genie::HandleRequest<#opposite_stub_struct_name>
            for #request_handler_struct_name<'state>
        {
            async fn handle_request(
                &self,
                method_path: &str,
                __rpc_client__: #opposite_stub_struct_name,
                mut __rpc_args__: rpc_genie::Args,
            ) -> rpc_genie::Result<rpc_genie::ReturnValue> {
                #fn_content
            }
        }
    }
}

fn impl_block_fn_content(
    associated_state_struct_name: &TokenStream,
    remote_methods: &[RemoteMethod],
) -> TokenStream {
    let sub_services_handle_request_call = quote! {
        self.sub_services
            .handle_request(method_path, __rpc_client__, __rpc_args__)
            .await
    };

    if remote_methods.is_empty() {
        return sub_services_handle_request_call;
    }

    let methods_matches = remote_methods
        .iter()
        .map(|remote_method| match_branch(associated_state_struct_name, remote_method));

    quote! {
        match method_path {
            #(#methods_matches)*
            _ => {
                #sub_services_handle_request_call
            }
        }
    }
}

fn match_branch(
    associated_state_struct_name: &TokenStream,
    remote_method: &RemoteMethod,
) -> TokenStream {
    let method_name_as_literal_str = ident_to_lit_str(&remote_method.ident);

    let code_that_parses_args_into_vars = remote_method.args.iter().map(|arg| {
        let pat = &arg.pat;
        let ty = &arg.ty;
        quote!(let #pat = __rpc_args__.read_arg::<#ty>()?;)
    });

    let args_names = remote_method.args.iter().map(|arg| {
        let pat = &arg.pat;
        quote!(#pat)
    });

    let remote_method_name = &remote_method.ident;

    let method_caller = match remote_method.receiver {
        None => quote!(#associated_state_struct_name::#remote_method_name(#(#args_names,)*)),
        Some(_) => quote!(self.state.#remote_method_name(#(#args_names,)*)),
    };

    quote! {
        #method_name_as_literal_str => {
            #(#code_that_parses_args_into_vars)*
            let res = #method_caller;
            Ok(rpc_genie::ReturnValue::new(res))
        }
    }
}

fn impl_handle_request_for_sub_service(
    associated_sub_service_struct_name: &TokenStream,
    opposite_stub_struct_name: &TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let fn_content = sub_service_handle_request_fn_content(sub_services);

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

fn sub_service_handle_request_fn_content(sub_services: &[SubService]) -> TokenStream {
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
