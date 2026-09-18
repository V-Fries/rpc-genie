#[rpc_genie::service]
mod counter {
    pub struct Server<RequestSender> {
        pub count: std::sync::atomic::AtomicU64,
        pub topic: Topic,
    }

    impl<RequestSender> Server<RequestSender> {
        #[remote_method]
        pub async fn subscribe(&self, client: ClientStub) {
            self.topic.subscribe(std::sync::Arc::clone(client)).await;

            client
                .update_count(self.count.load(std::sync::atomic::Ordering::Relaxed))
                .notify()
                .await
                .unwrap()
        }

        #[remote_method]
        pub async fn unsubscribe(&self, client: ClientStub) {
            self.topic.unsubscribe(&client).await;
        }

        #[remote_method]
        pub async fn increment(&self) {
            let new_count = 1 + self
                .count
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);

            self.topic
                .for_each(async |client| {
                    client.update_count(new_count).notify().await.unwrap();
                })
                .await
        }
    }

    pub struct Client<RequestSender> {
        pub count: std::sync::atomic::AtomicU64,
    }

    impl<RequestSender> Client<RequestSender> {
        #[remote_method]
        fn update_count(&self, new_count: u64) {
            self.count
                .fetch_max(new_count, std::sync::atomic::Ordering::Relaxed);
        }
    }
}

#[tokio::test]
async fn counter_test() {
    use std::{
        sync::{
            Arc,
            atomic::{self, AtomicU64},
        },
        time::Duration,
    };
    use tokio::time::{sleep, timeout};

    let server_handle = rpc_genie::Tcp::<1024>::start_server(
        "127.0.0.1:51235",
        Arc::new(counter::Server {
            count: AtomicU64::new(0),
            topic: rpc_genie::Topic::new().await,
        }),
    )
    .await
    .unwrap();

    let client_handle = timeout(Duration::from_secs(1), async {
        loop {
            if let Ok(client_handle) = rpc_genie::Tcp::<1024>::connect_client(
                "127.0.0.1:51235",
                Arc::new(counter::Client {
                    count: AtomicU64::new(0),
                    _request_sender: std::marker::PhantomData,
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

    client_handle.server_stub.subscribe().call().await.unwrap();
    client_handle.server_stub.increment().call().await.unwrap();

    timeout(Duration::from_secs(1), async {
        loop {
            if client_handle.state.count.load(atomic::Ordering::Relaxed) == 1 {
                return;
            }
        }
    })
    .await
    .expect("client never received the new count");

    client_handle
        .server_stub
        .unsubscribe()
        .call()
        .await
        .unwrap();
    client_handle.server_stub.increment().call().await.unwrap();
    sleep(Duration::from_secs(1)).await;
    // since we unsubscribed, we shouldn't have received the update
    assert_eq!(client_handle.state.count.load(atomic::Ordering::Relaxed), 1);

    client_handle.server_stub.subscribe().call().await.unwrap();
    timeout(Duration::from_secs(1), async {
        loop {
            if client_handle.state.count.load(atomic::Ordering::Relaxed) == 2 {
                return;
            }
        }
    })
    .await
    .expect("client never received the new count");

    server_handle.stop();
}
