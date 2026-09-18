use std::{io, time::Duration};

use tokio::net::{UnixListener, UnixStream};

pub trait Listener<Addr, Stream>: Sized + Send + 'static {
    fn bind(addr: &Addr) -> impl Future<Output = Result<Self, io::Error>>;

    fn accept(&self) -> impl Future<Output = Stream> + Send;
}

impl<Path> Listener<Path, UnixStream> for UnixListener
where
    Path: AsRef<std::path::Path>,
{
    async fn bind(path: &Path) -> Result<Self, io::Error> {
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
