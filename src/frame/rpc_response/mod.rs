pub mod builder;
pub use builder::RpcResponseBuilder;

use crate::{CallError, frame::RpcRequestId};

#[derive(serde::Deserialize, serde::Serialize, Debug, thiserror::Error)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum RpcResponseError {
    #[error("Method not found")]
    MethodNotFound,
    #[error("Failed to deserialize args: {serialize_error}")]
    FailedToDeserializeArg { serialize_error: String },
    #[error("Failed to deserialize response: {serialize_error}")]
    FailedToDeserializeResponse { serialize_error: String },
}

#[allow(clippy::from_over_into)]
impl Into<CallError> for RpcResponseError {
    fn into(self) -> CallError {
        match self {
            RpcResponseError::MethodNotFound => CallError::MethodNotFound,
            RpcResponseError::FailedToDeserializeArg { serialize_error } => {
                CallError::FailedToDeserializeArg {
                    deserialize_error: serialize_error,
                }
            }
            RpcResponseError::FailedToDeserializeResponse { serialize_error } => {
                CallError::FailedToDeserializeResponse {
                    deserialize_error: serialize_error,
                }
            }
        }
    }
}

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RpcResponse {
    pub id: RpcRequestId,
    pub response: Result<Box<[u8]>, RpcResponseError>,
}

impl RpcResponse {
    /// Getter for the response id
    pub fn id(&self) -> RpcRequestId {
        self.id
    }

    /// Tries to parse the result value from the response.
    pub fn get_response<T: serde::de::DeserializeOwned>(self) -> Result<T, RpcResponseError> {
        rmp_serde::from_slice(&self.response?).map_err(|err| {
            RpcResponseError::FailedToDeserializeResponse {
                serialize_error: err.to_string(),
            }
        })
    }

    pub fn builder() -> RpcResponseBuilder<builder::Uninit, builder::Uninit> {
        RpcResponseBuilder::default()
    }
}
