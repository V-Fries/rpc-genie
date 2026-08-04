mod builder;
// TODO remove allow unused
#[allow(unused)]
pub use builder::RpcResponseBuilder;

use crate::frame::RpcRequestId;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RpcResponse {
    pub id: RpcRequestId,
    pub response: Result<Box<[u8]>, crate::Error>,
}

impl RpcResponse {
    /// Getter for the response id
    pub fn id(&self) -> RpcRequestId {
        self.id
    }

    /// Tries to parse the result value from the response.
    pub fn get_response<T: serde::de::DeserializeOwned>(self) -> Result<T, crate::Error> {
        rmp_serde::from_slice(&self.response?).map_err(|err| {
            crate::Error::FailedToDeserializeResponse {
                details: err.to_string(),
            }
        })
    }

    pub fn builder() -> RpcResponseBuilder<builder::Uninit, builder::Uninit> {
        RpcResponseBuilder::default()
    }
}
