use proc_macro2::TokenStream;
use quote::quote;
use syn::ReturnType;

use crate::service::{
    RemoteMethod, SubService,
    code_generation::{stub_traits::visible_args, utils::ident_to_lit_str},
};

pub fn mock_builder(
    methods: &[RemoteMethod],
    mock_name: &TokenStream,
    mock_builder_name: &TokenStream,
    sub_services: &[SubService],
    opposite_stub_alias: &str,
) -> TokenStream {
    let struct_def = mock_builder_struct(mock_builder_name, sub_services);

    let constructor = mock_builder_constructor(mock_builder_name, sub_services);
    let build_method = mock_builder_build_method(mock_name, mock_builder_name, sub_services);
    let expectation_setters = expectation_setters(
        methods,
        mock_builder_name,
        sub_services,
        opposite_stub_alias,
    );

    quote! {
        #struct_def

        impl #mock_builder_name {
            #constructor
            #build_method
            #(#expectation_setters)*
        }
    }
}

fn mock_builder_struct(
    mock_builder_name: &TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let sub_services_fields_definition = sub_services
        .iter()
        .filter(|sub_service| sub_service.pub_keyword.is_some())
        .map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon,
                 path,
             }| quote!(#name #colon #path::#mock_builder_name),
        );

    quote! {
        /// Configures expectations for the service mock.
        pub struct #mock_builder_name {
            expectations: std::sync::Arc<std::sync::Mutex<rpc_genie::mock::Expectations>>,
            service_path: Option<String>,
            #(#sub_services_fields_definition,)*
        }
    }
}

fn mock_builder_constructor(
    mock_builder_name: &TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let sub_services_fields_creation = sub_services
        .iter()
        .filter(|sub_service| sub_service.pub_keyword.is_some())
        .map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon,
                 path,
             }| {
                let name_as_str = ident_to_lit_str(name);
                quote! {
                    #name #colon #path::#mock_builder_name::new(
                        Some(service_path.clone().unwrap_or_else(|| String::new()) + #name_as_str + "/"),
                        std::sync::Arc::clone(&expectations),
                    )
                }
            },
        );

    quote! {
        #[doc(hidden)]
        pub fn new(
            service_path: Option<String>,
            expectations: std::sync::Arc<std::sync::Mutex<rpc_genie::mock::Expectations>>
        ) -> Self {
            Self {
                #(#sub_services_fields_creation,)*
                service_path,
                expectations,
            }
        }
    }
}

fn mock_builder_build_method(
    mock_name: &TokenStream,
    mock_builder_name: &TokenStream,
    sub_services: &[SubService],
) -> TokenStream {
    let sub_services_fields_creation = sub_services
        .iter()
        .filter(|sub_service| sub_service.pub_keyword.is_some())
        .map(
            |SubService {
                 pub_keyword: _,
                 name,
                 colon,
                 path,
             }| quote!(#name #colon #path::#mock_builder_name::build(self.#name)),
        );

    quote! {
        #[doc(hidden)]
        pub fn build(self) -> #mock_name {
            #mock_name {
                service_path: self.service_path,
                expectations: self.expectations,
                #(#sub_services_fields_creation,)*
            }
        }
    }
}

fn expectation_setters(
    methods: &[RemoteMethod],
    mock_builder_name: &TokenStream,
    sub_services: &[SubService],
    opposite_stub_alias: &str,
) -> impl Iterator<Item = TokenStream> {
    methods
        .iter()
        .filter_map(|m| expectation_setter(m, opposite_stub_alias))
        .chain(
            sub_services
                .iter()
                .filter_map(|s| sub_service_expectation_setter(mock_builder_name, s)),
        )
}

fn expectation_setter(
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
    let vis = match &vis {
        syn::Visibility::Inherited => return None,
        vis => vis,
    };

    let args = visible_args(args, opposite_stub_alias);
    let inputs = args
        .iter()
        .map(|(name, colon, ty)| quote!(#name #colon #ty));
    let args_encoder = args.iter().map(|(name, _, ty)| {
        quote! {
            rpc_genie::mock::encode_value::<#ty>(#name)
        }
    });
    let output = expectation_setter_output(output);
    let method_name_str = ident_to_lit_str(ident);

    Some(quote! {
        /// Adds an expectation for this remote method.
        #vis fn #ident(&self, #(#inputs,)*) #output {
            let mut __rpc_genie_encoded_args__ = Vec::new();
            #(__rpc_genie_encoded_args__.push(#args_encoder);)*

            rpc_genie::mock::ExpectationSetter {
                method_path: self.service_path.clone().unwrap_or_else(|| String::new())
                    + #method_name_str,
                encoded_args: __rpc_genie_encoded_args__.into_boxed_slice(),
                expectations: &*self.expectations,
                return_value: std::marker::PhantomData,
            }
        }
    })
}

fn expectation_setter_output(output: &ReturnType) -> TokenStream {
    match output {
        ReturnType::Default => quote!(-> rpc_genie::mock::ExpectationSetter<'_, ()>),
        ReturnType::Type(arrow, type_ast) => {
            quote!(#arrow rpc_genie::mock::ExpectationSetter<'_, #type_ast>)
        }
    }
}

fn sub_service_expectation_setter(
    mock_builder_name: &TokenStream,
    SubService {
        pub_keyword,
        name,
        colon: _,
        path,
    }: &SubService,
) -> Option<TokenStream> {
    let pub_keyword = (*pub_keyword)?;

    Some(quote! {
        /// Configures expectations for this child service.
        #pub_keyword fn #name(&self) -> &#path::#mock_builder_name {
            &self.#name
        }
    })
}
