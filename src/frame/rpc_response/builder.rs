use crate::frame::RpcRequestId;

use super::RpcResponse;

pub struct Uninit;
type ResponseInit = Result<Box<[u8]>, crate::Error>;

pub struct RpcResponseBuilder<RequestId, Response> {
    id: RequestId,
    response: Response,
}

impl Default for RpcResponseBuilder<Uninit, Uninit> {
    fn default() -> Self {
        Self {
            id: Uninit,
            response: Uninit,
        }
    }
}

impl RpcResponseBuilder<RpcRequestId, ResponseInit> {
    pub fn build(self) -> RpcResponse {
        RpcResponse {
            id: self.id,
            response: self.response,
        }
    }
}

impl<RequestId> RpcResponseBuilder<RequestId, Uninit> {
    /// Sets the return value.
    ///
    /// # Implementation detail
    /// The return value will be encoded in MessagePack using the [rmp_serde] crate.
    ///
    /// # Panic
    /// This function will panic if return_value is not serializable using MessagePack
    pub fn response(
        self,
        response: &impl serde::Serialize,
    ) -> RpcResponseBuilder<RequestId, ResponseInit> {
        RpcResponseBuilder {
            id: self.id,
            response: Ok(rmp_serde::to_vec_named(response)
                .expect("return value must be serializable in message pack")
                .into_boxed_slice()),
        }
    }

    /// Disable the return value.
    pub fn error(self, error: crate::Error) -> RpcResponseBuilder<RequestId, ResponseInit> {
        RpcResponseBuilder {
            id: self.id,
            response: Err(error),
        }
    }
}

impl<Response> RpcResponseBuilder<Uninit, Response> {
    /// Sets the request id
    pub fn id(self, id: RpcRequestId) -> RpcResponseBuilder<RpcRequestId, Response> {
        RpcResponseBuilder {
            id,
            response: self.response,
        }
    }
}
