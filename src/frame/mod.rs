mod error;
// TODO remove allow dead_code
#[allow(dead_code)]
pub use error::{ReadError, WriteError};

// TODO remove allow dead_code
#[allow(dead_code)]
pub mod rpc_request;
use rpc_request::RpcRequest;

// TODO remove allow dead_code
#[allow(dead_code)]
pub mod rpc_response;
use rpc_response::RpcResponse;

mod request_id;
pub use request_id::RpcRequestId;

#[cfg(test)]
mod test;

use tokio::io::{
    AsyncRead, AsyncReadExt as _, AsyncWrite, AsyncWriteExt as _, BufReader, BufWriter,
};

// TODO remove allow dead_code
#[allow(dead_code)]
#[derive(Debug, serde::Deserialize, serde::Serialize)]
pub enum Frame {
    RpcRequest(RpcRequest),
    RpcResponse(RpcResponse),
}

impl Frame {
    /// This function does not automatically flush the stream.
    ///
    /// # Cancel safety
    /// This method is not cancellation safe. If the method is used as the
    /// event in a [`tokio::select!`] statement and some
    /// other branch completes first, then some data may be lost.
    // TODO remove allow dead_code
    #[allow(dead_code)]
    pub async fn write_frame<const MAX_FRAME_SIZE: usize>(
        self,
        stream: &mut BufWriter<impl AsyncWrite + Unpin>,
    ) -> Result<(), WriteError> {
        let data = rmp_serde::to_vec(&self)?;

        if data.len() > MAX_FRAME_SIZE {
            return Err(WriteError::FrameTooBig {
                size: data.len(),
                max_size: MAX_FRAME_SIZE,
            });
        }

        stream
            .write_u64(data.len() as u64)
            .await
            .map_err(WriteError::WriteFrameSize)?;
        stream
            .write_all(&data)
            .await
            .map_err(WriteError::WriteFrame)
    }

    /// # Cancel safety
    /// This method is not cancellation safe. If the method is used as the
    /// event in a [`tokio::select!`] statement and some
    /// other branch completes first, then some data may be lost.
    // TODO remove allow dead_code
    #[allow(dead_code)]
    pub async fn read_frame<const MAX_FRAME_SIZE: usize>(
        stream: &mut BufReader<impl AsyncRead + Unpin>,
    ) -> Result<Self, ReadError> {
        let size = stream.read_u64().await.map_err(ReadError::ReadFrameSize)?;

        if size as usize > MAX_FRAME_SIZE {
            return Err(ReadError::FrameTooBig {
                size: size as usize,
                max_size: MAX_FRAME_SIZE,
            });
        }

        let mut frame_bytes = Vec::<u8>::with_capacity(size as usize);
        stream
            .take(size)
            .read_to_end(&mut frame_bytes)
            .await
            .map_err(ReadError::ReadFrame)?;
        if frame_bytes.len() != size as usize {
            return Err(ReadError::MissingData {
                expected_size: size as usize,
                data_size: frame_bytes.len(),
            });
        }

        let frame = rmp_serde::from_slice(&frame_bytes)?;

        Ok(frame)
    }
}
