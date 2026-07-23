use thiserror::Error;
use tokio::io;

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("Failed to read frame size: {0}")]
    ReadFrameSize(io::Error),
    #[error("Frame is too big: size is {size}, expected a maximum of {max_size}")]
    FrameTooBig { size: usize, max_size: usize },
    #[error("Data is missing from frame: expected {expected_size} bytes, got {data_size} bytes")]
    MissingData {
        expected_size: usize,
        data_size: usize,
    },
    #[error("Failed to read frame: {0}")]
    ReadFrame(io::Error),
    #[error("Failed to deserialize frame: {0}")]
    DeserializeFrame(#[from] rmp_serde::decode::Error),
}

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("Failed to serialize frame: {0}")]
    SerializeFrame(#[from] rmp_serde::encode::Error),
    #[error("Frame is too big: size is {size}, expected a maximum of {max_size}")]
    FrameTooBig { size: usize, max_size: usize },
    #[error("Failed to write frame size: {0}")]
    WriteFrameSize(io::Error),
    #[error("Failed to write frame: {0}")]
    WriteFrame(io::Error),
}
