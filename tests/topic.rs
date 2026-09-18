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
