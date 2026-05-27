use crate::{RemoteMethod, SubService};

use super::*;
use proc_macro2::TokenStream;
use quote::quote;

fn parse_token_stream(token_stream: TokenStream) -> syn::Result<Service> {
    syn::parse_str::<Service>(&token_stream.to_string())
}

#[derive(Debug, PartialEq)]
struct ComparisonService {
    module_ident: String,

    server: ComparisonStruct,
    server_remote_methods: Vec<ComparisonRemoteMethod>,
    client: ComparisonStruct,
    client_remote_methods: Vec<ComparisonRemoteMethod>,
    sub_services: Vec<ComparisonSubService>,
}

#[derive(Debug, PartialEq)]
struct ComparisonStruct {
    vis: String,
    fields: Vec<ComparisonField>,
}

#[derive(Debug, PartialEq)]
struct ComparisonField {
    vis: String,
    ident: Option<String>,
    ty: String,
}

#[derive(Debug, PartialEq)]
struct ComparisonRemoteMethod {
    ident: String,
    receiver: Option<String>,
    args: Vec<ComparisonArg>,
}

#[derive(Debug, PartialEq)]
struct ComparisonArg {
    pat: String,
    ty: String,
}

#[derive(Debug, PartialEq)]
struct ComparisonSubService {
    vis: String,
    ident: String,
    path: String,
}

impl From<Service> for ComparisonService {
    fn from(service: Service) -> Self {
        ComparisonService {
            module_ident: service.module.ident.to_string(),
            server: service.server.into(),
            server_remote_methods: service
                .server_remote_methods
                .into_iter()
                .map(Into::into)
                .collect(),
            client: service.client.into(),
            client_remote_methods: service
                .client_remote_methods
                .into_iter()
                .map(Into::into)
                .collect(),
            sub_services: service.sub_services.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<syn::ItemStruct> for ComparisonStruct {
    fn from(struct_item: syn::ItemStruct) -> ComparisonStruct {
        let vis = struct_item.vis;

        ComparisonStruct {
            vis: {
                let vis_tokens = quote!(#vis);
                vis_tokens.to_string()
            },
            fields: struct_item.fields.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<syn::Field> for ComparisonField {
    fn from(field: syn::Field) -> Self {
        let vis = field.vis;
        let ty = field.ty;

        ComparisonField {
            vis: quote!(#vis).to_string(),
            ident: field.ident.as_ref().map(|i| i.to_string()),
            ty: quote!(#ty).to_string(),
        }
    }
}

impl From<RemoteMethod> for ComparisonRemoteMethod {
    fn from(remote_method: RemoteMethod) -> Self {
        ComparisonRemoteMethod {
            ident: remote_method.ident.to_string(),
            receiver: remote_method
                .receiver
                .map(|receiver| quote!(#receiver).to_string()),
            args: remote_method.args.into_iter().map(Into::into).collect(),
        }
    }
}

impl From<syn::PatType> for ComparisonArg {
    fn from(arg: syn::PatType) -> Self {
        let pat = arg.pat;
        let ty = arg.ty;

        ComparisonArg {
            pat: quote!(#pat).to_string(),
            ty: quote!(#ty).to_string(),
        }
    }
}

impl From<SubService> for ComparisonSubService {
    fn from(sub_service: SubService) -> Self {
        let pub_keyword = sub_service.pub_keyword;
        let path_tokens = sub_service.path;
        ComparisonSubService {
            vis: quote!(#pub_keyword).to_string(),
            ident: sub_service.name.to_string(),
            path: quote!(#path_tokens).to_string(),
        }
    }
}

macro_rules! expect_error {
    ($token_stream:expr, $expected_error:expr $(,)?) => {
        let result = parse_token_stream($token_stream);

        match result {
            Ok(_) => panic!("expected error: \"{}\", got Ok(...)", $expected_error),
            Err(err) => assert_eq!(err.to_string(), $expected_error),
        }
    };
}

#[test]
fn error_if_duplicate_server() {
    let service = quote! {
        mod foo {
            struct Server;
            struct Server;
        }
    };
    expect_error!(service, "Server struct was already defined");
}

#[test]
fn error_if_duplicate_client() {
    let service = quote! {
        mod foo {
            struct Client;
            struct Client;
        }
    };
    expect_error!(service, "Client struct was already defined");
}

#[test]
fn error_if_sub_services_name_is_also_a_server_field_name() {
    let service = quote! {
        mod foo {
            sub_services! {
                test: super::bar
            }

            struct Server { test: u32 }
        }
    };
    expect_error!(
        service,
        "field name can not be the same as a sub-service name"
    );
}

#[test]
fn error_if_sub_services_name_is_also_a_client_field_name() {
    let service = quote! {
        mod foo {
            sub_services! {
                test: super::bar
            }

            struct Client { test: u32 }
        }
    };
    expect_error!(
        service,
        "field name can not be the same as a sub-service name"
    );
}

#[test]
fn error_if_sub_services_macro_doesnt_use_braces() {
    let service = quote! {
        mod foo {
            sub_services![test: super::bar];
        }
    };
    expect_error!(service, "sub_services macro expects curly brackets");

    let service = quote! {
        mod foo {
            sub_services!(test: super::bar);
        }
    };
    expect_error!(service, "sub_services macro expects curly brackets");
}

#[test]
fn error_if_sub_services_names_are_duplicated() {
    let service = quote! {
        mod foo {
            sub_services! {
                test: super::bar,
                test: super::bar,
            }
        }
    };
    expect_error!(service, "name is already taken by another sub-service");

    let service = quote! {
        mod foo {
            sub_services! { test: super::bar }
            sub_services! { test: super::bar }
        }
    };
    expect_error!(service, "name is already taken by another sub-service");
}

#[test]
fn error_if_service_on_something_other_than_a_module() {
    let service = quote! {
        fn foo() {}
    };

    expect_error!(service, "expected `mod`");
}

#[test]
fn error_if_remote_methods_block_has_functions_that_use_attributes() {
    let service = quote! {
        mod foo {
            struct Server {}

            #[remote_methods]
            impl Server {
                #[foo]
                fn foo() {}
            }
        }
    };

    expect_error!(service, "Remote methods are not allowed to use attributes");
}

#[test]
fn error_if_remote_methods_attr_is_not_the_only_attribute() {
    let service = quote! {
        mod foo {
            struct Server {}

            #[remote_methods]
            #[foo]
            impl Server {
                fn foo() {}
            }
        }
    };

    expect_error!(
        service,
        "When #[remote_methods] attribute is used, other attributes are not allowed",
    );
}

#[test]
fn error_if_server_struct_is_missing() {
    let service = quote! {
        mod foo {
            struct Client {}
        }
    };
    expect_error!(
        service,
        "rpc_genie::service macro requires a Server struct to be present in the module",
    );
}

#[test]
fn error_if_client_struct_is_missing() {
    let service = quote! {
        mod foo {
            struct Server {}
        }
    };
    expect_error!(
        service,
        "rpc_genie::service macro requires a Client struct to be present in the module",
    );
}

#[test]
fn error_if_remote_methods_attr_is_present_multiple_times() {
    let service = quote! {
        mod foo {
            struct Server {}

            #[remote_methods]
            #[remote_methods]
            impl Server {
                fn foo() {}
            }
        }
    };

    expect_error!(
        service,
        "#[remote_methods] attribute should only be present once",
    );
}

#[test]
fn error_if_remote_methods_attr_is_on_an_impl_block_that_is_neither_client_nor_server() {
    let service = quote! {
        mod foo {
            struct Foo {}

            #[remote_methods]
            impl Foo {
                fn foo() {}
            }
        }
    };

    expect_error!(
        service,
        "Only the Server and Client structs may use the #[remote_methods] attribute",
    );
}

#[test]
fn error_if_no_module_body() {
    let error_string_generator = |mod_name| {
        format!(
            "module must have a body (e.g., `mod {mod_name} {{ ... }}`) instead of just `mod \
             {mod_name};`"
        )
    };

    let service = quote! {
        mod foo;
    };
    expect_error!(service, error_string_generator("foo"));

    let service = quote! {
        mod bar;
    };
    expect_error!(service, error_string_generator("bar"));
}

#[test]
fn error_if_remote_methods_has_arguments() {
    let service = quote! {
        mod foo {
            #[remote_methods(42)]
            impl Server {}
        }
    };
    expect_error!(service, "#[remote_methods] does not take any arguments");

    let service = quote! {
        mod foo {
            #[remote_methods{name = "foo"}]
            impl Server {}
        }
    };
    expect_error!(service, "#[remote_methods] does not take any arguments");
}

#[test]
fn with_sub_service() {
    let service = quote! {
        pub mod rpc {
            sub_services! {
                pub counter: super::counter,
            }

            pub struct Server {
                pub server_name: String,
            }

            #[remote_methods]
            impl Server {
                async fn get_server_name(&self) -> String {
                    self.server_name.clone()
                }
            }

            pub struct Client {}
        }
    };

    let service: ComparisonService = parse_token_stream(service).unwrap().into();

    assert_eq!(
        service,
        ComparisonService {
            module_ident: "rpc".to_owned(),

            server: ComparisonStruct {
                vis: "pub".to_owned(),
                fields: vec![ComparisonField {
                    vis: "pub".to_owned(),
                    ident: Some("server_name".to_owned()),
                    ty: "String".to_owned(),
                }],
            },

            server_remote_methods: vec![ComparisonRemoteMethod {
                ident: "get_server_name".to_owned(),
                receiver: Some("& self".to_owned()),
                args: vec![],
            }],

            client: ComparisonStruct {
                vis: "pub".to_owned(),
                fields: vec![],
            },

            client_remote_methods: vec![],

            sub_services: vec![ComparisonSubService {
                vis: "pub".to_owned(),
                ident: "counter".to_owned(),
                path: "super :: counter".to_owned(),
            }],
        }
    );
}

#[test]
fn with_client_handle() {
    let service = quote! {
        #[rpc_genie::service]
        pub mod counter {
            pub struct Server {
                pub count: tokio::sync::Mutex<u32>,
                pub subscribed_clients: rpc_genie::SubscribedClients<Client>,
            }

            #[remote_methods]
            impl Server {
                async fn increment(&self) {
                    let new_value = {
                        let lock = self.count.lock().await;
                        *lock += 1;
                        *lock
                    };
                    subscribed_clients
                        .for_each(async |client| client.change_event(new_value).await)
                    .await
                }

                async fn decrement(&self) {
                    let new_value = {
                        let lock = self.count.lock().await;
                        *lock -= 1;
                        *lock
                    };
                    self
                        .subscribed_clients
                        .for_each(async |client| client.change_event(new_value).await)
                    .await
                }

                async fn get(&self) -> u32 {
                    *self.count.lock().await
                }

                async fn subscribe(&self, client: rpc_genie::ClientHandle<Client>) {
                    let count_lock = self.count.lock().await;
                    client.change_event(count_lock).await;
                    self.subscribed_clients.add(client).await
                }
            }

            pub struct Client {
                pub current_count: tokio::sync::Mutex<u32>,
            }

            #[remote_methods]
            impl Client {
                async fn change_event(&self, new_value: u32) {
                    *self.current_count.lock().await = new_value;
                }
            }
        }
    };

    let service: ComparisonService = parse_token_stream(service).unwrap().into();
    assert_eq!(
        service,
        ComparisonService {
            module_ident: "counter".to_owned(),

            server: ComparisonStruct {
                vis: "pub".to_owned(),
                fields: vec![
                    ComparisonField {
                        vis: "pub".to_owned(),
                        ident: Some("count".to_owned()),
                        ty: "tokio :: sync :: Mutex < u32 >".to_owned(),
                    },
                    ComparisonField {
                        vis: "pub".to_owned(),
                        ident: Some("subscribed_clients".to_owned()),
                        ty: "rpc_genie :: SubscribedClients < Client >".to_owned(),
                    },
                ],
            },

            server_remote_methods: vec![
                ComparisonRemoteMethod {
                    ident: "increment".to_owned(),
                    receiver: Some("& self".to_owned()),
                    args: vec![],
                },
                ComparisonRemoteMethod {
                    ident: "decrement".to_owned(),
                    receiver: Some("& self".to_owned()),
                    args: vec![],
                },
                ComparisonRemoteMethod {
                    ident: "get".to_owned(),
                    receiver: Some("& self".to_owned()),
                    args: vec![],
                },
                ComparisonRemoteMethod {
                    ident: "subscribe".to_owned(),
                    receiver: Some("& self".to_owned()),
                    args: vec![ComparisonArg {
                        pat: "client".to_owned(),
                        ty: "rpc_genie :: ClientHandle < Client >".to_owned(),
                    }],
                },
            ],

            client: ComparisonStruct {
                vis: "pub".to_owned(),
                fields: vec![ComparisonField {
                    vis: "pub".to_owned(),
                    ident: Some("current_count".to_owned()),
                    ty: "tokio :: sync :: Mutex < u32 >".to_owned(),
                }],
            },

            client_remote_methods: vec![ComparisonRemoteMethod {
                ident: "change_event".to_owned(),
                receiver: Some("& self".to_owned()),
                args: vec![ComparisonArg {
                    pat: "new_value".to_owned(),
                    ty: "u32".to_owned()
                }]
            }],

            sub_services: vec![],
        }
    );
}
