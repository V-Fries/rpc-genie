mod subscribed_stubs;
pub use subscribed_stubs::SubscribedStubs;

use std::sync::Arc;

use tokio::sync::{RwLock, mpsc, oneshot};

use crate::{SubscribableStub, stream_handler::StreamId, topic::TopicId};

pub(super) struct Routine<Stub>
where
    Stub: SubscribableStub,
{
    id: TopicId,
    subscribed_stubs: Arc<RwLock<SubscribedStubs<Stub>>>,
}

pub(super) type RoutineCommandSender<Stub> = mpsc::Sender<RoutineCommand<Stub>>;
type RoutineCommandReceiver<Stub> = mpsc::Receiver<RoutineCommand<Stub>>;

type StubDiedNotificationOwnedSender = mpsc::UnboundedSender<StreamId>;
#[doc(hidden)]
pub type StubDiedNotificationSender = mpsc::WeakUnboundedSender<StreamId>;
type StubDiedNotificationReceiver = mpsc::UnboundedReceiver<StreamId>;

pub(super) enum RoutineCommand<Stub> {
    Subscribe {
        stub: Arc<Stub>,
        confirmation_sender: oneshot::Sender<()>,
    },
    Unsubscribe {
        stream_id: StreamId,
        confirmation_sender: oneshot::Sender<()>,
    },
}

impl<Stub> Routine<Stub>
where
    Stub: Send + Sync + SubscribableStub + 'static,
{
    pub(super) async fn spawn() -> (
        RoutineCommandSender<Stub>,
        Arc<RwLock<SubscribedStubs<Stub>>>,
    ) {
        let id = TopicId::next();

        let subscribed_stubs = Arc::new(RwLock::new(SubscribedStubs::new(id)));

        let (command_sender, command_receiver) = mpsc::channel(16);

        // Using unbounded channel because we need to be able to send message without awaiting (in
        // stream_handler::drop)
        let (stub_died_sender, stub_died_receiver) = mpsc::unbounded_channel();

        let subscribed_stubs_clone = Arc::clone(&subscribed_stubs);
        tokio::spawn(async move {
            Self {
                id,
                subscribed_stubs: subscribed_stubs_clone,
            }
            .routine(command_receiver, stub_died_sender, stub_died_receiver)
            .await
        });

        (command_sender, subscribed_stubs)
    }

    async fn routine(
        mut self,
        mut command_receiver: RoutineCommandReceiver<Stub>,
        stub_died_sender: StubDiedNotificationOwnedSender,
        mut stub_died_receiver: StubDiedNotificationReceiver,
    ) {
        loop {
            tokio::select! {
                maybe_command = command_receiver.recv() => {
                    let Some(command) = maybe_command else {
                        break;
                    };

                    self.handle_command(command, &stub_died_sender).await
                }

                maybe_stream_id = stub_died_receiver.recv() => {
                    let stream_id = maybe_stream_id.expect(
                        "Topic routine owns stub_died_sender so this can never be None",
                    );

                     self.handle_stub_died(stream_id).await
                }
            }
        }
    }

    async fn handle_command(
        &mut self,
        command: RoutineCommand<Stub>,
        stub_died_sender: &StubDiedNotificationOwnedSender,
    ) {
        match command {
            RoutineCommand::Subscribe {
                stub,
                confirmation_sender,
            } => {
                self.handle_stub_subscribed(stub, stub_died_sender.downgrade())
                    .await;
                let _ = confirmation_sender.send(());
            }

            RoutineCommand::Unsubscribe {
                stream_id,
                confirmation_sender,
            } => {
                self.handle_stub_unsubscribed(stream_id).await;
                let _ = confirmation_sender.send(());
            }
        }
    }

    async fn handle_stub_subscribed(
        &mut self,
        stub: Arc<Stub>,
        stub_died_sender: StubDiedNotificationSender,
    ) {
        self.subscribed_stubs
            .write()
            .await
            .add(stub, self.id, stub_died_sender)
            .await
    }

    async fn handle_stub_unsubscribed(&mut self, stream_id: StreamId) {
        self.subscribed_stubs
            .write()
            .await
            .remove::<true>(stream_id, self.id);
    }

    async fn handle_stub_died(&mut self, stream_id: StreamId) {
        self.subscribed_stubs
            .write()
            .await
            .remove::<false>(stream_id, self.id);
    }
}
