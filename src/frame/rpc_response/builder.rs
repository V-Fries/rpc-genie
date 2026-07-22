use crate::frame::RpcRequestId;

use super::{RpcError, RpcResponse};

pub struct Uninit;
type ReturnValueInit = Result<Box<[u8]>, RpcError>;

pub struct RpcResponseBuilder<RequestId, ReturnValue> {
    id: RequestId,
    return_value: ReturnValue,
}

impl Default for RpcResponseBuilder<Uninit, Uninit> {
    fn default() -> Self {
        Self {
            id: Uninit,
            return_value: Uninit,
        }
    }
}

impl RpcResponseBuilder<Uninit, Uninit> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl RpcResponseBuilder<RpcRequestId, ReturnValueInit> {
    pub fn build(self) -> RpcResponse {
        RpcResponse {
            id: self.id,
            return_value: self.return_value,
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
    pub fn set_return_value(
        self,
        return_value: &impl serde::Serialize,
    ) -> RpcResponseBuilder<RequestId, ReturnValueInit> {
        RpcResponseBuilder {
            id: self.id,
            return_value: Ok(rmp_serde::to_vec_named(return_value)
                .expect("return value must be serializable in message pack")
                .into_boxed_slice()),
        }
    }

    /// Disable the return value.
    pub fn set_error(self, error: RpcError) -> RpcResponseBuilder<RequestId, ReturnValueInit> {
        RpcResponseBuilder {
            id: self.id,
            return_value: Err(error),
        }
    }
}

impl<ReturnValue> RpcResponseBuilder<Uninit, ReturnValue> {
    /// Sets the request id
    pub fn id(self, id: impl Into<RpcRequestId>) -> RpcResponseBuilder<RpcRequestId, ReturnValue> {
        RpcResponseBuilder {
            id: id.into(),
            return_value: self.return_value,
        }
    }
}
