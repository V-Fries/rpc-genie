use crate::frame::rpc_request::RpcRequestId;

use super::RpcRequest;

pub struct Uninit;

pub struct RpcRequestBuilder<RpcRequestId, Method> {
    method: Method,
    params: Vec<u8>,
    request_id: RpcRequestId,
}

impl Default for RpcRequestBuilder<Uninit, Uninit> {
    fn default() -> Self {
        Self {
            method: Uninit,
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
            method: self.method,
            params: self.params.into_boxed_slice(),
            request_id: self.request_id,
        }
    }
}

impl<Method> RpcRequestBuilder<Uninit, Method> {
    /// Sets the rpc request id to the given id (see [RpcRequestId])
    pub fn id(self, id: impl Into<RpcRequestId>) -> RpcRequestBuilder<RpcRequestId, Method> {
        RpcRequestBuilder {
            method: self.method,
            params: self.params,
            request_id: id.into(),
        }
    }
}

impl<RpcRequestId> RpcRequestBuilder<RpcRequestId, Uninit> {
    /// Setter for the method path
    pub fn method(self, method: impl Into<String>) -> RpcRequestBuilder<RpcRequestId, String> {
        RpcRequestBuilder {
            method: method.into(),
            params: self.params,
            request_id: self.request_id,
        }
    }
}

impl<RpcRequestId, Method> RpcRequestBuilder<RpcRequestId, Method> {
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
