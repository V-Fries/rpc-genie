use thiserror::Error;
use tokio::io;

#[derive(Debug, Error)]
pub enum ReadError {
    #[error("Failed to read frame size")]
    ReadFrameSize(#[source] io::Error),
    #[error("Frame is too big: size is {size}, expected a maximum of {max_size}")]
    FrameTooBig { size: usize, max_size: usize },
    #[error("Data is missing from frame: expected {expected_size} bytes, got {data_size} bytes")]
    MissingData {
        expected_size: usize,
        data_size: usize,
    },
    #[error("Failed to read frame")]
    ReadFrame(#[source] io::Error),
    #[error("Failed to deserialize frame")]
    DeserializeFrame(#[from] rmp_serde::decode::Error),
}

#[derive(Debug, Error)]
pub enum WriteError {
    #[error("Failed to serialize frame")]
    SerializeFrame(#[from] rmp_serde::encode::Error),
    #[error("Frame is too big: size is {size}, expected a maximum of {max_size}")]
    FrameTooBig { size: usize, max_size: usize },
    #[error("Failed to write frame size")]
    WriteFrameSize(#[source] io::Error),
    #[error("Failed to write frame")]
    WriteFrame(#[source] io::Error),
    #[error("Failed to flush stream")]
    FlushStream(#[source] io::Error),
}
