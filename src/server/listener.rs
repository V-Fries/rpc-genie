use std::io;

pub trait Listener<Addr, Stream>: Sized + Send + 'static {
    async fn bind(addr: &Addr) -> Result<Self, io::Error>;

    fn accept(&self) -> impl Future<Output = Stream> + Send;
}
