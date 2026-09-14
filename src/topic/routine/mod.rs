mod subscribed_stubs;
pub use subscribed_stubs::SubscribedStubs;

mod stub_death_notification_receiver_tasks;
pub use stub_death_notification_receiver_tasks::StubDiedNotificationSender;
use stub_death_notification_receiver_tasks::{
    StubDeathNotificationReceiverTasks, StubDiedNotificationReceiver,
};

use std::sync::{
    Arc,
    atomic::{self, AtomicU64},
};

use tokio::sync::{RwLock, mpsc, oneshot};

use crate::{SubscribableStub, stream_handler::StreamId, topic::TopicId};

pub struct Routine<Stub>
where
    Stub: SubscribableStub,
{
    id: TopicId,
    subscribed_stubs: Arc<RwLock<SubscribedStubs<Stub>>>,
    stub_death_notification_receiver_tasks: StubDeathNotificationReceiverTasks<Stub>,
}

pub type RoutineCommandSender<Stub> = mpsc::Sender<RoutineCommand<Stub>>;
type RoutineCommandWeakSender<Stub> = mpsc::WeakSender<RoutineCommand<Stub>>;
type RoutineCommandReceiver<Stub> = mpsc::Receiver<RoutineCommand<Stub>>;

pub enum RoutineCommand<Stub> {
    Subscribe {
        stub: Stub,
        confirmation_sender: oneshot::Sender<()>,
    },
    Unsubscribe {
        stream_id: StreamId,
        confirmation_sender: oneshot::Sender<()>,
    },
    StubDied(StreamId),
}

impl<Stub> Routine<Stub>
where
    Stub: Send + Sync + SubscribableStub + 'static,
{
    pub async fn start() -> (
        RoutineCommandSender<Stub>,
        Arc<RwLock<SubscribedStubs<Stub>>>,
    ) {
        static NEXT_SUBSCRIBED_STUBS_ID: AtomicU64 = AtomicU64::new(0);
        let id = TopicId(NEXT_SUBSCRIBED_STUBS_ID.fetch_add(1, atomic::Ordering::Relaxed));

        let subscribed_stubs = Arc::new(RwLock::new(SubscribedStubs::new(id)));

        let (sender, receiver) = mpsc::channel(16);

        let sender_weak_ref = sender.downgrade();
        let subscribed_stubs_clone = Arc::clone(&subscribed_stubs);
        tokio::spawn(async move {
            Self {
                id,
                subscribed_stubs: subscribed_stubs_clone,
                stub_death_notification_receiver_tasks: StubDeathNotificationReceiverTasks::new(
                    sender_weak_ref,
                ),
            }
            .routine(receiver)
            .await
        });

        (sender, subscribed_stubs)
    }

    async fn routine(mut self, mut receiver: RoutineCommandReceiver<Stub>) {
        while let Some(command) = receiver.recv().await {
            match command {
                RoutineCommand::Subscribe {
                    stub,
                    confirmation_sender,
                } => {
                    self.handle_stub_subscribed(stub).await;
                    let _ = confirmation_sender.send(());
                }

                RoutineCommand::Unsubscribe {
                    stream_id,
                    confirmation_sender,
                } => {
                    self.handle_stub_unsubscribed(stream_id).await;
                    let _ = confirmation_sender.send(());
                }
                RoutineCommand::StubDied(stream_id) => self.handle_stub_died(stream_id).await,
            }
        }
    }

    async fn handle_stub_subscribed(&mut self, stub: Stub) {
        let Some(stream_id) = stub.stream_id() else {
            return;
        };

        if let Some(stub_died_receiver) =
            self.subscribed_stubs.write().await.add(stub, self.id).await
        {
            self.stub_death_notification_receiver_tasks
                .new_task(stream_id, stub_died_receiver)
        }
    }

    async fn handle_stub_unsubscribed(&mut self, stream_id: StreamId) {
        if self
            .subscribed_stubs
            .write()
            .await
            .remove::<true>(stream_id, self.id)
        {
            self.stub_death_notification_receiver_tasks
                .abort_task(stream_id)
        }
    }

    async fn handle_stub_died(&mut self, stream_id: StreamId) {
        self.subscribed_stubs
            .write()
            .await
            .remove::<false>(stream_id, self.id);
    }
}
