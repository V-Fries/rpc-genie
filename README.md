# rpc-genie

This crate is currently is final stages of testing before publishing version 0.1.0 to crates.io.

## Typed bidirectional RPC over TCP or Unix sockets

## Usage:

- Define a service with the [`service`] macro
- Use [`Tcp`]/[`UnixSocket`] to start the server and connect the clients

## Example:

```rust
mod services {
    #[rpc_genie::service]
    pub mod a {
        // Define other services to include in the current service
        sub_services! {
            // Can have any number of sub services
            pub message_publisher: super::message_publisher,
        }

        pub struct Server<Stream> {
            // Server will have one field per sub_services added automatically
            pub server_name: String,
        }

        impl<Stream> Server<Stream> {
            // The #[remote_method] attribute marks this method as being callable from the
            // client
            #[remote_method]
            pub fn get_server_name(&self) -> String {
                self.server_name.clone()
            }

            #[remote_method]
            pub async fn foo(client_stub: ClientStub) {
                // When an argument has the type ClientStub, it allows to call remote methods
                // on the client.
                assert_eq!(6, client_stub.add(4, 2).call().await.unwrap());

                // sub services can also be accessed through the stub
                client_stub.message_publisher.print("foo".to_owned()).notify().await.unwrap();
            }
        }

        pub struct Client<Stream> {
            // Client will have one field per sub_services added automatically
        }

        impl<Stream> Client<Stream> {
            // Clients can also have methods with #[remote_method], making the method callable
            // from the server
            #[remote_method]
            pub fn add(a: u32, b: u32) -> u32 {
                a + b
            }

            #[remote_method]
            pub async fn bar(server_stub: ServerStub) {
                // The client can also get a stub to the server as an argument to a remote
                // method
                let _server_name = server_stub.get_server_name().call().await.unwrap();
                server_stub.message_publisher.publish("bar".to_owned()).notify().await.unwrap();
            }
        }
    }

    #[rpc_genie::service]
    pub mod message_publisher {
        use std::sync::Arc;

        pub struct Server<Stream> {
            // topic makes it trivial to write pub/subs
            pub topic: Topic,
        }

        impl<Stream> Server<Stream> {
            #[remote_method]
            pub async fn subscribe(&self, client_stub: ClientStub) {
                self.topic.subscribe(Arc::clone(client_stub)).await
            }

            #[remote_method]
            pub async fn unsubscribe(&self, client_stub: ClientStub) {
                self.topic.unsubscribe(client_stub).await
            }

            #[remote_method]
            pub async fn publish(&self, msg: String) {
                // Call the print method on each subscribed clients
                self.topic.for_each(async |client_stub| {
                    client_stub.print(msg.clone()).notify().await.unwrap()
                }).await
            }
        }

        pub struct Client<Stream> {}

        impl<Stream> Client<Stream> {
            #[remote_method]
            pub fn print(msg: String) {
                println!("{msg}")
            }
        }
    }
}

use tokio::{task::JoinHandle, sync::oneshot};
use rpc_genie::Topic;
use std::{time::Duration, sync::Arc, marker::PhantomData};

const MAX_FRAME_SIZE: usize = 1024 * 1024;
const ADDR: &str = "127.0.0.1:45864";

#[tokio::main]
async fn main() {
    let (server_doesnt_need_client_anymore_sender, server_doesnt_need_client_anymore_receiver)
        = oneshot::channel();

    let server_join_handle = tokio::spawn(async move {
        server(server_doesnt_need_client_anymore_sender).await
    });

    client(server_doesnt_need_client_anymore_receiver).await;

    server_join_handle.await.unwrap();
}

async fn server(server_doesnt_need_client_anymore_sender: oneshot::Sender<()>) {
    let server_handle = rpc_genie::tcp::start_server(
        ADDR,
        MAX_FRAME_SIZE,
        Arc::new(services::a::Server {
            server_name: "server".to_string(),
            // Here we see that the sub services state are added to the parent service state
            message_publisher: Arc::new(services::message_publisher::Server {
                topic: Topic::new().await,
            }),
        }),
    )
    .await
    .unwrap();

    // wait till client connects before running our tests
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if server_handle.map_each_client(|_| async {}).await.len() == 1 {
                break;
            }
        }
    })
    .await
    .expect("server did not observe the connected client");

    // broadcast message with for_each_client()
    let () = server_handle
        .for_each_client(async |client| {
                client
                    .message_publisher
                    .print("hey".to_string())
                    .call()
                    .await
                    .unwrap()
        })
        .await;

    // use map_each_client() to get the result back
    assert_eq!(
        vec![6],
        server_handle
            .map_each_client(async |client| client.add(4, 2).call().await.unwrap())
            .await
    );

    server_doesnt_need_client_anymore_sender.send(()).unwrap();
}

async fn client(server_doesnt_need_client_anymore_receiver: oneshot::Receiver<()>) {
    let client_handle = tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            let result = rpc_genie::tcp::connect_client(
                ADDR,
                MAX_FRAME_SIZE,
                Arc::new(services::a::Client {
                    // Here we see that the sub services state are added to the parent service state
                    message_publisher: Arc::new(services::message_publisher::Client {
                        // When a service has neither a sub-service nor a topic, we need to add
                        // a phantom data of the request sender
                        _stream: PhantomData,
                    }),
                })
            ).await;

            if let Ok(client_handle) = result {
                return client_handle;
            }
        }
    })
    .await
    .expect("client failed to connect to server");

    // You can then call the server's remote method directly
    assert_eq!(
        "server",
        client_handle
            .server_stub
            .get_server_name()
            .call()
            .await
            .unwrap()
    );

    // use notify if you don't care about the result and don't need to check whether the server
    // received the request
    let () = client_handle
        .server_stub
        .get_server_name()
        .notify()
        .await
        .unwrap();

    server_doesnt_need_client_anymore_receiver.await.unwrap();
}
```

## Planned features:

- Logging (will probably use the [tracing](https://docs.rs/tracing/latest/tracing/) crate)
- On connect and on disconnect events
- Request time-out
- Client auto-reconnect on error
- Request encryption (for now, use something like wireguard if you transfer data over the net)

## Limitations:

- Still in early development, breaking changes may often occur
