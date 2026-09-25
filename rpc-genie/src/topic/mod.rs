mod topic_id;
pub use topic_id::TopicId;

mod routine;
pub use routine::StubDiedNotificationSender;

use futures::{StreamExt, stream::FuturesUnordered};

use std::sync::Arc;

use tokio::sync::{RwLock, oneshot};

use crate::{
    SubscribableStub,
    topic::routine::{Routine, RoutineCommand, RoutineCommandSender, SubscribedStubs},
};

#[cfg(test)]
mod test;

/// A set of subscribed client stubs for broadcasting messages.
///
/// A topic removes subscriptions automatically when a stub's connection dies.
///
/// # Example:
/// ```rust
/// #[rpc_genie::service]
/// mod message_publisher {
///     use std::sync::Arc;
///
///     pub struct Server<Stream> {
///         topic: Topic,
///     }
///
///     impl<Stream> Server<Stream> {
///         #[remote_method]
///         pub async fn subscribe(&self, client_stub: ClientStub) {
///             self.topic.subscribe(Arc::clone(client_stub)).await
///         }
///
///         #[remote_method]
///         pub async fn unsubscribe(&self, client_stub: ClientStub) {
///             self.topic.unsubscribe(client_stub).await
///         }
///
///         #[remote_method]
///         pub async fn publish(&self, msg: String) {
///             self.topic.for_each(async |client_stub| {
///                 client_stub.print(msg.clone()).notify().await.unwrap()
///             }).await
///         }
///     }
///
///     pub struct Client<Stream> {}
///
///     impl<Stream> Client<Stream> {
///         // Clients can also have methods with #[remote_method], making the method callable
///         // from the server
///         #[remote_method]
///         pub fn print(msg: String) {
///             println!("{msg}")
///         }
///     }
/// }
/// ```
pub struct Topic<Stub>
where
    Stub: SubscribableStub,
{
    subscribed_stubs: Arc<RwLock<SubscribedStubs<Stub>>>,
    routine_command_sender: RoutineCommandSender<Stub>,
}

impl<Stub> Topic<Stub>
where
    Stub: Send + Sync + SubscribableStub + 'static,
{
    /// Create an empty topic.
    pub async fn new() -> Self {
        let (sender, subscribed_stubs) = Routine::spawn().await;

        Self {
            subscribed_stubs,
            routine_command_sender: sender,
        }
    }

    /// Subscribe a client stub. Subscribing an already subscribed stub is harmless.
    pub async fn subscribe(&self, stub: Arc<Stub>) {
        let (sender, receiver) = oneshot::channel();

        let Ok(()) = self
            .routine_command_sender
            .send(RoutineCommand::Subscribe {
                stub,
                confirmation_sender: sender,
            })
            .await
        else {
            return;
        };

        let _ = receiver.await;
    }

    /// Remove a client stub from the topic. Unsubscribing an unsubscribed stub is harmless.
    pub async fn unsubscribe(&self, stub: &Arc<Stub>) {
        let Some(stream_id) = stub.stream_id() else {
            // stub is already dead so we don't subscribe
            return;
        };

        let (sender, receiver) = oneshot::channel();

        let Ok(()) = self
            .routine_command_sender
            .send(RoutineCommand::Unsubscribe {
                stream_id,
                confirmation_sender: sender,
            })
            .await
        else {
            return;
        };

        let _ = receiver.await;
    }
}

impl<Stub> Topic<Stub>
where
    Stub: SubscribableStub,
{
    /// Invoke a callback concurrently for every subscribed stub and collect results.
    pub async fn map<CallbackFuture, FutureOutput>(
        &self,
        mut callback: impl FnMut(Arc<Stub>) -> CallbackFuture,
    ) -> Vec<FutureOutput>
    where
        CallbackFuture: Future<Output = FutureOutput>,
    {
        self.spawn_callback_for_each_stub(&mut callback)
            .await
            .collect()
            .await
    }

    /// Invoke a callback concurrently for every subscribed stub.
    pub async fn for_each<CallbackFuture>(
        &self,
        mut callback: impl FnMut(Arc<Stub>) -> CallbackFuture,
    ) where
        CallbackFuture: Future<Output = ()>,
    {
        let mut tasks = self.spawn_callback_for_each_stub(&mut callback).await;
        while let Some(()) = tasks.next().await {}
    }

    async fn spawn_callback_for_each_stub<CallbackFuture, FutureOutput>(
        &self,
        callback: &mut impl FnMut(Arc<Stub>) -> CallbackFuture,
    ) -> FuturesUnordered<CallbackFuture>
    where
        CallbackFuture: Future<Output = FutureOutput>,
    {
        self.subscribed_stubs
            .read()
            .await
            .iter()
            .cloned()
            .map(callback)
            .collect()
    }
}
