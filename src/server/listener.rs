use std::io;

pub trait Listener<Addr, Stream>: Sized + Send + 'static {
    fn bind(addr: &Addr) -> impl Future<Output = Result<Self, io::Error>>;

    fn accept(&self) -> impl Future<Output = Stream> + Send;
}
