pub mod builder;
pub use builder::RpcResponseBuilder;

use crate::frame::RpcRequestId;

#[derive(serde::Deserialize, serde::Serialize, Debug, thiserror::Error)]
#[cfg_attr(test, derive(Eq, PartialEq))]
pub enum RpcResponseError {
    #[error("Method not found")]
    MethodNotFound,
    #[error("Failed to deserialize args: {details}")]
    FailedToDeserializeArg { details: String },
    #[error("Failed to deserialize response: {details}")]
    FailedToDeserializeResponse { details: String },
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
                details: err.to_string(),
            }
        })
    }

    pub fn builder() -> RpcResponseBuilder<builder::Uninit, builder::Uninit> {
        RpcResponseBuilder::default()
    }
}
