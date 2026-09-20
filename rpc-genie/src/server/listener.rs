use std::{io, time::Duration};

use tokio::net::{TcpListener, TcpStream, UnixListener, UnixStream};

pub trait Listener: Sized + Send + 'static {
    type Stream;

    fn bind(addr: &str) -> impl Future<Output = Result<Self, io::Error>>;

    fn accept(&self) -> impl Future<Output = Self::Stream> + Send;

    fn on_routine_end(addr: &str);
}

impl Listener for UnixListener {
    type Stream = UnixStream;

    async fn bind(path: &str) -> Result<Self, io::Error> {
        Self::bind(path)
    }

    async fn accept(&self) -> UnixStream {
        loop {
            match self.accept().await {
                Ok((stream, _)) => return stream,
                Err(error) => handle_accept_error(error).await,
            }
        }
    }

    fn on_routine_end(path: &str) {
        if let Err(err) = std::fs::remove_file(path) {
            eprintln!("Error: Failed to delete unix socket file {path:?}: {err}",)
        }
    }
}

impl Listener for TcpListener {
    type Stream = TcpStream;

    async fn bind(addr: &str) -> Result<Self, io::Error> {
        Self::bind(addr).await
    }

    async fn accept(&self) -> TcpStream {
        loop {
            match self.accept().await {
                Ok((stream, _)) => return stream,
                Err(error) => handle_accept_error(error).await,
            }
        }
    }

    fn on_routine_end(_addr: &str) {}
}

async fn handle_accept_error(error: io::Error) {
    if matches!(
        error.kind(),
        io::ErrorKind::ConnectionRefused
            | io::ErrorKind::ConnectionReset
            | io::ErrorKind::ConnectionAborted
    ) {
        return;
    }

    // TODO log error

    tokio::time::sleep(Duration::from_secs(1)).await;
}
