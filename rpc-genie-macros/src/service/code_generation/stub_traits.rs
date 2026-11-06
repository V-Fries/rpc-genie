use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use syn::{Pat, Type};

use crate::{
    Service,
    service::code_generation::utils::single_request_sender_output,
    service::{RemoteMethod, SubService},
};

impl Service {
    pub(super) fn stub_traits(&self) -> TokenStream {
        let server_stub_trait = stub_trait(
            &self.server_remote_methods,
            quote!(ServerStubTrait),
            quote!(ServerStub),
            "ClientStub",
            &self.sub_services,
            quote!(ServerStubTrait),
        );

        let client_stub_trait = stub_trait(
            &self.client_remote_methods,
            quote!(ClientStubTrait),
            quote!(ClientStub),
            "ServerStub",
            &self.sub_services,
            quote!(ClientStubTrait),
        );

        quote! {
            #server_stub_trait
            #client_stub_trait
        }
    }
}

fn stub_trait(
    methods: &[RemoteMethod],
    trait_name: TokenStream,
    stub_name: TokenStream,
    opposite_stub_alias: &str,
    sub_services: &[SubService],
    child_trait_name: TokenStream,
) -> TokenStream {
    let methods = methods
        .iter()
        .filter(|method| matches!(&method.vis, syn::Visibility::Public(_)))
        .collect::<Vec<_>>();

    let trait_definition = stub_trait_definition(
        &methods,
        &trait_name,
        opposite_stub_alias,
        sub_services,
        &child_trait_name,
    );

    let trait_impl = stub_trait_impl(
        &methods,
        &trait_name,
        &stub_name,
        opposite_stub_alias,
        sub_services,
        &child_trait_name,
    );
    quote! {
        #trait_definition
        #trait_impl
    }
}

fn stub_trait_definition(
    methods: &[&RemoteMethod],
    trait_name: &TokenStream,
    opposite_stub_alias: &str,
    sub_services: &[SubService],
    child_trait_name: &TokenStream,
) -> TokenStream {
    let trait_methods = methods.iter().map(|method| {
        let ident = &method.ident;
        let output = single_request_sender_output(&method.output, quote!(Self::RequestSender));
        let args = visible_args(method, opposite_stub_alias);
        let inputs = args.iter().map(|(name, ty)| quote!(#name: #ty));

        quote! {
            fn #ident(&self, #(#inputs),*) #output;
        }
    });

    let sub_service_getters = sub_service_getters(sub_services, child_trait_name);

    quote! {
        /// Trait for the generated stub's callable remote methods. Implement this trait on a mock
        /// stub to use it in unit tests.
        pub trait #trait_name {
            type RequestSender: rpc_genie::send_request::SendRequest;

            #(#trait_methods)*
            #(#sub_service_getters)*
        }

    }
}

fn stub_trait_impl(
    methods: &[&RemoteMethod],
    trait_name: &TokenStream,
    stub_name: &TokenStream,
    opposite_stub_alias: &str,
    sub_services: &[SubService],
    child_trait_name: &TokenStream,
) -> TokenStream {
    let trait_impl_methods = methods.iter().map(|method| {
        let ident = &method.ident;
        let output = single_request_sender_output(&method.output, quote!(RequestSender));
        let args = visible_args(method, opposite_stub_alias);
        let inputs = args.iter().map(|(name, ty)| quote!(#name: #ty));
        let args = args.iter().map(|(name, _)| name);

        quote! {
            fn #ident(&self, #(#inputs),*) #output {
                self.#ident(#(#args),*)
            }
        }
    });

    let sub_service_getter_impls = sub_service_getter_impls(sub_services, child_trait_name);

    quote! {
        impl<RequestSender> #trait_name for #stub_name<RequestSender>
        where
            RequestSender: rpc_genie::send_request::SendRequest,
        {
            type RequestSender = RequestSender;

            #(#trait_impl_methods)*
            #(#sub_service_getter_impls)*
        }
    }
}

fn sub_service_getters(
    sub_services: &[SubService],
    child_trait_name: &TokenStream,
) -> Vec<TokenStream> {
    sub_services
        .iter()
        .filter_map(|sub_service| {
            sub_service.pub_keyword.as_ref().map(|_| {
                let name = &sub_service.name;
                let path = &sub_service.path;
                quote! {
                    fn #name(
                        &self,
                    ) -> &impl #path::#child_trait_name<RequestSender = Self::RequestSender>;
                }
            })
        })
        .collect()
}

fn sub_service_getter_impls(
    sub_services: &[SubService],
    child_trait_name: &TokenStream,
) -> Vec<TokenStream> {
    sub_services
        .iter()
        .filter_map(|sub_service| {
            sub_service.pub_keyword.as_ref().map(|_| {
                let name = &sub_service.name;
                let path = &sub_service.path;
                quote! {
                    fn #name(
                        &self,
                    ) -> &impl #path::#child_trait_name<RequestSender = RequestSender> {
                        self.#name()
                    }
                }
            })
        })
        .collect()
}

pub(super) fn visible_args<'a>(
    method: &'a RemoteMethod,
    opposite_stub_alias: &str,
) -> Vec<(syn::Ident, &'a Type)> {
    method
        .args
        .iter()
        .filter(|arg| {
            !matches!(
                arg.ty.as_ref(),
                Type::Path(type_path) if type_path.path.is_ident(opposite_stub_alias)
            )
        })
        .enumerate()
        .map(|(index, arg)| {
            let name = match arg.pat.as_ref() {
                Pat::Ident(pat_ident) => pat_ident.ident.clone(),
                // We have to rename pattern args as trait don't allow them
                _ => format_ident!("__rpc_arg_{index}__"),
            };
            (name, arg.ty.as_ref())
        })
        .collect()
}
