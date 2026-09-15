use serde::{Deserialize, Serialize};

#[derive(Debug, thiserror::Error, Serialize, Deserialize, Clone)]
pub enum ReadError {
    #[error("Failed to read frame size: {io_error}")]
    ReadFrameSize { io_error: String },
    #[error("Frame is too big: size is {size}, expected a maximum of {max_size}")]
    FrameTooBig { size: usize, max_size: usize },
    #[error("Data is missing from frame: expected {expected_size} bytes, got {data_size} bytes")]
    MissingData {
        expected_size: usize,
        data_size: usize,
    },
    #[error("Failed to read frame")]
    ReadFrame { io_error: String },
    #[error("Failed to deserialize frame: {deserialize_error}")]
    DeserializeFrame { deserialize_error: String },
}

#[derive(Debug, thiserror::Error, Serialize, Deserialize, Clone)]
pub enum WriteError {
    #[error("Failed to serialize frame: {serialize_error}")]
    SerializeFrame { serialize_error: String },
    #[error("Frame is too big: size is {size}, expected a maximum of {max_size}")]
    FrameTooBig { size: usize, max_size: usize },
    #[error("Failed to write frame size: {io_error}")]
    WriteFrameSize { io_error: String },
    #[error("Failed to write frame: {io_error}")]
    WriteFrame { io_error: String },
    #[error("Failed to flush stream: {io_error}")]
    FlushStream { io_error: String },
}
