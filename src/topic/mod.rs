mod topic_id;
pub use topic_id::TopicId;

mod routine;
pub use routine::StubDiedNotificationSender;

use std::{panic, sync::Arc};

use tokio::{
    sync::{RwLock, oneshot},
    task::JoinSet,
};

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
    Stub: Send + Sync + SubscribableStub + 'static,
{
    pub async fn map<Callback, CallbackFuture, FutureOutput>(
        &self,
        callback: Callback,
    ) -> Vec<FutureOutput>
    where
        Callback: Fn(Arc<Stub>) -> CallbackFuture + Send + 'static + Clone,
        CallbackFuture: Future<Output = FutureOutput> + Send,
        FutureOutput: Send + 'static,
    {
        self.spawn_callback_for_each_stub(callback)
            .await
            .join_all()
            .await
    }

    pub async fn for_each<Callback, CallbackFuture>(&self, callback: Callback)
    where
        Callback: Fn(Arc<Stub>) -> CallbackFuture + Send + 'static + Clone,
        CallbackFuture: Future<Output = ()> + Send,
    {
        let mut join_set = self.spawn_callback_for_each_stub(callback).await;

        while let Some(res) = join_set.join_next().await {
            match res {
                Ok(_) => {}
                Err(err) if err.is_panic() => panic::resume_unwind(err.into_panic()),
                Err(err) => panic!("{err}"),
            }
        }
    }

    async fn spawn_callback_for_each_stub<Callback, CallbackFuture, FutureOutput>(
        &self,
        callback: Callback,
    ) -> JoinSet<FutureOutput>
    where
        Callback: Fn(Arc<Stub>) -> CallbackFuture + Send + 'static + Clone,
        CallbackFuture: Future<Output = FutureOutput> + Send,
        FutureOutput: Send + 'static,
    {
        let mut join_set = JoinSet::new();

        let lock = self.subscribed_stubs.read().await;

        for stub in lock.iter().cloned() {
            let callback_clone = callback.clone();
            join_set.spawn(async move { callback_clone(stub).await });
        }

        join_set
    }
}
