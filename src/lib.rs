#[doc(hidden)]
pub mod frame;

use std::sync::Arc;

pub use rpc_genie_macros::service;

use crate::frame::{RpcRequestId, rpc_request::RpcRequestArgReader, rpc_response::RpcResponse};

#[derive(serde::Deserialize, serde::Serialize, Debug, thiserror::Error)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum Error {
    #[error("Method not found")]
    MethodNotFound,
    #[error("Failed to deserialize args: {details}")]
    FailedToDeserializeArg { details: String },
    #[error("Failed to deserialize response: {details}")]
    FailedToDeserializeResponse { details: String },
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[doc(hidden)]
#[allow(dead_code)] // TODO remove allow dead_code
pub struct RequestHandler<State, SubServices> {
    pub service_path: Option<Arc<String>>,
    pub state: Arc<State>,
    pub sub_services: SubServices,
}

#[doc(hidden)]
#[allow(async_fn_in_trait)]
pub trait HandleRequest<Stub>: Send {
    fn handle_request(
        &self,
        method_path: &str,
        __rpc_stub__: Stub,
        __rpc_request_arg_reader__: RpcRequestArgReader,
        __rpc_request_id__: RpcRequestId,
    ) -> impl Future<Output = RpcResponse> + Send;
}

#[doc(hidden)]
pub trait IntoRequestHandler<RequestHandler, SubServices: SubServicesFromState<Self>, Stub>
where
    Self: Sized,
    RequestHandler: HandleRequest<Stub>,
{
    fn into_request_handler(self: Arc<Self>, service_path: Option<Arc<String>>) -> RequestHandler;
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
