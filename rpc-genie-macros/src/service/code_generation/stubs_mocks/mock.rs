use proc_macro2::TokenStream;
use quote::quote;

use crate::service::{
    RemoteMethod, SubService,
    code_generation::{
        stub_traits::{send_single_request_output, visible_args},
        utils::ident_to_lit_str,
    },
};

pub fn mock(
    methods: &[RemoteMethod],
    mock_name: &TokenStream,
    mock_builder_name: &TokenStream,
    stub_trait_name: &TokenStream,
    sub_services: &[SubService],
    opposite_stub_alias: &str,
) -> TokenStream {
    let struct_definition = struct_definition(mock_name, sub_services);

    let new_method = new_method(mock_builder_name);

    let stub_trait_implementation = stub_trait_implementation(
        methods,
        mock_name,
        stub_trait_name,
        sub_services,
        opposite_stub_alias,
    );

    quote! {
        #struct_definition

        impl #mock_name {
            #new_method
        }

        #stub_trait_implementation
    }
}

fn struct_definition(mock_name: &TokenStream, sub_services: &[SubService]) -> TokenStream {
    let sub_services_fields_definition = sub_services
        .iter()
        .filter(|sub_service| sub_service.pub_keyword.is_some())
        .map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon,
                 path,
             }| quote!(#name #colon #path::#mock_name),
        );

    quote! {
        /// A mock implementation of the service stub.
        pub struct #mock_name {
            service_path: Option<String>,
            expectations: std::sync::Arc<std::sync::Mutex<rpc_genie::mock::Expectations>>,
            #(#sub_services_fields_definition,)*
        }
    }
}

fn new_method(mock_builder_name: &TokenStream) -> TokenStream {
    quote! {
        /// Creates a mock and configures its expectations.
        ///
        /// See more information at [`rpc_genie::mock`]
        pub fn new(expectation_setter_callback: impl FnOnce(&mut #mock_builder_name)) -> Self {
            let expectations = std::sync::Arc::new(std::sync::Mutex::new(
                rpc_genie::mock::Expectations::default()
            ));
            let mut mock_builder = #mock_builder_name::new(None, expectations);
            expectation_setter_callback(&mut mock_builder);
            mock_builder.build()
        }
    }
}

fn stub_trait_implementation(
    methods: &[RemoteMethod],
    mock_name: &TokenStream,
    stub_trait_name: &TokenStream,
    sub_services: &[SubService],
    opposite_stub_alias: &str,
) -> TokenStream {
    let remote_methods = stub_trait_remote_methods(methods, opposite_stub_alias);
    let sub_services_getters = stub_trait_sub_services_getters(stub_trait_name, sub_services);

    quote! {
        impl #stub_trait_name for #mock_name {
            #(#remote_methods)*
            #sub_services_getters
        }
    }
}

fn stub_trait_remote_methods(
    methods: &[RemoteMethod],
    opposite_stub_alias: &str,
) -> impl Iterator<Item = TokenStream> {
    methods
        .iter()
        .filter_map(|m| stub_trait_remote_method(m, opposite_stub_alias))
}

fn stub_trait_remote_method(
    RemoteMethod {
        vis,
        is_async: _,
        ident,
        receiver: _,
        args,
        output,
    }: &RemoteMethod,
    opposite_stub_alias: &str,
) -> Option<TokenStream> {
    if let syn::Visibility::Inherited = &vis {
        return None;
    }

    let args = visible_args(args, opposite_stub_alias);
    let inputs = args
        .iter()
        .map(|(name, colon, ty)| quote!(#name #colon #ty));
    let args_encoder = args.iter().map(|(name, _, ty)| {
        quote! {
            rpc_genie::mock::encode_value::<#ty>(#name)
        }
    });
    let output_type = match output {
        syn::ReturnType::Default => quote!(()),
        syn::ReturnType::Type(_, output_type) => quote!(#output_type),
    };
    let output = send_single_request_output(output);
    let method_name_str = ident_to_lit_str(ident);

    Some(quote! {
        fn #ident(&self, #(#inputs,)*) #output {
            let mut __rpc_genie_encoded_args__ = Vec::new();
            #(__rpc_genie_encoded_args__.push(#args_encoder);)*

            rpc_genie::mock::MockSingleRequestSender::<#output_type> {
                method_path: self.service_path.clone().unwrap_or_else(|| String::new())
                    + #method_name_str,
                encoded_args: __rpc_genie_encoded_args__.into_boxed_slice(),
                expectations: std::sync::Arc::clone(&self.expectations),
                return_value: std::marker::PhantomData,
            }
        }
    })
}

fn stub_trait_sub_services_getters(
    stub_trait_name: &TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let sub_services_getters = sub_services.iter().filter_map(
        |SubService {
             pub_keyword,
             name,
             colon: _,
             path,
         }| {
            (*pub_keyword)?;

            Some(quote! {
                fn #name(&self) -> &impl #path::#stub_trait_name {
                    &self.#name
                }
            })
        },
    );

    quote!(#(#sub_services_getters)*)
}
