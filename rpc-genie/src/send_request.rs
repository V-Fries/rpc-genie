use crate::frame::{
    rpc_request::{RpcRequestBuilder, RpcRequestBuilderUninit},
    rpc_response::RpcResponse,
};

pub trait SendRequest: Clone + Send + Sync + 'static {
    fn call(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> impl Future<Output = Result<RpcResponse, crate::CallError>> + Send;

    fn notify(
        &self,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> impl Future<Output = Result<(), crate::NotifyError>> + Send;
}
