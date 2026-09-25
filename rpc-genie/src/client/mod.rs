mod stream;
pub use stream::Stream;

use std::sync::{Arc, Weak};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    HandleRequest, IntoRequestHandler, SubServicesFromState,
    stream_handler::{self, StreamId},
};

// TODO it'd be nice if server_stub was private and we generated a Trait for each Server Stubs that
// is automatically implemented for ClientHandle<StubTypeSpecificToThisTrait, ...>, it would just
// route the functions calls to self.server_stub.method_name()
/// The live client connection and the client-side application state.
///
/// The [`service`](crate::service) macro generates an alias for the current service so you never
/// have to type out the full type.
///
/// # Examples
/// ```rust
/// use std::{marker::PhantomData, sync::Arc};
/// use tokio::net::TcpStream;
///
/// #[rpc_genie::service]
/// mod service {
///     pub struct Server<Stream> {}
///     pub struct Client<Stream> {}
/// }
///
/// const MAX_FRAME_SIZE: usize = 1024;
///
/// #[tokio::main]
/// async fn main() {
///     let result: Result<
///             // use the generated alias not the original ClientHandle type (it's way too long)
///             service::ClientHandle<TcpStream>,
///             rpc_genie::client::Error,
///         > =
///         rpc_genie::client::connect_client(
///             "127.0.0.1:12323",
///             MAX_FRAME_SIZE,
///             Arc::new(service::Client {
///                 _stream: PhantomData,
///             })
///         ).await;
/// }
/// ```
pub struct ClientHandle<Stub, Client> {
    /// Stub used to invoke methods on the server.
    pub server_stub: Stub,
    pub state: Arc<Client>,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to connect to \"{addr}\"")]
    ConnectError {
        addr: String,
        source: std::io::Error,
    },
}

impl<Stub, Client> ClientHandle<Stub, Client> {
    /// Disconnect from the server.
    ///
    /// Dropping the `ClientHandle` has the same effect as calling this method.
    pub fn disconnect(self) {
        // Dropping the server_stub field will disconnect automatically so no need to do anything
    }
}

/// Connect using a stream implementation and start the client request routine.
///
/// You probably never want to call this directly, use
/// [`Tcp::connect_client`](crate::Tcp::connect_client) /
/// [`UnixSocket::connect_client`](crate::UnixSocket::connect_client) instead.
///
/// This function is only useful to call manually if you plan on implementing the
/// [`Listener`](crate::server::Listener) and [`Stream`] traits for a custom transport.
pub async fn connect_client<
    Addr,
    Stream,
    Client,
    RequestHandler,
    ServerStubArcHandle,
    ServerStubWeakHandle,
    SubServices,
>(
    addr: Addr,
    max_frame_size: usize,
    client_state: Arc<Client>,
) -> Result<ClientHandle<ServerStubArcHandle, Client>, Error>
where
    Stream: stream::Stream<Addr> + AsyncWrite + AsyncRead + Send + 'static,
    Addr: Into<String>,
    Client: crate::Client<Stream, ServerStubArcHandle = ServerStubArcHandle>
        + IntoRequestHandler<RequestHandler, SubServices>,
    RequestHandler: HandleRequest<ServerStubWeakHandle>,
    ServerStubArcHandle: crate::Stub<Arc<stream_handler::Handle<Stream>>>,
    ServerStubWeakHandle: crate::Stub<Weak<stream_handler::Handle<Stream>>>,
    SubServices: SubServicesFromState<Client>,
{
    let stream = Stream::connect(&addr)
        .await
        .map_err(|err| Error::ConnectError {
            addr: addr.into(),
            source: err,
        })?;
    let stream_id = StreamId::next();
    let request_handler = Arc::clone(&client_state).into_request_handler(None);

    Ok(ClientHandle {
        server_stub: stream_handler::spawn_client_routine::<
            Stream,
            RequestHandler,
            ServerStubArcHandle,
            ServerStubWeakHandle,
        >(max_frame_size, stream, stream_id, request_handler)
        .await,
        state: client_state,
    })
}
