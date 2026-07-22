pub use rmp_serde::decode::Error as ReadParamError;

use crate::frame::rpc_request::RpcRequestId;

use super::RpcRequest;

use std::io::Cursor;

pub struct RpcRequestReader {
    method: String,
    params: Cursor<Box<[u8]>>,
    id: RpcRequestId,
}

impl From<RpcRequest> for RpcRequestReader {
    fn from(raw_rpc_request: RpcRequest) -> Self {
        Self {
            method: raw_rpc_request.method,
            params: Cursor::new(raw_rpc_request.params),
            id: raw_rpc_request.request_id,
        }
    }
}

impl RpcRequestReader {
    /// Getter for the request id
    pub fn id(&self) -> RpcRequestId {
        self.id
    }

    /// Getter for the request method path
    pub fn method(&self) -> &str {
        &self.method
    }

    /// Tries to read a param from the request.
    /// Should only fail when invalid data was passed through the stream. i.e invalid version,
    /// malicious request, etc...
    pub fn read_param<T: serde::de::DeserializeOwned>(&mut self) -> Result<T, ReadParamError> {
        rmp_serde::from_read(&mut self.params)
    }
}
