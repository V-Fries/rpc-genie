pub trait Stream<Addr>: Sized {
    async fn connect(addr: &Addr) -> Result<Self, std::io::Error>;
}
