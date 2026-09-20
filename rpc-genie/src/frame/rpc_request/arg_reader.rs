pub use rmp_serde::decode::Error as ReadParamError;

use std::io::Cursor;

pub struct RpcRequestArgReader {
    args: Cursor<Box<[u8]>>,
}

impl From<Box<[u8]>> for RpcRequestArgReader {
    fn from(raw_rpc_request_args: Box<[u8]>) -> Self {
        Self {
            args: Cursor::new(raw_rpc_request_args),
        }
    }
}

impl RpcRequestArgReader {
    /// Tries to read an arg from the request.
    /// Should only fail when invalid data was passed through the stream. i.e invalid version,
    /// malicious request, etc...
    pub fn read_arg<T: serde::de::DeserializeOwned>(
        &mut self,
    ) -> Result<T, rmp_serde::decode::Error> {
        rmp_serde::from_read(&mut self.args)
    }
}
