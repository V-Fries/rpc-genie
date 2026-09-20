use tokio::net::{TcpStream, ToSocketAddrs, UnixStream};

pub trait Stream<Addr>: Sized {
    fn connect(addr: &Addr) -> impl Future<Output = Result<Self, std::io::Error>>;
}

impl<Path> Stream<Path> for UnixStream
where
    Path: AsRef<std::path::Path>,
{
    async fn connect(path: &Path) -> Result<Self, std::io::Error> {
        Self::connect(path).await
    }
}

impl<Addr> Stream<Addr> for TcpStream
where
    Addr: ToSocketAddrs,
{
    async fn connect(addr: &Addr) -> Result<Self, std::io::Error> {
        Self::connect(addr).await
    }
}
