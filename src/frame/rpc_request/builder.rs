use crate::frame::rpc_request::RpcRequestId;

use super::RpcRequest;

pub struct Uninit;

pub struct RpcRequestBuilder<RpcRequestId, MethodPath> {
    method_path: MethodPath,
    params: Vec<u8>,
    request_id: RpcRequestId,
}

impl Default for RpcRequestBuilder<Uninit, Uninit> {
    fn default() -> Self {
        Self {
            method_path: Uninit,
            params: Vec::new(),
            request_id: Uninit,
        }
    }
}

impl RpcRequestBuilder<Uninit, Uninit> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RpcRequestBuilder<RpcRequestId, String> {
    pub fn build(self) -> RpcRequest {
        RpcRequest {
            method_path: self.method_path,
            params: self.params.into_boxed_slice(),
            request_id: self.request_id,
        }
    }
}

impl<MethodPath> RpcRequestBuilder<Uninit, MethodPath> {
    /// Sets the rpc request id to the given id (see [RpcRequestId])
    pub fn id(self, id: impl Into<RpcRequestId>) -> RpcRequestBuilder<RpcRequestId, MethodPath> {
        RpcRequestBuilder {
            method_path: self.method_path,
            params: self.params,
            request_id: id.into(),
        }
    }
}

impl<RpcRequestId> RpcRequestBuilder<RpcRequestId, Uninit> {
    /// Setter for the method path
    pub fn method_path(self, method: impl Into<String>) -> RpcRequestBuilder<RpcRequestId, String> {
        RpcRequestBuilder {
            method_path: method.into(),
            params: self.params,
            request_id: self.request_id,
        }
    }
}

impl<RpcRequestId, MethodPath> RpcRequestBuilder<RpcRequestId, MethodPath> {
    /// Adds a param that the rpc request function will use.
    ///
    /// # Implementation detail
    /// The param will be encoded in MessagePack using the [rmp_serde] crate.
    ///
    /// # Panic
    /// This function will panic if param is not serializable using MessagePack
    pub fn add_param(mut self, param: &impl serde::Serialize) -> Self {
        let serialized_param = rmp_serde::to_vec_named(param).expect("Failed to serialize param");

        self.params.extend(serialized_param);
        self
    }
}
