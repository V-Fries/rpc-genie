use proc_macro2::TokenStream;
use quote::quote;
use syn::ReturnType;

use crate::service::{RemoteMethod, Service, SubService};

impl Service {
    pub fn stubs(&self) -> TokenStream {
        let client_stub = self.stub(quote!(ClientStub), &self.client_remote_methods);
        let server_stub = self.stub(quote!(ServerStub), &self.server_remote_methods);

        quote! {
            #server_stub
            #client_stub
        }
    }

    fn stub(
        &self,
        stub_struct_name: TokenStream,
        associated_remote_methods: &[RemoteMethod],
    ) -> TokenStream {
        let struct_fields = self.stub_struct_fields(&stub_struct_name);
        let remote_methods_callers =
            remote_methods_callers(&stub_struct_name, associated_remote_methods);
        let impl_stub_trait = self.impl_stub_trait(&stub_struct_name);

        quote! {
            #[derive(Clone)]
            pub struct #stub_struct_name {
                #struct_fields
            }

            #impl_stub_trait

            #remote_methods_callers
        }
    }

    fn stub_struct_fields(&self, stub_struct_name: &TokenStream) -> TokenStream {
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
                    #pub_keyword #name #colon #path::#stub_struct_name,
                }
            },
        )
    }

    fn impl_stub_trait(&self, stub_struct_name: &TokenStream) -> TokenStream {
        let field_constructor = self.sub_services.iter().map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon,
                 path,
             }| {
                quote! {
                    #name #colon #path::#stub_struct_name::new()
                }
            },
        );

        quote! {
            impl rpc_genie::Stub for #stub_struct_name {
                fn new() -> Self {
                    Self {
                        #(#field_constructor,)*
                    }
                }
            }
        }
    }
}

fn remote_methods_callers(
    stub_struct_name: &TokenStream,
    associated_remote_methods: &[RemoteMethod],
) -> TokenStream {
    let associated_remote_methods = associated_remote_methods.iter().map(remote_method_caller);

    quote! {
        impl #stub_struct_name {
            #(#associated_remote_methods)*
        }
    }
}

fn remote_method_caller(
    RemoteMethod {
        vis,
        ident,
        receiver: _,
        args,
        output,
    }: &RemoteMethod,
) -> TokenStream {
    let output = match output {
        ReturnType::Default => quote!(-> rpc_genie::SingleRequestSender<()>),
        ReturnType::Type(arrow, type_ast) => {
            quote!(#arrow rpc_genie::SingleRequestSender<#type_ast>)
        }
    };

    // TODO remove this once we finish generating the actual fn content (maybe next PR?)
    let remove_unused_var_warning = args.iter().map(|arg| {
        let pat = &arg.pat;
        quote!(let _ = #pat;)
    });

    quote! {
        #vis fn #ident(&self, #(#args,)*) #output {
            // TODO remove this once we finish generating the actual fn content (maybe next PR?)
            #(#remove_unused_var_warning)*

            todo!("Send the request to server and receive the response")
        }
    }
}
