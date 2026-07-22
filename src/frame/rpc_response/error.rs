#[derive(Debug, thiserror::Error)]
pub enum ReturnValueError {
    #[error("{0}")]
    RpcError(#[from] RpcError),
    #[error("Failed to decode return value: {0}")]
    DecodingReturnValue(#[from] rmp_serde::decode::Error),
}

#[derive(Debug, thiserror::Error, serde::Deserialize, serde::Serialize)]
#[cfg_attr(test, derive(PartialEq, Eq))]
pub enum RpcError {
    #[error("No method named \"{method_name}\" was found")]
    NoSuchMethod { method_name: String },
    #[error("Connection error")]
    ConnectionError,
}

#[cfg(test)]
impl ReturnValueError {
    pub fn unwrap_rpc_err(self) -> RpcError {
        match self {
            Self::RpcError(error) => error,
            err => panic!("Expected RpcError, found: {err}"),
        }
    }
}
