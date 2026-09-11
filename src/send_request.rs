use crate::frame::{
    rpc_request::{RpcRequestBuilder, RpcRequestBuilderUninit},
    rpc_response::RpcResponse,
};

#[allow(async_fn_in_trait)]
pub trait SendRequest: Clone + Send + Sync + 'static {
    async fn call(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<RpcResponse, crate::CallError>;

    async fn notify(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Result<(), crate::NotifyError>;
}
