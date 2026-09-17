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
    pub async fn new() -> Self {
        let (sender, subscribed_stubs) = Routine::spawn().await;

        Self {
            subscribed_stubs,
            routine_command_sender: sender,
        }
    }

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
