use std::{
    collections::HashMap,
    mem,
    ops::Deref,
    sync::{Arc, Weak},
};

use tokio::{
    io::{AsyncWrite, BufWriter, WriteHalf},
    sync::{Mutex, MutexGuard, oneshot},
};

use crate::{
    frame::{
        self, Frame, RpcRequestId,
        rpc_request::{RpcRequest, RpcRequestBuilder, RpcRequestBuilderUninit, RpcResponseMode},
        rpc_response::RpcResponse,
    },
    send_request::SendRequest,
    stream_handler::{KillRoutineSender, ResponseSender},
};

pub struct Handle<const MAX_FRAME_SIZE: usize, Stream>(
    pub(super) Mutex<State<MAX_FRAME_SIZE, Stream>>,
);

pub(super) enum State<const MAX_FRAME_SIZE: usize, Stream> {
    Running(RunningState<MAX_FRAME_SIZE, Stream>),
    Stopped(StopReason),
}

pub(super) struct RunningState<const MAX_FRAME_SIZE: usize, Stream> {
    pub(super) kill_routine_sender: KillRoutineSender,
    pub(super) request_map: Arc<Mutex<HashMap<RpcRequestId, ResponseSender>>>,
    pub(super) next_request_id: u64,
    pub(super) buf_writer: Arc<Mutex<BufWriter<WriteHalf<Stream>>>>,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug, thiserror::Error)]
pub enum StopReason {
    #[error("The routine was stopped manually")]
    ManualStop,
    #[error("Error while reading from stream: {details}")]
    StreamReadError { details: String },
    #[error("Error while writing to stream: {details}")]
    StreamWriteError { details: String },
    #[error("The handle was already dropped")]
    HandleWasDropped,
}

impl<const MAX_FRAME_SIZE: usize, Stream> Handle<MAX_FRAME_SIZE, Stream> {
    pub(super) async fn lock(&self) -> MutexGuard<'_, State<MAX_FRAME_SIZE, Stream>> {
        self.0.lock().await
    }
}

impl<const MAX_FRAME_SIZE: usize, Stream> Drop for RunningState<MAX_FRAME_SIZE, Stream> {
    fn drop(&mut self) {
        let _ = self.kill_routine_sender.try_send(());
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
            return Err(crate::CallError::RoutineIsStopped(
                StopReason::HandleWasDropped,
            ));
        };

        handle.deref().call(request_builder).await
    }

    async fn notify(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<(), crate::NotifyError> {
        let Some(handle) = self.upgrade() else {
            return Err(crate::NotifyError::RoutineIsStopped(
                StopReason::HandleWasDropped,
            ));
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

        let mut lock = self.lock().await;

        let running_state = lock
            .running_state()
            .map_err(|stop_reason| crate::CallError::RoutineIsStopped(stop_reason.clone()))?;

        let request_id = running_state
            .register_new_call_request(response_sender)
            .await;

        let request = request_builder
            .response_mode(RpcResponseMode::ExpectsResponseWithId(request_id))
            .build();

        match running_state.write_frame(request).await {
            Ok(()) => {}
            Err(write_error) => {
                lock.stop_with(StopReason::StreamWriteError {
                    details: write_error.to_string(),
                })
                .await;
                return Err(crate::CallError::FailedToWriteFrame(write_error));
            }
        }

        drop(lock);

        // TODO implement request time out

        response_receiver.await.expect(
            "stop_with() method sends a message to the receiver before dropping it, so this should \
             never fail",
        ).map_err(crate::CallError::RoutineIsStopped)
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

        let mut lock = self.lock().await;

        let running_state = lock
            .running_state()
            .map_err(|stop_reason| crate::NotifyError::RoutineIsStopped(stop_reason.clone()))?;

        match running_state.write_frame(request).await {
            Ok(()) => Ok(()),
            Err(write_error) => {
                lock.stop_with(StopReason::StreamWriteError {
                    details: write_error.to_string(),
                })
                .await;
                Err(crate::NotifyError::FailedToWriteFrame(write_error))
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
    pub async fn stop(&mut self) {
        self.stop_with(StopReason::ManualStop).await
    }

    pub(super) async fn stop_with(&mut self, stop_reason: StopReason) {
        match self {
            State::Running(state) => {
                // Don't check for error as the receiver could already be dropped (this is expected
                // in some cases, e.g. stop_with() was called by the routine's task after
                // the routine ended on it's own)
                let _ = state.kill_routine_sender.send(()).await;

                let mut request_map = HashMap::new();
                mem::swap(&mut request_map, &mut *state.request_map.lock().await);

                for sender in request_map.into_values() {
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
