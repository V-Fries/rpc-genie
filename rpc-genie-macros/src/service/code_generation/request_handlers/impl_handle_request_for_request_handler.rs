use std::ops::Deref;

use proc_macro2::TokenStream;
use quote::quote;
use syn::Type;

use crate::service::{RemoteMethod, code_generation::utils::ident_to_lit_str};

pub fn impl_handle_request_for_request_handler(
    request_handler_struct_name: &TokenStream,
    associated_state_struct_name: &TokenStream,
    associated_remote_methods: &[RemoteMethod],
    opposite_stub_weak_handle_struct_name: &TokenStream,
    opposite_stub_alias: &str,
) -> TokenStream {
    let fn_content = fn_content(
        associated_state_struct_name,
        associated_remote_methods,
        opposite_stub_alias,
    );

    quote! {
        impl<Stream>
            rpc_genie::HandleRequest<#opposite_stub_weak_handle_struct_name<Stream>>
            for #request_handler_struct_name<Stream>
        where
            Stream: Send + 'static + tokio::io::AsyncWrite,
        {
            async fn handle_request(
                &self,
                method_path: &str,
                __rpc_opposite_stub_weak_handle__:
                    &std::sync::Arc<#opposite_stub_weak_handle_struct_name<Stream>>,
                mut __rpc_request_arg_reader__: rpc_genie::frame::rpc_request::RpcRequestArgReader,
            ) -> rpc_genie::frame::rpc_response::RpcResponseBuilder<
                rpc_genie::frame::rpc_response::builder::Uninit,
                rpc_genie::frame::rpc_response::builder::ResponseInit,
            > {
                #fn_content
            }
        }
    }
}

fn fn_content(
    associated_state_struct_name: &TokenStream,
    remote_methods: &[RemoteMethod],
    opposite_stub_alias: &str,
) -> TokenStream {
    let sub_services_handle_request_call = quote! {
        self.sub_services
            .handle_request(
                method_path,
                __rpc_opposite_stub_weak_handle__,
                __rpc_request_arg_reader__,
            )
            .await
    };

    if remote_methods.is_empty() {
        return sub_services_handle_request_call;
    }

    let methods_matches = remote_methods.iter().map(|remote_method| {
        match_branch(
            associated_state_struct_name,
            remote_method,
            opposite_stub_alias,
        )
    });

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
    opposite_stub_alias: &str,
) -> TokenStream {
    let method_name_as_literal_str = ident_to_lit_str(&remote_method.ident);

    let code_that_parses_args_into_vars = remote_method.args.iter().map(|arg| {
        match arg.ty.deref() {
            Type::Path(type_path) if type_path.path.is_ident(opposite_stub_alias) => {
                // No need to deserialize anything as the stub in not included in the request. It
                // is the handle_request function __rpc_opposite_stub_weak_handle__ argument.
                quote!()
            }
            _ =>  {
                let pat = &arg.pat;
                let ty = &arg.ty;
                quote! {
                    let #pat = match __rpc_request_arg_reader__.read_arg::<#ty>() {
                        Ok(arg) => arg,
                        Err(err) => {
                            return rpc_genie::frame::rpc_response::RpcResponse::builder().error(
                                rpc_genie::frame::rpc_response::RpcResponseError::FailedToDeserializeArg {
                                    serialize_error: err.to_string(),
                                },
                            );
                        },
                    };
                }
            }
        }
    });

    let args_names = remote_method.args.iter().map(|arg| match arg.ty.deref() {
        Type::Path(type_path) if type_path.path.is_ident(opposite_stub_alias) => {
            quote! { __rpc_opposite_stub_weak_handle__ }
        }
        _ => {
            let pat = &arg.pat;
            quote!(#pat)
        }
    });

    let remote_method_name = &remote_method.ident;

    let mut method_caller = match remote_method.receiver {
        None => quote! {
            #associated_state_struct_name::<Stream>::#remote_method_name(#(#args_names,)*)
        },
        Some(_) => quote!(self.state.#remote_method_name(#(#args_names,)*)),
    };

    if remote_method.is_async {
        method_caller = quote!(#method_caller.await)
    }

    quote! {
        #method_name_as_literal_str => {
            #(#code_that_parses_args_into_vars)*
            rpc_genie::frame::rpc_response::RpcResponse::builder().response(&#method_caller)
        }
    }
}
