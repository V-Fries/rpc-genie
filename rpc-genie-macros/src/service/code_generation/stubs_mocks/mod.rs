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
            quote!(ServerStub),
            quote!(ServerStubMock),
            quote!(ServerStubMockBuilder),
            "ClientStub",
            &self.sub_services,
        );

        let client = generate_mock_items(
            &self.client_remote_methods,
            quote!(ClientStub),
            quote!(ClientStubMock),
            quote!(ClientStubMockBuilder),
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
    stub_name: TokenStream,
    mock_name: TokenStream,
    mock_builder_name: TokenStream,
    opposite_stub_alias: &str,
    sub_services: &[SubService],
) -> TokenStream {
    let mock = mock(&stub_name, &mock_name, &mock_builder_name, sub_services);
    let mock_builder = mock_builder(
        methods,
        &mock_name,
        &mock_builder_name,
        opposite_stub_alias,
        sub_services,
    );

    quote! {
        #mock
        #mock_builder
    }
}
