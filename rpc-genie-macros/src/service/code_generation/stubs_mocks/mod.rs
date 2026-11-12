mod mock;
use mock::mock;

mod mock_builder;
use mock_builder::mock_builder;

use proc_macro2::TokenStream;
use quote::quote;

use crate::{
    Service,
    service::{RemoteMethod, SubService},
};

impl Service {
    pub(super) fn stub_mocks(&self) -> TokenStream {
        let server = generate_mock_items(
            &self.server_remote_methods,
            quote!(ServerStubMock),
            quote!(ServerStubMockBuilder),
            quote!(ServerStub),
            "ClientStub",
            &self.sub_services,
        );

        let client = generate_mock_items(
            &self.client_remote_methods,
            quote!(ClientStubMock),
            quote!(ClientStubMockBuilder),
            quote!(ClientStub),
            "ServerStub",
            &self.sub_services,
        );

        quote! {
            #server
            #client
        }
    }
}

fn generate_mock_items(
    methods: &[RemoteMethod],
    mock_name: TokenStream,
    mock_builder_name: TokenStream,
    stub_trait_name: TokenStream,
    opposite_stub_alias: &str,
    sub_services: &[SubService],
) -> TokenStream {
    let mock = mock(
        methods,
        &mock_name,
        &mock_builder_name,
        &stub_trait_name,
        sub_services,
        opposite_stub_alias,
    );
    let mock_builder = mock_builder(
        methods,
        &mock_name,
        &mock_builder_name,
        sub_services,
        opposite_stub_alias,
    );

    quote! {
        #mock
        #mock_builder
    }
}
