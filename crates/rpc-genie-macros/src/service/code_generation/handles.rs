use proc_macro2::TokenStream;
use quote::quote;
use syn::ReturnType;

use crate::service::{RemoteMethod, Service, SubService};

impl Service {
    pub fn handles(&self) -> TokenStream {
        let client_handle = self.handle(quote!(ClientHandle), &self.server_remote_methods);
        let server_handle = self.handle(quote!(ServerHandle), &self.client_remote_methods);

        quote! {
            #server_handle
            #client_handle
        }
    }

    fn handle(
        &self,
        handle_struct_name: TokenStream,
        opposite_remote_methods: &[RemoteMethod],
    ) -> TokenStream {
        let struct_fields = self.handle_struct_fields(&handle_struct_name);
        let methods = methods(&handle_struct_name, opposite_remote_methods);

        quote! {
            pub struct #handle_struct_name {
                #struct_fields
            }

            #methods
        }
    }

    fn handle_struct_fields(&self, handle_struct_name: &TokenStream) -> TokenStream {
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
                    #pub_keyword #name #colon #path::#handle_struct_name,
                }
            },
        )
    }
}

fn methods(
    handle_struct_name: &TokenStream,
    opposite_remote_methods: &[RemoteMethod],
) -> TokenStream {
    let methods = opposite_remote_methods.iter().map(method);

    quote! {
        impl #handle_struct_name {
            #(#methods)*
        }
    }
}

fn method(
    RemoteMethod {
        vis,
        ident,
        receiver: _,
        args,
        output,
    }: &RemoteMethod,
) -> TokenStream {
    let output = match output {
        ReturnType::Default => quote!(-> rpc_genie::Result<()>),
        ReturnType::Type(arrow, type_ast) => quote!(#arrow rpc_genie::Result<#type_ast>),
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
