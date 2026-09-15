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

use crate::{HandleRequest, frame::rpc_response::RpcResponse};

type KillRoutineSender = mpsc::Sender<()>;
type KillRoutineReceiver = mpsc::Receiver<()>;

type ResponseSender = oneshot::Sender<Result<RpcResponse, StopReason>>;
type _ResponseReceiver = oneshot::Receiver<Result<RpcResponse, StopReason>>;

#[cfg(test)]
mod test;

/// # Cancel safety
/// This method is not cancellation safe. If the method is used as the
/// event in a [`tokio::select!`] statement and some
/// other branch completes first, then some data may be lost.
// TODO remove allow dead_code
#[allow(dead_code)]
pub(crate) async fn spawn_routine<
    const MAX_FRAME_SIZE: usize,
    Stream,
    RequestHandler,
    WeakOppositeStub,
>(
    stream: Stream,
    stream_id: StreamId,
    request_handler: Arc<RequestHandler>,
) -> Arc<Handle<MAX_FRAME_SIZE, Stream>>
where
    Stream: AsyncWrite + AsyncRead + Send + 'static,
    RequestHandler: HandleRequest<WeakOppositeStub>,
    WeakOppositeStub: crate::Stub<Weak<Handle<MAX_FRAME_SIZE, Stream>>>,
{
    let (read_stream, write_stream) = tokio::io::split(stream);
    let buf_writer = Arc::new(Mutex::new(BufWriter::new(write_stream)));
    let request_map = Arc::new(Mutex::new(HashMap::new()));
    // Using mpsc so that we can emit the kill msg from multiple sources
    let (kill_routine_sender, kill_routine_receiver) = mpsc::channel::<()>(1);

    let handle = Arc::new(Handle::<MAX_FRAME_SIZE, Stream> {
        state: Mutex::new(handle::State::Running(handle::RunningState {
            kill_routine_sender,
            request_map: Arc::clone(&request_map),
            next_request_id: 0,
            buf_writer: Arc::clone(&buf_writer),
        })),
        stream_id,
        registered_topics: Default::default(),
    });

    let handle_weak_ref = Arc::downgrade(&handle);
    tokio::spawn(async move {
        Routine::<MAX_FRAME_SIZE, Stream, RequestHandler, WeakOppositeStub> {
            stream_id,
            request_handler,
            handle: handle_weak_ref.clone(),
            request_map,
            weak_opposite_stub: Arc::new(WeakOppositeStub::new(handle_weak_ref, None)),
        }
        .routine(kill_routine_receiver, read_stream, buf_writer)
        .await
    });

    handle
}
