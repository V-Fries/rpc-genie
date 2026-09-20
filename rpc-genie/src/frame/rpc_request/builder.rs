use super::{RpcRequest, RpcResponseMode};

pub struct Uninit;

pub struct RpcRequestBuilder<RpcResponseMode, MethodPath> {
    method_path: MethodPath,
    args: Vec<u8>,
    response_mode: RpcResponseMode,
}

impl Default for RpcRequestBuilder<Uninit, Uninit> {
    fn default() -> Self {
        Self {
            method_path: Uninit,
            args: Vec::new(),
            response_mode: Uninit,
        }
    }
}

impl RpcRequestBuilder<RpcResponseMode, String> {
    pub fn build(self) -> RpcRequest {
        RpcRequest {
            method_path: self.method_path,
            args: self.args.into_boxed_slice(),
            response_mode: self.response_mode,
        }
    }
}

impl<MethodPath> RpcRequestBuilder<Uninit, MethodPath> {
    pub fn response_mode(
        self,
        response_mode: RpcResponseMode,
    ) -> RpcRequestBuilder<RpcResponseMode, MethodPath> {
        RpcRequestBuilder {
            method_path: self.method_path,
            args: self.args,
            response_mode,
        }
    }
}

impl<RpcResponseMode> RpcRequestBuilder<RpcResponseMode, Uninit> {
    pub fn method_path(
        self,
        method_path: impl Into<String>,
    ) -> RpcRequestBuilder<RpcResponseMode, String> {
        RpcRequestBuilder {
            method_path: method_path.into(),
            args: self.args,
            response_mode: self.response_mode,
        }
    }
}

impl<RpcResponseMode, MethodPath> RpcRequestBuilder<RpcResponseMode, MethodPath> {
    /// Adds a param that the rpc request function will use.
    ///
    /// # Implementation detail
    /// The param will be encoded in MessagePack using the [rmp_serde] crate.
    ///
    /// # Panic
    /// This function will panic if param is not serializable using MessagePack
    pub fn add_param<T: serde::Serialize>(mut self, param: T) -> Self {
        let serialized_param = rmp_serde::to_vec_named(&param).expect("Failed to serialize param");

        self.args.extend(serialized_param);
        self
    }
}
