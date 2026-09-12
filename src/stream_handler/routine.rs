use std::{
    collections::HashMap,
    marker::PhantomData,
    ops::DerefMut,
    sync::{Arc, Weak},
};

use tokio::{
    io::{AsyncRead, AsyncWrite, BufReader, BufWriter, ReadHalf, WriteHalf},
    sync::Mutex,
};

use crate::{
    HandleRequest,
    frame::{
        Frame, RpcRequestId,
        rpc_request::{RpcRequest, RpcRequestArgReader, RpcResponseMode},
        rpc_response::RpcResponse,
    },
    stream_handler::{self, KillRoutineReceiver, ResponseSender, StopReason, stream_id::StreamId},
};

// TODO remove allow dead_code
#[allow(dead_code)]
pub(super) struct Routine<const MAX_FRAME_SIZE: usize, RequestHandler, Stub, Stream> {
    pub stream_id: StreamId,
    pub request_handler: Arc<RequestHandler>,
    pub request_map: Arc<Mutex<HashMap<RpcRequestId, ResponseSender>>>,
    pub handle: Weak<super::Handle<MAX_FRAME_SIZE, Stream>>,
    pub _stream: PhantomData<fn() -> Stub>,
}

impl<const MAX_FRAME_SIZE: usize, RequestHandler, Stub, Stream>
    Routine<MAX_FRAME_SIZE, RequestHandler, Stub, Stream>
where
    RequestHandler: HandleRequest<Stub>,
    Stream: AsyncWrite + AsyncRead + Send + 'static,
    Stub: crate::Stub<Weak<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>>,
{
    pub async fn routine(
        self,
        mut kill_routine_receiver: KillRoutineReceiver,
        read_stream: ReadHalf<Stream>,
        buf_writer: Arc<Mutex<BufWriter<WriteHalf<Stream>>>>,
    ) where
        Stream: AsyncWrite + AsyncRead + Send + 'static,
    {
        tokio::select! {
            _ = kill_routine_receiver.recv() => {}

            _ = self.frame_handler_loop(
                BufReader::new(read_stream),
                buf_writer,
            ) => {}
        }
    }

    /// # Cancel safety
    /// This method is not cancellation safe. If the method is used as the
    /// event in a [`tokio::select!`] statement and some
    /// other branch completes first, then some data may be lost.
    async fn frame_handler_loop(
        self,
        mut buf_reader: BufReader<ReadHalf<Stream>>,
        buf_writer: Arc<Mutex<BufWriter<WriteHalf<Stream>>>>,
    ) where
        Stream: AsyncWrite + AsyncRead + Send + 'static,
    {
        loop {
            let frame = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut buf_reader).await {
                Ok(frame) => frame,
                Err(err) => {
                    eprintln!(
                        "Stream {}: Error reading frame, stopping handler loop: {err}",
                        self.stream_id
                    );

                    let Some(handle) = self.handle.upgrade() else {
                        break;
                    };

                    handle
                        .lock()
                        .await
                        .stop_with(StopReason::StreamReadError {
                            details: err.to_string(),
                        })
                        .await;
                    break;
                }
            };

            println!("Stream {}: Received new frame", self.stream_id);

            match frame {
                Frame::RpcRequest(request) => {
                    let request_handler_clone = Arc::clone(&self.request_handler);
                    let handle_clone = self.handle.clone();
                    let buf_writer_clone = Arc::clone(&buf_writer);
                    tokio::spawn(async move {
                        Self::handle_rpc_request(
                            self.stream_id,
                            request_handler_clone,
                            request,
                            handle_clone,
                            buf_writer_clone,
                        )
                        .await
                    });
                }
                Frame::RpcResponse(response) => self.handle_rpc_response(response).await,
            }
        }
    }

    async fn handle_rpc_request(
        stream_id: StreamId,
        request_handler: Arc<RequestHandler>,
        request: RpcRequest,
        handle: Weak<super::Handle<MAX_FRAME_SIZE, Stream>>,
        buf_writer: Arc<Mutex<BufWriter<WriteHalf<Stream>>>>,
    ) where
        Stream: AsyncWrite,
    {
        let request_arg_reader = RpcRequestArgReader::from(request.args);

        let response_builder = request_handler
            .handle_request(
                &request.method_path,
                Stub::new(handle.clone()),
                request_arg_reader,
            )
            .await;

        let rpc_request_id = match request.response_mode {
            RpcResponseMode::NoResponse => return,
            RpcResponseMode::ExpectsResponseWithId(rpc_request_id) => rpc_request_id,
        };

        let response = response_builder.id(rpc_request_id).build();

        let write_result = Frame::RpcResponse(response)
            .write_frame::<MAX_FRAME_SIZE>(buf_writer.lock().await.deref_mut())
            .await;

        match write_result {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Stream {stream_id}: Error writing frame, stopping handler loop: {err}");
                let Some(handle) = handle.upgrade() else {
                    return;
                };

                handle
                    .lock()
                    .await
                    .stop_with(StopReason::StreamWriteError {
                        details: err.to_string(),
                    })
                    .await;
            }
        }
    }

    async fn handle_rpc_response(&self, response: RpcResponse) {
        let Some(sender) = self.request_map.lock().await.remove(&response.id()) else {
            eprintln!(
                "Stream {}: Received a response with an invalid id({})",
                self.stream_id,
                response.id()
            );
            // TODO maybe send an error message to the other end of the stream
            return;
        };

        match sender.send(Ok(response)) {
            Ok(()) => {}
            Err(_) => {
                eprintln!(
                    "Stream {}: Failed to send response through timer, (maybe the request \
                     timed-out)",
                    self.stream_id
                )
            }
        }
    }
}
