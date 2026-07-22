mod builder;
// TODO remove allow unused
#[allow(unused)]
pub use builder::{RpcRequestBuilder, Uninit as RpcRequestBuilderUninit};

mod reader;
// TODO remove allow unused
#[allow(unused)]
pub use reader::{ReadParamError, RpcRequestReader};

use crate::frame::RpcRequestId;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RpcRequest {
    method: String,
    params: Box<[u8]>,
    request_id: RpcRequestId,
}
