use std::{
    collections::HashMap,
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
    stream_handler::{
        KillRoutineReceiver, ResponseSender, ShouldSendDisconnectFrame, stream_id::StreamId,
    },
};

pub(super) struct Routine<Stream, RequestHandler, WeakOppositeStub> {
    pub stream_id: StreamId,
    pub request_handler: Arc<RequestHandler>,
    pub request_map: Arc<Mutex<HashMap<RpcRequestId, ResponseSender>>>,
    pub handle: Weak<super::Handle<Stream>>,
    pub opposite_stub_weak_handle: Arc<WeakOppositeStub>,
}

impl<Stream, RequestHandler, WeakOppositeStub> Routine<Stream, RequestHandler, WeakOppositeStub>
where
    Stream: AsyncWrite + AsyncRead + Send + 'static,
    RequestHandler: HandleRequest<WeakOppositeStub>,
    WeakOppositeStub: crate::Stub<Weak<super::Handle<Stream>>>,
{
    pub async fn routine(
        self,
        mut kill_routine_receiver: KillRoutineReceiver,
        read_stream: ReadHalf<Stream>,
        buf_writer: Arc<Mutex<BufWriter<WriteHalf<Stream>>>>,
        max_frame_size: usize,
    ) where
        Stream: AsyncWrite + AsyncRead + Send + 'static,
    {
        let should_send_disconnect_msg = tokio::select! {
            should_send_disconnect_msg = kill_routine_receiver.recv() => {
                should_send_disconnect_msg
                    .expect(
                        "stream_handler::Handle holds a sender and uses it on Drop, so this can \
                         never be None"
                    )
            }

            // frame_handler_loop() is not cancel safe, but it doesn't matter as the only thing
            // canceling it corrupts is the read buf which we won't use anymore anyway if we kill
            // the routine
            () = self.frame_handler_loop(
                BufReader::new(read_stream),
                Arc::clone(&buf_writer),
                max_frame_size,
            ) => {
                // frame_handler_loop only returns on error, we don't send the disconnect frame on
                // error
                ShouldSendDisconnectFrame::No
            }
        };

        // TODO think about what to do with requests currently being handled
        // For now we will let them continue
        if let ShouldSendDisconnectFrame::Yes = should_send_disconnect_msg {
            // TODO log error
            let _ = Frame::Disconnected
                .write_frame(&mut *buf_writer.lock().await, max_frame_size)
                .await;
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
        max_frame_size: usize,
    ) where
        Stream: AsyncWrite + AsyncRead + Send + 'static,
    {
        loop {
            let frame = match Frame::read_frame(&mut buf_reader, max_frame_size).await {
                Ok(frame) => frame,
                Err(err) => {
                    eprintln!(
                        "Stream {}: Error reading frame, stopping handler loop: {err}",
                        self.stream_id
                    );

                    if let Some(handle) = self.handle.upgrade() {
                        handle.stop_with(err.into()).await;
                    };

                    // Something went wrong with the stream so sending a disconnect message would
                    // fail.
                    break;
                }
            };

            println!("Stream {}: Received new frame", self.stream_id);

            match frame {
                Frame::Disconnected => {
                    // Disconnection was initiated by peer, so no need to send our own message.
                    break;
                }
                Frame::RpcRequest(request) => {
                    let request_handler_clone = Arc::clone(&self.request_handler);
                    let handle_clone = self.handle.clone();
                    let stub_clone = self.opposite_stub_weak_handle.clone();
                    let buf_writer_clone = Arc::clone(&buf_writer);
                    tokio::spawn(async move {
                        Self::handle_rpc_request(
                            self.stream_id,
                            request_handler_clone,
                            request,
                            handle_clone,
                            stub_clone,
                            buf_writer_clone,
                            max_frame_size,
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
        handle: Weak<super::Handle<Stream>>,
        weak_opposite_stub: Arc<WeakOppositeStub>,
        buf_writer: Arc<Mutex<BufWriter<WriteHalf<Stream>>>>,
        max_frame_size: usize,
    ) where
        Stream: AsyncWrite,
    {
        let request_arg_reader = RpcRequestArgReader::from(request.args);

        let response_builder = request_handler
            .handle_request(
                &request.method_path,
                &weak_opposite_stub,
                request_arg_reader,
            )
            .await;

        let rpc_request_id = match request.response_mode {
            RpcResponseMode::NoResponse => return,
            RpcResponseMode::ExpectsResponseWithId(rpc_request_id) => rpc_request_id,
        };

        let response = response_builder.id(rpc_request_id).build();

        let write_result = Frame::RpcResponse(response)
            .write_frame(buf_writer.lock().await.deref_mut(), max_frame_size)
            .await;

        match write_result {
            Ok(()) => {}
            Err(err) => {
                eprintln!("Stream {stream_id}: Error writing frame, stopping handler loop: {err}");
                let Some(handle) = handle.upgrade() else {
                    return;
                };

                handle.stop_with(err.into()).await;
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
