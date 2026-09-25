mod handle;
pub use handle::Handle;
pub use handle::StopReason;

mod routine;
use routine::Routine;

mod stream_id;
pub use stream_id::StreamId;

use std::sync::Weak;
use std::{collections::HashMap, sync::Arc};

use tokio::{
    io::{AsyncRead, AsyncWrite, BufWriter},
    sync::{Mutex, mpsc, oneshot},
};

use crate::SubscribableStub;
use crate::Topic;
use crate::{HandleRequest, frame::rpc_response::RpcResponse};

type KillRoutineSender = mpsc::Sender<ShouldSendDisconnectFrame>;
type KillRoutineReceiver = mpsc::Receiver<ShouldSendDisconnectFrame>;

type ResponseSender = oneshot::Sender<Result<RpcResponse, StopReason>>;
type _ResponseReceiver = oneshot::Receiver<Result<RpcResponse, StopReason>>;

pub enum ShouldSendDisconnectFrame {
    Yes,
    No,
}

#[cfg(test)]
mod test;

/// # Cancel safety
/// This method is not cancellation safe. If the method is used as the
/// event in a [`tokio::select!`] statement and some
/// other branch completes first, then some data may be lost.
pub(crate) async fn spawn_client_routine<
    Stream,
    RequestHandler,
    OppositeStubArcHandle,
    OppositeStubWeakHandle,
>(
    max_frame_size: usize,
    stream: Stream,
    stream_id: StreamId,
    request_handler: Arc<RequestHandler>,
) -> OppositeStubArcHandle
where
    Stream: AsyncWrite + AsyncRead + Send + 'static,
    RequestHandler: HandleRequest<OppositeStubWeakHandle>,
    OppositeStubArcHandle: crate::Stub<Arc<Handle<Stream>>>,
    OppositeStubWeakHandle: crate::Stub<Weak<Handle<Stream>>>,
{
    let (read_stream, write_stream) = tokio::io::split(stream);
    let buf_writer = Arc::new(Mutex::new(BufWriter::new(write_stream)));
    let request_map = Arc::new(Mutex::new(HashMap::new()));
    // Using mpsc so that we can emit the kill msg from multiple sources
    let (kill_routine_sender, kill_routine_receiver) = mpsc::channel(1);

    let handle = Arc::new(Handle::<Stream> {
        state: Mutex::new(handle::State::Running(handle::RunningState {
            kill_routine_sender,
            request_map: Arc::clone(&request_map),
            next_request_id: 0,
            buf_writer: Arc::clone(&buf_writer),
            max_frame_size,
        })),
        stream_id,
        registered_topics: Default::default(),
    });

    let handle_weak_ref = Arc::downgrade(&handle);
    tokio::spawn(async move {
        Routine::<Stream, RequestHandler, OppositeStubWeakHandle> {
            stream_id,
            request_handler,
            handle: handle_weak_ref.clone(),
            request_map,
            opposite_stub_weak_handle: Arc::new(OppositeStubWeakHandle::new(handle_weak_ref, None)),
        }
        .routine(
            kill_routine_receiver,
            read_stream,
            buf_writer,
            max_frame_size,
        )
        .await;
    });

    OppositeStubArcHandle::new(handle, None)
}

/// # Cancel safety
/// This method is not cancellation safe. If the method is used as the
/// event in a [`tokio::select!`] statement and some
/// other branch completes first, then some data may be lost.
pub(crate) async fn spawn_server_routine<
    Stream,
    RequestHandler,
    OppositeStubArcHandle,
    OppositeStubWeakHandle,
>(
    max_frame_size: usize,
    stream: Stream,
    stream_id: StreamId,
    request_handler: Arc<RequestHandler>,
    topic_to_disconnect_from_on_routine_death: Weak<Topic<OppositeStubArcHandle>>,
) -> Arc<OppositeStubArcHandle>
where
    Stream: AsyncWrite + AsyncRead + Send + 'static,
    RequestHandler: HandleRequest<OppositeStubWeakHandle>,
    OppositeStubArcHandle: crate::Stub<Arc<Handle<Stream>>> + SubscribableStub,
    OppositeStubWeakHandle: crate::Stub<Weak<Handle<Stream>>>,
{
    let (read_stream, write_stream) = tokio::io::split(stream);
    let buf_writer = Arc::new(Mutex::new(BufWriter::new(write_stream)));
    let request_map = Arc::new(Mutex::new(HashMap::new()));
    // Using mpsc so that we can emit the kill msg from multiple sources
    let (kill_routine_sender, kill_routine_receiver) = mpsc::channel(1);

    let handle = Arc::new(Handle::<Stream> {
        state: Mutex::new(handle::State::Running(handle::RunningState {
            kill_routine_sender,
            request_map: Arc::clone(&request_map),
            next_request_id: 0,
            buf_writer: Arc::clone(&buf_writer),
            max_frame_size,
        })),
        stream_id,
        registered_topics: Default::default(),
    });

    let handle_weak_ref = Arc::downgrade(&handle);
    let opposite_stub_arc_handle = Arc::new(OppositeStubArcHandle::new(handle, None));
    let weak_opposite_stub_arc_handle = Arc::downgrade(&opposite_stub_arc_handle);
    tokio::spawn(async move {
        Routine::<Stream, RequestHandler, OppositeStubWeakHandle> {
            stream_id,
            request_handler,
            handle: handle_weak_ref.clone(),
            request_map,
            opposite_stub_weak_handle: Arc::new(OppositeStubWeakHandle::new(handle_weak_ref, None)),
        }
        .routine(
            kill_routine_receiver,
            read_stream,
            buf_writer,
            max_frame_size,
        )
        .await;

        if let Some(topic) = Weak::upgrade(&topic_to_disconnect_from_on_routine_death)
            && let Some(opposite_stub) = Weak::upgrade(&weak_opposite_stub_arc_handle)
        {
            topic.unsubscribe(&opposite_stub).await
        }
    });

    opposite_stub_arc_handle
}
