mod builder;
pub use builder::{RpcRequestBuilder, Uninit as RpcRequestBuilderUninit};

mod arg_reader;
pub use arg_reader::{ReadParamError, RpcRequestArgReader};

use crate::frame::RpcRequestId;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RpcRequest {
    pub(crate) method_path: String,
    pub(crate) args: Box<[u8]>,
    pub(crate) response_mode: RpcResponseMode,
}

#[derive(Debug, serde::Deserialize, serde::Serialize, Clone, Copy)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum RpcResponseMode {
    ExpectsResponseWithId(RpcRequestId),
    NoResponse,
}

impl RpcRequest {
    pub fn builder() -> RpcRequestBuilder<RpcRequestBuilderUninit, RpcRequestBuilderUninit> {
        RpcRequestBuilder::default()
    }
}
