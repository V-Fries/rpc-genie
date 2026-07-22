mod error;
pub use error::{ReturnValueError, RpcError};

mod builder;
// TODO remove allow unused
#[allow(unused)]
pub use builder::RpcResponseBuilder;

use crate::frame::RpcRequestId;

#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub struct RpcResponse {
    pub id: RpcRequestId,
    pub return_value: Result<Box<[u8]>, RpcError>,
}

impl RpcResponse {
    /// Getter for the response id
    pub fn id(&self) -> RpcRequestId {
        self.id
    }

    /// Tries to parse the result value from the response.
    pub fn get_return_value<T: serde::de::DeserializeOwned>(self) -> Result<T, ReturnValueError> {
        Ok(rmp_serde::from_slice(&self.return_value?)?)
    }
}
