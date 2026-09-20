// To see the generated code, you can run `cargo expand --test basic_example`

#[rpc_genie::service]
pub mod service_a {
    use std::sync::atomic::{self, AtomicU32};

    // Define other services to include in the current service
    sub_services! {
        // Can have any number of sub services
        pub sub_service_1: super::service_b,
        pub sub_service_2: super::service_b,
    }

    pub struct Server<RequestSender> {
        pub name: String,
    }

    impl<RequestSender> Server<RequestSender> {
        // pass &self for stateful functions (use mutexes and other solutions for mutability)
        #[remote_method]
        pub fn server_name(&self) -> String {
            self.name.clone()
        }

        // Don't pass &self if you don't need the state
        #[remote_method]
        pub fn add(a: u32, b: u32) -> u32 {
            a + b
        }

        #[remote_method]
        pub fn add_4(a: u32) -> u32 {
            // Self::add() is designated as a remote method, but it can still be called locally:
            Self::add(a, 4)
        }

        #[remote_method]
        pub fn access_sub_service_1_state(&self) -> u32 {
            self.sub_service_1.some_state
        }

        #[remote_method]
        pub fn access_sub_service_2_state(&self) -> u32 {
            self.sub_service_2.some_state
        }

        #[remote_method]
        pub fn complicated_pattern_arg((a, (b, c)): (u32, (String, f32))) {
            println!("received: ({a}, ({b}, {c}))");
        }
    }

    pub struct Client<RequestSender> {
        pub count: AtomicU32,
    }

    impl<RequestSender> Client<RequestSender> {
        // You can also define remote methods on the client. These will be callable from the server
        #[remote_method]
        pub fn increment_count(&self) -> u32 {
            self.count.fetch_add(1, atomic::Ordering::Relaxed)
        }
    }
}

#[rpc_genie::service]
mod service_b {
    pub struct Server<RequestSender> {
        pub some_state: u32,
    }

    pub struct Client<RequestSender> {}

    impl<RequestSender> Client<RequestSender> {
        #[remote_method]
        pub fn add(a: u32, b: u32) -> u32 {
            a + b
        }
    }
}

#[tokio::test]
async fn test() {
    let (client_disconnected_sender, client_disconnected_receiver) =
        tokio::sync::oneshot::channel();
    let (server_doesnt_need_client_anymore_sender, server_doesnt_need_client_anymore_receiver) =
        tokio::sync::oneshot::channel();

    let server_join_handle = tokio::spawn(async move {
        server(
            client_disconnected_receiver,
            server_doesnt_need_client_anymore_sender,
        )
        .await
    });

    client(
        client_disconnected_sender,
        server_doesnt_need_client_anymore_receiver,
    )
    .await;

    server_join_handle.await.unwrap();
}

async fn server(
    client_disconnected_receiver: tokio::sync::oneshot::Receiver<()>,
    server_doesnt_need_client_anymore_sender: tokio::sync::oneshot::Sender<()>,
) {
    use std::sync::Arc;

    let server_handle = rpc_genie::Tcp::<1024>::start_server(
        // The port can be any available port, but it must match the port used by the client
        "127.0.0.1:51234",
        Arc::new(service_a::Server {
            name: "server".to_string(),
            // Here we see that the sub services state are added to the main service state
            sub_service_1: Arc::new(service_b::Server {
                some_state: 1,
                _request_sender: std::marker::PhantomData,
            }),
            // If you wanted, you could pass a clone of the Arc of sub_service_1, that way they
            // would share their state
            sub_service_2: Arc::new(service_b::Server {
                some_state: 2,
                _request_sender: std::marker::PhantomData,
            }),
        }),
    )
    .await
    .unwrap();

    server_tests(
        server_handle,
        client_disconnected_receiver,
        server_doesnt_need_client_anymore_sender,
    )
    .await
}

async fn server_tests(
    server_handle: service_a::ServerHandle<1024, tokio::net::TcpStream>,
    client_disconnected_receiver: tokio::sync::oneshot::Receiver<()>,
    server_doesnt_need_client_anymore_sender: tokio::sync::oneshot::Sender<()>,
) {
    // wait till client connects before running our tests
    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if server_handle.map_each_client(|_| async {}).await.len() == 1 {
                break;
            }

            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("server did not observe the connected client");

    // broadcast message with for_each_client()
    let () = server_handle
        .for_each_client(async |client| client.increment_count().notify().await.unwrap())
        .await;

    // use map_each_client() to get the result back
    assert_eq!(
        vec![3],
        server_handle
            .map_each_client(async |client| client.sub_service_2.add(1, 2).call().await.unwrap())
            .await
    );

    server_doesnt_need_client_anymore_sender.send(()).unwrap();
    client_disconnected_receiver.await.unwrap();

    tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if server_handle.map_each_client(|_| async {}).await.len() == 0 {
                break;
            }

            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("client was not removed from topic");

    server_handle.stop();
}

async fn client(
    client_disconnected_sender: tokio::sync::oneshot::Sender<()>,
    server_doesnt_need_client_anymore_receiver: tokio::sync::oneshot::Receiver<()>,
) {
    use std::sync::{Arc, atomic::AtomicU32};

    let client_handle = tokio::time::timeout(std::time::Duration::from_secs(1), async {
        loop {
            if let Ok(client_handle) = rpc_genie::Tcp::<1024>::connect_client(
                // The port can be any available port, but it must match the port used by the server
                "127.0.0.1:51234",
                Arc::new(service_a::Client {
                    count: AtomicU32::new(0),
                    // Here we see that the sub services state are added to the main service state
                    sub_service_1: Arc::new(service_b::Client {
                        _request_sender: std::marker::PhantomData,
                    }),
                    sub_service_2: Arc::new(service_b::Client {
                        _request_sender: std::marker::PhantomData,
                    }),
                }),
            )
            .await
            {
                return client_handle;
            }
        }
    })
    .await
    .expect("client failed to connect to server");

    client_tests(
        client_handle,
        client_disconnected_sender,
        server_doesnt_need_client_anymore_receiver,
    )
    .await
}

async fn client_tests(
    client_handle: service_a::ClientHandle<1024, tokio::net::TcpStream>,
    client_disconnected_sender: tokio::sync::oneshot::Sender<()>,
    server_doesnt_need_client_anymore_receiver: tokio::sync::oneshot::Receiver<()>,
) {
    // You can then call the server's remote method directly
    assert_eq!(
        "server",
        client_handle
            .server_stub
            .server_name()
            .call()
            .await
            .unwrap()
    );
    assert_eq!(
        42,
        client_handle.server_stub.add(40, 2).call().await.unwrap()
    );
    assert_eq!(
        42,
        client_handle.server_stub.add_4(38).call().await.unwrap()
    );
    assert_eq!(
        1,
        client_handle
            .server_stub
            .access_sub_service_1_state()
            .call()
            .await
            .unwrap()
    );
    assert_eq!(
        2,
        client_handle
            .server_stub
            .access_sub_service_2_state()
            .call()
            .await
            .unwrap()
    );

    // use notify if you don't care about the result and don't need to check whether the server
    // received the request
    client_handle
        .server_stub
        .complicated_pattern_arg((42, ("hey".to_string(), 4.)))
        .notify()
        .await
        .unwrap();

    server_doesnt_need_client_anymore_receiver.await.unwrap();

    client_handle.disconnect();
    client_disconnected_sender.send(()).unwrap();
}
