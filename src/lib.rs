mod single_request_sender;
pub use single_request_sender::SingleRequestSender;

#[doc(hidden)]
pub mod stream_handler;

#[doc(hidden)]
pub mod send_request;
pub use send_request::SendRequest;

#[doc(hidden)]
pub mod topic;
pub use topic::Topic;

#[doc(hidden)]
pub mod frame;

pub mod client;
pub mod server;

use std::sync::Arc;

pub use service_macro::service;

use crate::{
    frame::{
        rpc_request::RpcRequestArgReader,
        rpc_response::{self, RpcResponseBuilder},
    },
    stream_handler::StreamId,
};

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("Stream handler routine is stopped")]
    RoutineIsStopped(#[from] stream_handler::StopReason),
    #[error("Failed to write frame")]
    FailedToWriteFrame(#[from] frame::WriteError),
    #[error("Method not found")]
    MethodNotFound,
    #[error("Failed to deserialize args: {deserialize_error}")]
    FailedToDeserializeArg { deserialize_error: String },
    #[error("Failed to deserialize response: {deserialize_error}")]
    FailedToDeserializeResponse { deserialize_error: String },
}

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("Stream handler routine is stopped")]
    RoutineIsStopped(#[from] stream_handler::StopReason),
    #[error("Failed to write frame")]
    FailedToWriteFrame(#[from] frame::WriteError),
}

#[doc(hidden)]
#[allow(async_fn_in_trait)]
pub trait HandleRequest<OppositeStubWeakHandle>: Sync + Send + 'static {
    fn handle_request(
        &self,
        method_path: &str,
        __rpc_opposite_stub_weak_handle__: &Arc<OppositeStubWeakHandle>,
        __rpc_request_arg_reader__: RpcRequestArgReader,
    ) -> impl Future<
        Output = RpcResponseBuilder<
            rpc_response::builder::Uninit,
            rpc_response::builder::ResponseInit,
        >,
    > + Send;
}

#[doc(hidden)]
pub trait IntoRequestHandler<RequestHandler, SubServices: SubServicesFromState<Self>>
where
    Self: Sized,
{
    fn into_request_handler(self: Arc<Self>, service_path: Option<String>) -> Arc<RequestHandler>;
}

#[doc(hidden)]
pub trait SubServicesFromState<State> {
    fn from_state(state: Arc<State>, service_path: Option<&str>) -> Self;
}

#[doc(hidden)]
pub trait State: Send + Sync + 'static {}

/// Every Client structs in rpc-genie services will automatically implement this trait.
///
/// It is used to identify a struct as a Client to check at compile time that you don't pass a
/// client state to a function that expects a server state.
pub trait Client<const MAX_FRAME_SIZE: usize, Stream, ServerStubWeakHandle>: State {
    type ServerStubArcHandle;
}

/// Every Server structs in rpc-genie services will automatically implement this trait.
///
/// It is used to identify a struct as a Server to check at compile time that you don't pass a
/// server state to a function that expects a client state.
pub trait Server<const MAX_FRAME_SIZE: usize, Stream, ClientStubWeakHandle>: State {
    type ClientStubArcHandle;
}

#[doc(hidden)]
pub trait Stub<RequestSender>
where
    Self: Clone + Send + 'static + Sync,
    RequestSender: send_request::SendRequest,
{
    fn new(request_sender: RequestSender, service_path: Option<String>) -> Self;
}

#[doc(hidden)]
pub trait SubscribableStub: Sized {
    /// Returns false is the stub is already dead, true otherwise
    fn add_registered_topic(
        &self,
        topic_id: topic::TopicId,
        stub_died_notification_sender: topic::StubDiedNotificationSender,
    ) -> impl Future<Output = bool> + Send;

    fn remove_registered_topic(&self, topic_id: topic::TopicId);

    /// Returns None if the stream identity is unavailable, Some(stream_id) otherwise
    fn stream_id(&self) -> Option<StreamId>;
}
