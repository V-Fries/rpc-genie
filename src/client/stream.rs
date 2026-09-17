pub trait Stream<Addr>: Sized {
    fn connect(addr: &Addr) -> impl Future<Output = Result<Self, std::io::Error>>;
}
