mod builder;
// TODO remove allow unused
#[allow(unused)]
pub use builder::{RpcRequestBuilder, Uninit as RpcRequestBuilderUninit};

mod arg_reader;
// TODO remove allow unused
#[allow(unused)]
pub use arg_reader::{ReadParamError, RpcRequestArgReader};

use crate::frame::RpcRequestId;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RpcRequest {
    pub(crate) method_path: String,
    pub(crate) args: Box<[u8]>,
    pub(crate) id: RpcRequestId,
}

impl RpcRequest {
    pub fn builder() -> RpcRequestBuilder<RpcRequestBuilderUninit, RpcRequestBuilderUninit> {
        RpcRequestBuilder::default()
    }
}
