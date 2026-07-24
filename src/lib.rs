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
pub struct RequestHandler<'state, State, SubServices> {
    service_path: Option<Arc<String>>,
    pub state: &'state State,
    pub sub_services: SubServices,
}

#[doc(hidden)]
#[allow(async_fn_in_trait)]
pub trait HandleRequest<Stub> {
    async fn handle_request(
        &self,
        method_path: &str,
        __rpc_stub__: Stub,
        __rpc_request_arg_reader__: RpcRequestArgReader,
        __rpc_request_id__: RpcRequestId,
    ) -> RpcResponse;
}

#[doc(hidden)]
pub trait AsRequestHandler<'state, SubServices: SubServicesFromState<'state, Self>>: Sized {
    fn as_request_handler(
        &'state self,
        service_path: Option<Arc<String>>,
    ) -> RequestHandler<'state, Self, SubServices> {
        RequestHandler {
            state: self,
            sub_services: SubServices::from_state(
                self,
                service_path.as_deref().map(String::as_str),
            ),
            service_path,
        }
    }
}

#[doc(hidden)]
pub trait SubServicesFromState<'state, State> {
    fn from_state(state: &'state State, service_path: Option<&str>) -> Self;
}
