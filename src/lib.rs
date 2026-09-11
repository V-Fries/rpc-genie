mod single_request_sender;
pub use single_request_sender::SingleRequestSender;

#[doc(hidden)]
pub mod frame;

use std::sync::Arc;

pub use rpc_genie_macros::service;

use crate::frame::{
    rpc_request::RpcRequestArgReader,
    rpc_response::{self, RpcResponseBuilder},
};

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("Failed to write frame: {0}")]
    FailedToWriteFrame(frame::WriteError),
    #[error("Method not found")]
    MethodNotFound,
    #[error("Failed to deserialize args: {details}")]
    FailedToDeserializeArg { details: String },
    #[error("Failed to deserialize response: {details}")]
    FailedToDeserializeResponse { details: String },
}

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("Failed to write frame: {0}")]
    FailedToWriteFrame(frame::WriteError),
}

#[doc(hidden)]
#[allow(async_fn_in_trait)]
pub trait HandleRequest<Stub>: Send {
    fn handle_request(
        &self,
        method_path: &str,
        __rpc_stub__: Stub,
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
pub trait Client: State {}

/// Every Server structs in rpc-genie services will automatically implement this trait.
///
/// It is used to identify a struct as a Server to check at compile time that you don't pass a
/// server state to a function that expects a client state.
pub trait Server: State {}

#[doc(hidden)]
pub trait Stub: Clone + Send {
    // TODO This prototype will be replaced in the future. We just need a way to create a stub so
    // we can continue working on other parts of the code
    fn new() -> Self;
}
