mod spawn_client_routine;
mod spawn_server_routine;

use std::ops::Deref;

use crate::{
    SendRequest, Stub,
    frame::rpc_request::{RpcRequest, RpcRequestBuilder, RpcRequestBuilderUninit, RpcResponseMode},
};

use super::*;

const MAX_FRAME_SIZE: usize = 1024;

type TestStream = tokio::io::DuplexStream;
type TestHandle = super::Handle<MAX_FRAME_SIZE, TestStream>;

struct TestStubWeakHandle(Weak<TestHandle>);

impl SendRequest for TestStubWeakHandle {
    async fn call(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<RpcResponse, crate::CallError> {
        self.0.call(request_builder).await
    }

    async fn notify(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<(), crate::NotifyError> {
        self.0.notify(request_builder).await
    }
}

impl Clone for TestStubWeakHandle {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Stub<Weak<TestHandle>> for TestStubWeakHandle {
    fn new(request_sender: Weak<TestHandle>, _service_path: Option<String>) -> Self {
        Self(request_sender)
    }
}

struct TestStubArcHandle(Arc<TestHandle>);

impl SubscribableStub for TestStubArcHandle {
    fn add_registered_topic(
        &self,
        topic_id: crate::topic::TopicId,
        stub_died_notification_sender: crate::topic::StubDiedNotificationSender,
    ) -> impl Future<Output = bool> + Send {
        self.0
            .deref()
            .add_registered_topic(topic_id, stub_died_notification_sender)
    }

    fn remove_registered_topic(&self, topic_id: crate::topic::TopicId) {
        self.0.deref().remove_registered_topic(topic_id)
    }

    fn stream_id(&self) -> Option<StreamId> {
        self.0.deref().stream_id()
    }
}

impl SendRequest for TestStubArcHandle {
    async fn call(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<RpcResponse, crate::CallError> {
        self.0.call(request_builder).await
    }

    async fn notify(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<(), crate::NotifyError> {
        self.0.notify(request_builder).await
    }
}

impl TestStubArcHandle {
    async fn stop(&self) {
        self.0.stop().await;
    }
}

impl Clone for TestStubArcHandle {
    fn clone(&self) -> Self {
        Self(self.0.clone())
    }
}

impl Stub<Arc<TestHandle>> for TestStubArcHandle {
    fn new(request_sender: Arc<TestHandle>, _service_path: Option<String>) -> Self {
        Self(request_sender)
    }
}

struct TestHandler;

impl HandleRequest<TestStubWeakHandle> for TestHandler {
    async fn handle_request(
        &self,
        method_path: &str,
        _stub: &Arc<TestStubWeakHandle>,
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
