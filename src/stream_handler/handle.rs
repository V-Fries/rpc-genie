use std::{
    assert_matches,
    collections::{HashMap, hash_map},
    mem,
    ops::Deref,
    sync::{Arc, Mutex as StdMutex, Weak},
};

use tokio::{
    io::{AsyncWrite, BufWriter, WriteHalf},
    sync::{Mutex as TokioMutex, oneshot},
};

use crate::{
    frame::{
        self, Frame, RpcRequestId,
        rpc_request::{RpcRequest, RpcRequestBuilder, RpcRequestBuilderUninit, RpcResponseMode},
        rpc_response::RpcResponse,
    },
    send_request::SendRequest,
    stream_handler::{KillRoutineSender, ResponseSender, ShouldSendDisconnectFrame, StreamId},
    topic::{self, StubDiedNotificationSender, TopicId},
};

pub struct Handle<const MAX_FRAME_SIZE: usize, Stream> {
    pub(super) state: TokioMutex<State<MAX_FRAME_SIZE, Stream>>,
    pub(super) stream_id: StreamId,
    pub(super) registered_topics: StdMutex<RegisteredTopics>,
}

pub(super) enum State<const MAX_FRAME_SIZE: usize, Stream> {
    Running(RunningState<MAX_FRAME_SIZE, Stream>),
    Stopped(StopReason),
}

pub(super) struct RunningState<const MAX_FRAME_SIZE: usize, Stream> {
    pub(super) kill_routine_sender: KillRoutineSender,
    pub(super) request_map: Arc<TokioMutex<HashMap<RpcRequestId, ResponseSender>>>,
    pub(super) next_request_id: u64,
    pub(super) buf_writer: Arc<TokioMutex<BufWriter<WriteHalf<Stream>>>>,
}

#[derive(Default)]
pub(super) struct RegisteredTopics {
    topics: Vec<RegisteredTopic>,
    positions: HashMap<TopicId, usize>,
}

struct RegisteredTopic {
    id: TopicId,
    stub_died_notification_sender: topic::StubDiedNotificationSender,
}

impl RegisteredTopics {
    fn add(
        &mut self,
        topic_id: TopicId,
        stub_died_notification_sender: topic::StubDiedNotificationSender,
    ) {
        match self.positions.entry(topic_id) {
            hash_map::Entry::Occupied(_) => {}
            hash_map::Entry::Vacant(entry) => {
                entry.insert(self.topics.len());

                self.topics.push(RegisteredTopic {
                    id: topic_id,
                    stub_died_notification_sender,
                });
            }
        }
    }

    fn remove(&mut self, topic_id: TopicId) {
        let Some(index) = self.positions.remove(&topic_id) else {
            return;
        };

        let last_index = self.topics.len() - 1;
        let _removed_topic = self.topics.swap_remove(index);

        if index != last_index {
            let moved_stub_id = self.topics[index].id;

            assert_matches!(
                self.positions.insert(moved_stub_id, index),
                Some(overwritten_index) if overwritten_index == last_index,
            );
        }
    }

    fn send_death_notifications(self) {
        for topic in self.topics.into_iter() {
            let _ = topic.stub_died_notification_sender.send(());
        }
    }
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, thiserror::Error)]
pub enum StopReason {
    #[error("The routine was stopped manually")]
    ManualStop,
    #[error("Error while reading from stream")]
    StreamReadError(#[from] frame::ReadError),
    #[error("Error while writing to stream")]
    StreamWriteError(#[from] frame::WriteError),
    #[error("The handle was already dropped")]
    HandleWasDropped,
}

impl<const MAX_FRAME_SIZE: usize, Stream> Handle<MAX_FRAME_SIZE, Stream> {
    #[allow(dead_code)]
    pub async fn stop(&self) {
        self.stop_with(StopReason::ManualStop).await
    }

    pub(super) async fn stop_with(&self, stop_reason: StopReason) {
        self.state.lock().await.stop_with(stop_reason).await;
        self.send_death_notifications_to_topics();
    }

    fn send_death_notifications_to_topics(&self) {
        let mut registered_topics_lock = self
            .registered_topics
            .lock()
            .unwrap_or_else(|poison_error| poison_error.into_inner());

        mem::take(&mut *registered_topics_lock).send_death_notifications();
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> Drop for Handle<MAX_FRAME_SIZE, Stream> {
    fn drop(&mut self) {
        self.send_death_notifications_to_topics();
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> Drop for RunningState<MAX_FRAME_SIZE, Stream> {
    fn drop(&mut self) {
        // Ignore result as the routine could end before we drop the handles. ie.e the receiver is
        // dropped
        let _ = self
            .kill_routine_sender
            .try_send(ShouldSendDisconnectFrame::Yes);
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> SendRequest for Arc<Handle<MAX_FRAME_SIZE, Stream>>
where
    Stream: AsyncWrite + Send + 'static,
{
    async fn call(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<RpcResponse, crate::CallError> {
        self.deref().call(request_builder).await
    }

    async fn notify(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<(), crate::NotifyError> {
        self.deref().notify(request_builder).await
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> SendRequest for Weak<Handle<MAX_FRAME_SIZE, Stream>>
where
    Stream: AsyncWrite + Send + 'static,
{
    async fn call(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<RpcResponse, crate::CallError> {
        let Some(handle) = self.upgrade() else {
            return Err(StopReason::HandleWasDropped.into());
        };

        handle.deref().call(request_builder).await
    }

    async fn notify(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<(), crate::NotifyError> {
        let Some(handle) = self.upgrade() else {
            return Err(StopReason::HandleWasDropped.into());
        };

        handle.deref().notify(request_builder).await
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> Handle<MAX_FRAME_SIZE, Stream>
where
    Stream: AsyncWrite + Send + 'static,
{
    /// # Cancel safety
    /// This method is not cancellation safe. If the method is used as the
    /// event in a [`tokio::select!`] statement and some other branch completes first a frame could
    /// be partially written to the stream which will corrupt the stream.
    /// It could also lead to memory leakage (until the Handle is dropped)
    /// If the frame was fully written, the request will not be cancelled on the remote device.
    async fn call(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<RpcResponse, crate::CallError> {
        let (response_sender, response_receiver) = oneshot::channel();

        let mut lock = self.state.lock().await;

        let running_state = lock
            .running_state()
            .map_err(|stop_reason| stop_reason.clone())?;

        let request_id = running_state
            .register_new_call_request(response_sender)
            .await;

        let request = request_builder
            .response_mode(RpcResponseMode::ExpectsResponseWithId(request_id))
            .build();

        match running_state.write_frame(request).await {
            Ok(()) => {}
            Err(write_error) => {
                lock.stop_with(write_error.clone().into()).await;
                return Err(write_error.into());
            }
        }

        drop(lock);

        // TODO implement request time out

        response_receiver.await.expect(
            "stop_with() method sends a message to the receiver before dropping it, so this should \
             never fail",
        ).map_err(Into::into)
    }

    /// # Cancel safety
    /// This method is not cancellation safe. If the method is used as the
    /// event in a [`tokio::select!`] statement and some other branch completes first a frame could
    /// be partially written to the stream which will corrupt the stream.
    /// If the frame was fully written, the request will not be cancelled on the remote device.
    async fn notify(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<(), crate::NotifyError> {
        let request = request_builder
            .response_mode(RpcResponseMode::NoResponse)
            .build();

        let mut lock = self.state.lock().await;

        let running_state = lock
            .running_state()
            .map_err(|stop_reason| stop_reason.clone())?;

        match running_state.write_frame(request).await {
            Ok(()) => Ok(()),
            Err(write_error) => {
                lock.stop_with(write_error.clone().into()).await;
                Err(write_error.into())
            }
        }
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> State<MAX_FRAME_SIZE, Stream> {
    fn running_state(&mut self) -> Result<&mut RunningState<MAX_FRAME_SIZE, Stream>, &StopReason> {
        match self {
            State::Running(state) => Ok(state),
            State::Stopped(stop_reason) => Err(stop_reason),
        }
    }

    // TODO remove allow(dead_code)
    #[allow(dead_code)]
    async fn stop(&mut self) {
        self.stop_with(StopReason::ManualStop).await
    }

    async fn stop_with(&mut self, stop_reason: StopReason) {
        match self {
            State::Running(state) => {
                // Don't check for error as the receiver could already be dropped (this is expected
                // in some cases, e.g. stop_with() was called by the routine's task after
                // the routine ended on it's own)
                match stop_reason {
                    StopReason::ManualStop | StopReason::HandleWasDropped => {
                        let _ = state
                            .kill_routine_sender
                            .send(ShouldSendDisconnectFrame::Yes)
                            .await;
                    }
                    StopReason::StreamReadError(_) | StopReason::StreamWriteError(_) => {
                        let _ = state
                            .kill_routine_sender
                            .send(ShouldSendDisconnectFrame::No)
                            .await;
                    }
                }

                for sender in mem::take(&mut *state.request_map.lock().await).into_values() {
                    let _ = sender.send(Err(stop_reason.clone()));
                }

                *self = State::Stopped(stop_reason);
            }
            State::Stopped(_) => {
                // We always want to keep the first StopReason as a new StopReason might be sent
                // only because of the previous error / manual stop
            }
        }
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> RunningState<MAX_FRAME_SIZE, Stream>
where
    Stream: AsyncWrite + Send + 'static,
{
    async fn register_new_call_request(&mut self, sender: ResponseSender) -> RpcRequestId {
        loop {
            let next_id = self.next_request_id.into();
            self.next_request_id = self.next_request_id.wrapping_add(1);

            match self.request_map.lock().await.entry(next_id) {
                std::collections::hash_map::Entry::Vacant(entry) => {
                    entry.insert(sender);
                    return next_id;
                }
                std::collections::hash_map::Entry::Occupied(_) => {
                    // The ID wrapped around while the previous request was still
                    // pending.
                    // Will try with the next ID next loop.
                    // This works since pending requests eventually time out or receive a response.
                }
            }
        }
    }

    async fn write_frame(&self, request: RpcRequest) -> Result<(), frame::WriteError> {
        Frame::RpcRequest(request)
            .write_frame::<MAX_FRAME_SIZE>(&mut *self.buf_writer.lock().await)
            .await
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> crate::SubscribableStub
    for Arc<Handle<MAX_FRAME_SIZE, Stream>>
where
    Stream: Send,
{
    async fn add_registered_topic(
        &self,
        topic_id: topic::TopicId,
        stub_died_notification_sender: topic::StubDiedNotificationSender,
    ) -> bool {
        self.deref()
            .add_registered_topic(topic_id, stub_died_notification_sender)
            .await
    }

    fn remove_registered_topic(&self, topic_id: topic::TopicId) {
        self.deref().remove_registered_topic(topic_id)
    }

    fn stream_id(&self) -> Option<StreamId> {
        self.deref().stream_id()
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> crate::SubscribableStub
    for Weak<Handle<MAX_FRAME_SIZE, Stream>>
where
    Stream: Send,
{
    async fn add_registered_topic(
        &self,
        topic_id: topic::TopicId,
        stub_died_notification_sender: topic::StubDiedNotificationSender,
    ) -> bool {
        let Some(handle) = self.upgrade() else {
            return false;
        };

        handle
            .deref()
            .add_registered_topic(topic_id, stub_died_notification_sender)
            .await
    }

    fn remove_registered_topic(&self, topic_id: topic::TopicId) {
        let Some(handle) = self.upgrade() else {
            return;
        };

        handle.deref().remove_registered_topic(topic_id)
    }

    fn stream_id(&self) -> Option<StreamId> {
        self.upgrade()?.deref().stream_id()
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> crate::SubscribableStub for Handle<MAX_FRAME_SIZE, Stream>
where
    Stream: Send,
{
    async fn add_registered_topic(
        &self,
        topic_id: TopicId,
        stub_died_notification_sender: StubDiedNotificationSender,
    ) -> bool {
        let mut lock = self.state.lock().await;

        let Ok(_) = lock.running_state() else {
            return false;
        };

        // Don't drop(lock) yet, we want to finish the add below first

        // MAKE SURE NOT TO AWAIT ANYTHING WHILE THIS MUTEX IS HELD
        self.registered_topics
            .lock()
            .unwrap_or_else(|poison_error| poison_error.into_inner())
            .add(topic_id, stub_died_notification_sender);

        true
    }

    fn remove_registered_topic(&self, topic_id: TopicId) {
        // MAKE SURE NOT TO AWAIT ANYTHING WHILE THIS MUTEX IS HELD
        self.registered_topics
            .lock()
            .unwrap_or_else(|poison_error| poison_error.into_inner())
            .remove(topic_id)
    }

    fn stream_id(&self) -> Option<StreamId> {
        Some(self.stream_id)
    }
}
