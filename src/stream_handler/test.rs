use std::sync::{Arc, Weak};

use tokio::io::{BufReader, BufWriter};

use crate::{
    HandleRequest, Stub,
    frame::{
        Frame,
        rpc_request::{RpcRequest, RpcResponseMode},
        rpc_response::RpcResponse,
    },
    send_request::SendRequest,
};

use super::{StopReason, StreamId, start_routine};

const MAX_FRAME_SIZE: usize = 1024;

type TestStream = tokio::io::DuplexStream;
type TestHandle = super::Handle<MAX_FRAME_SIZE, TestStream>;

#[derive(Clone)]
struct TestStub;

impl Stub<Weak<TestHandle>> for TestStub {
    fn new(_request_sender: Weak<TestHandle>) -> Self {
        Self
    }
}

struct TestHandler;

impl HandleRequest<TestStub> for TestHandler {
    async fn handle_request(
        &self,
        method_path: &str,
        _stub: TestStub,
        _args: crate::frame::rpc_request::RpcRequestArgReader,
    ) -> crate::frame::rpc_response::RpcResponseBuilder<
        crate::frame::rpc_response::builder::Uninit,
        crate::frame::rpc_response::builder::ResponseInit,
    > {
        assert_eq!(method_path, "echo");
        RpcResponse::builder().response(&"response")
    }
}

fn request(response_mode: RpcResponseMode) -> RpcRequest {
    RpcRequest::builder()
        .method_path("echo")
        .response_mode(response_mode)
        .build()
}

async fn start_test_routine() -> (
    Arc<TestHandle>,
    BufReader<tokio::io::ReadHalf<TestStream>>,
    BufWriter<tokio::io::WriteHalf<TestStream>>,
) {
    let (server_stream, client_stream) = tokio::io::duplex(4096);

    let handle = start_routine::<MAX_FRAME_SIZE, _, _, TestStub>(
        server_stream,
        StreamId::next(),
        Arc::new(TestHandler),
    )
    .await;

    let (client_read, client_write) = tokio::io::split(client_stream);

    (
        handle,
        BufReader::new(client_read),
        BufWriter::new(client_write),
    )
}

#[tokio::test]
async fn incoming_request_gets_handler_response() {
    let (_handle, mut client_reader, mut client_writer) = start_test_routine().await;
    let request = request(RpcResponseMode::ExpectsResponseWithId(7.into()));

    Frame::RpcRequest(request)
        .write_frame::<MAX_FRAME_SIZE>(&mut client_writer)
        .await
        .unwrap();

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame::<MAX_FRAME_SIZE>(&mut client_reader),
    )
    .await
    .unwrap()
    .unwrap();

    match response {
        Frame::RpcResponse(response) => {
            assert_eq!(response.id(), 7);
            assert_eq!(response.get_response::<String>().unwrap(), "response");
        }
        Frame::RpcRequest(_) => panic!("expected response frame"),
    }
}

#[tokio::test]
async fn call_correlates_response_with_request_id() {
    let (handle, mut client_reader, mut client_writer) = start_test_routine().await;
    let call_handle = Arc::clone(&handle);
    let call_task = tokio::spawn(async move {
        call_handle
            .call(crate::frame::rpc_request::RpcRequest::builder().method_path("echo"))
            .await
    });

    let request = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame::<MAX_FRAME_SIZE>(&mut client_reader),
    )
    .await
    .unwrap()
    .unwrap();
    let request_id = match request {
        Frame::RpcRequest(request) => match request.response_mode {
            RpcResponseMode::ExpectsResponseWithId(id) => id,
            RpcResponseMode::NoResponse => panic!("expected response id"),
        },
        Frame::RpcResponse(_) => panic!("expected request frame"),
    };

    let response = RpcResponse::builder()
        .id(request_id)
        .response(&"client response")
        .build();
    Frame::RpcResponse(response)
        .write_frame::<MAX_FRAME_SIZE>(&mut client_writer)
        .await
        .unwrap();

    let response = call_task.await.unwrap().unwrap();
    assert_eq!(
        response.get_response::<String>().unwrap(),
        "client response"
    );
}

#[tokio::test]
async fn notify_sends_request_without_response_id() {
    let (handle, mut client_reader, _client_writer) = start_test_routine().await;

    handle
        .notify(crate::frame::rpc_request::RpcRequest::builder().method_path("echo"))
        .await
        .unwrap();

    let request = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame::<MAX_FRAME_SIZE>(&mut client_reader),
    )
    .await
    .unwrap()
    .unwrap();

    match request {
        Frame::RpcRequest(request) => {
            assert_eq!(request.response_mode, RpcResponseMode::NoResponse);
        }
        Frame::RpcResponse(_) => panic!("expected request frame"),
    }
}

#[tokio::test]
async fn stop_resolves_pending_call() {
    let (handle, mut client_reader, _client_writer) = start_test_routine().await;
    let call_handle = Arc::clone(&handle);
    let call_task = tokio::spawn(async move {
        call_handle
            .call(crate::frame::rpc_request::RpcRequest::builder().method_path("echo"))
            .await
    });

    let _request = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame::<MAX_FRAME_SIZE>(&mut client_reader),
    )
    .await
    .unwrap()
    .unwrap();

    handle.stop().await;

    let error = tokio::time::timeout(std::time::Duration::from_secs(1), call_task)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        error,
        crate::CallError::RoutineIsStopped(StopReason::ManualStop)
    ));
}

#[tokio::test]
async fn peer_disconnect_resolves_pending_call() {
    let (handle, mut client_reader, client_writer) = start_test_routine().await;
    let call_handle = Arc::clone(&handle);
    let call_task = tokio::spawn(async move {
        call_handle
            .call(crate::frame::rpc_request::RpcRequest::builder().method_path("echo"))
            .await
    });

    let _request = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame::<MAX_FRAME_SIZE>(&mut client_reader),
    )
    .await
    .unwrap()
    .unwrap();
    drop(client_reader);
    drop(client_writer);

    let error = tokio::time::timeout(std::time::Duration::from_secs(1), call_task)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert!(matches!(
        error,
        crate::CallError::RoutineIsStopped(StopReason::StreamReadError { .. })
    ));
}
