use std::{assert_matches, sync::Arc, time::Duration};

use tokio::{
    io::{BufReader, BufWriter},
    time::timeout,
};

use crate::{
    frame::{Frame, rpc_request::RpcResponseMode, rpc_response::RpcResponse},
    send_request::SendRequest,
};

use super::*;

async fn start_test_routine(
    topic: Weak<Topic<TestStubArcHandle>>,
) -> (
    Arc<TestStubArcHandle>,
    BufReader<tokio::io::ReadHalf<TestStream>>,
    BufWriter<tokio::io::WriteHalf<TestStream>>,
) {
    let (server_stream, client_stream) = tokio::io::duplex(4096);

    let stub = spawn_server_routine::<_, _, TestStubArcHandle, TestStubWeakHandle>(
        MAX_FRAME_SIZE,
        server_stream,
        StreamId::next(),
        Arc::new(TestHandler),
        topic,
    )
    .await;

    let (client_read, client_write) = tokio::io::split(client_stream);

    (
        stub,
        BufReader::new(client_read),
        BufWriter::new(client_write),
    )
}

#[tokio::test]
async fn incoming_request_gets_handler_response() {
    let topic = Arc::new(Topic::new().await);
    let (stub, mut client_reader, mut client_writer) =
        start_test_routine(Arc::downgrade(&topic)).await;

    topic.subscribe(Arc::clone(&stub)).await;
    assert_eq!(topic.map(async |_| {}).await.len(), 1);

    let request = request(RpcResponseMode::ExpectsResponseWithId(7.into()));

    Frame::RpcRequest(request)
        .write_frame(&mut client_writer, MAX_FRAME_SIZE)
        .await
        .unwrap();

    let response = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame(&mut client_reader, MAX_FRAME_SIZE),
    )
    .await
    .unwrap()
    .unwrap();

    match response {
        Frame::RpcResponse(response) => {
            assert_eq!(response.id(), 7);
            assert_eq!(response.get_response::<String>().unwrap(), "response");
        }
        _ => panic!("expected response frame"),
    }

    stub.stop().await;
    timeout(Duration::from_secs(1), async {
        loop {
            if topic.map(async |_| {}).await.len() == 0 {
                break;
            }
        }
    })
    .await
    .expect("client was not removed from topic");
}

#[tokio::test]
async fn call_correlates_response_with_request_id() {
    let topic = Arc::new(Topic::new().await);
    let (stub, mut client_reader, mut client_writer) =
        start_test_routine(Arc::downgrade(&topic)).await;

    topic.subscribe(Arc::clone(&stub)).await;
    assert_eq!(topic.map(async |_| {}).await.len(), 1);

    let stub_clone = stub.clone();
    let call_task = tokio::spawn(async move {
        stub_clone
            .call(crate::frame::rpc_request::RpcRequest::builder().method_path("echo"))
            .await
    });

    let request = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame(&mut client_reader, MAX_FRAME_SIZE),
    )
    .await
    .unwrap()
    .unwrap();
    let request_id = match request {
        Frame::RpcRequest(request) => match request.response_mode {
            RpcResponseMode::ExpectsResponseWithId(id) => id,
            RpcResponseMode::NoResponse => panic!("expected response id"),
        },
        _ => panic!("expected request frame"),
    };

    let response = RpcResponse::builder()
        .id(request_id)
        .response(&"client response")
        .build();
    Frame::RpcResponse(response)
        .write_frame(&mut client_writer, MAX_FRAME_SIZE)
        .await
        .unwrap();

    let response = call_task.await.unwrap().unwrap();
    assert_eq!(
        response.get_response::<String>().unwrap(),
        "client response"
    );

    stub.stop().await;
    timeout(Duration::from_secs(1), async {
        loop {
            if topic.map(async |_| {}).await.len() == 0 {
                break;
            }
        }
    })
    .await
    .expect("client was not removed from topic");
}

#[tokio::test]
async fn notify_sends_request_without_response_id() {
    let topic = Arc::new(Topic::new().await);
    let (stub, mut client_reader, _client_writer) =
        start_test_routine(Arc::downgrade(&topic)).await;

    topic.subscribe(Arc::clone(&stub)).await;
    assert_eq!(topic.map(async |_| {}).await.len(), 1);

    stub.notify(crate::frame::rpc_request::RpcRequest::builder().method_path("echo"))
        .await
        .unwrap();

    let request = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame(&mut client_reader, MAX_FRAME_SIZE),
    )
    .await
    .unwrap()
    .unwrap();

    match request {
        Frame::RpcRequest(request) => {
            assert_eq!(request.response_mode, RpcResponseMode::NoResponse);
        }
        _ => panic!("expected request frame"),
    }

    stub.stop().await;
    timeout(Duration::from_secs(1), async {
        loop {
            if topic.map(async |_| {}).await.len() == 0 {
                break;
            }
        }
    })
    .await
    .expect("client was not removed from topic");
}

#[tokio::test]
async fn stop_resolves_pending_call() {
    let topic = Arc::new(Topic::new().await);
    let (stub, mut client_reader, _client_writer) =
        start_test_routine(Arc::downgrade(&topic)).await;

    topic.subscribe(Arc::clone(&stub)).await;
    assert_eq!(topic.map(async |_| {}).await.len(), 1);

    let stub_clone = stub.clone();
    let call_task = tokio::spawn(async move {
        stub_clone
            .call(crate::frame::rpc_request::RpcRequest::builder().method_path("echo"))
            .await
    });

    let _request = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame(&mut client_reader, MAX_FRAME_SIZE),
    )
    .await
    .unwrap()
    .unwrap();

    stub.stop().await;

    let error = tokio::time::timeout(std::time::Duration::from_secs(1), call_task)
        .await
        .unwrap()
        .unwrap()
        .unwrap_err();
    assert_matches!(
        error,
        crate::CallError::RoutineIsStopped(StopReason::HandleWasDropped)
    );

    stub.stop().await;
    timeout(Duration::from_secs(1), async {
        loop {
            if topic.map(async |_| {}).await.len() == 0 {
                break;
            }
        }
    })
    .await
    .expect("client was not removed from topic");
}

#[tokio::test]
async fn peer_disconnect_resolves_pending_call() {
    let topic = Arc::new(Topic::new().await);
    let (stub, mut client_reader, client_writer) = start_test_routine(Arc::downgrade(&topic)).await;

    topic.subscribe(Arc::clone(&stub)).await;
    assert_eq!(topic.map(async |_| {}).await.len(), 1);

    let stub_clone = stub.clone();
    let call_task = tokio::spawn(async move {
        stub_clone
            .call(crate::frame::rpc_request::RpcRequest::builder().method_path("echo"))
            .await
    });

    let _request = tokio::time::timeout(
        std::time::Duration::from_secs(1),
        Frame::read_frame(&mut client_reader, MAX_FRAME_SIZE),
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
    assert_matches!(
        error,
        crate::CallError::RoutineIsStopped(StopReason::StreamReadError { .. })
    );

    stub.stop().await;
    timeout(Duration::from_secs(1), async {
        loop {
            if topic.map(async |_| {}).await.len() == 0 {
                break;
            }
        }
    })
    .await
    .expect("client was not removed from topic");
}
