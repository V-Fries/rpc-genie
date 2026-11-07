//! Integration test for nested sub-service paths.
//!
//! `root -> child -> grandchild` exercises both branches of the generated
//! service path construction: the top-level (compile-time `concat!`) branch
//! and the nested (runtime-prefixed) branch, in both directions.

use std::{marker::PhantomData, sync::Arc};

#[rpc_genie::service]
mod root {
    sub_services! {
        pub child: super::child,
    }

    pub struct Server<Stream> {}

    impl<Stream> Server<Stream> {
        #[remote_method]
        pub fn root_ping() -> u32 {
            10
        }

        // Exercises the server's stub tree down to the grandchild.
        #[remote_method]
        pub async fn call_client_grandchild(client: ClientStub) -> u32 {
            client
                .child
                .grandchild
                .client_grandchild_ping()
                .call()
                .await
                .unwrap()
        }
    }

    pub struct Client<Stream> {}

    impl<Stream> Client<Stream> {
        #[remote_method]
        pub fn client_root_ping() -> u32 {
            20
        }
    }
}

#[rpc_genie::service]
mod child {
    sub_services! {
        pub grandchild: super::grandchild,
    }

    pub struct Server<Stream> {}

    impl<Stream> Server<Stream> {
        #[remote_method]
        pub fn child_ping() -> u32 {
            30
        }
    }

    pub struct Client<Stream> {}

    impl<Stream> Client<Stream> {
        #[remote_method]
        pub fn client_child_ping() -> u32 {
            40
        }
    }
}

#[rpc_genie::service]
mod grandchild {
    pub struct Server<Stream> {}

    impl<Stream> Server<Stream> {
        #[remote_method]
        pub fn grandchild_ping() -> u32 {
            50
        }
    }

    pub struct Client<Stream> {}

    impl<Stream> Client<Stream> {
        #[remote_method]
        pub fn client_grandchild_ping() -> u32 {
            60
        }
    }
}

const MAX_FRAME_SIZE: usize = 1024 * 1024;

async fn free_local_addr() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    drop(listener);
    addr.to_string()
}

#[tokio::test]
async fn nested_sub_service_paths_route_correctly() {
    let addr = free_local_addr().await;

    let _server_handle = rpc_genie::tcp::start_server(
        &addr,
        MAX_FRAME_SIZE,
        Arc::new(root::Server {
            child: Arc::new(child::Server {
                grandchild: Arc::new(grandchild::Server {
                    _stream: PhantomData,
                }),
            }),
        }),
    )
    .await
    .unwrap();

    let client = rpc_genie::tcp::connect_client(
        &addr,
        MAX_FRAME_SIZE,
        Arc::new(root::Client {
            child: Arc::new(child::Client {
                grandchild: Arc::new(grandchild::Client {
                    _stream: PhantomData,
                }),
            }),
        }),
    )
    .await
    .unwrap();

    // Depth 0: no service path.
    assert_eq!(client.server_stub.root_ping().call().await.unwrap(), 10);

    // Depth 1: compile-time `concat!` path.
    assert_eq!(
        client.server_stub.child.child_ping().call().await.unwrap(),
        30
    );

    // Depth 2: runtime-prefixed path.
    assert_eq!(
        client
            .server_stub
            .child
            .grandchild
            .grandchild_ping()
            .call()
            .await
            .unwrap(),
        50
    );

    // Server -> client through the same nested path.
    assert_eq!(
        client
            .server_stub
            .call_client_grandchild()
            .call()
            .await
            .unwrap(),
        60
    );

    client.disconnect();
}
