use std::{fmt::Debug, os::fd::RawFd, time::Instant};

// TODO remove allow dead_code
#[allow(dead_code)]
#[derive(Debug, Clone, Copy)]
pub struct StreamId {
    fd: RawFd,
    creation_time: Instant,
}

impl From<RawFd> for StreamId {
    fn from(fd: RawFd) -> Self {
        Self {
            fd,
            creation_time: Instant::now(),
        }
    }
}

impl std::fmt::Display for StreamId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        <Self as Debug>::fmt(self, f)
    }
}
