use std::sync::{Arc, Weak};

use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};

use crate::{
    HandleRequest, IntoRequestHandler, SubServicesFromState, SubscribableStub,
    client::{self, ClientHandle},
    server, stream_handler,
};

/// TCP transport for RPC clients and servers.
///
/// `MAX_FRAME_SIZE` limits the serialized size of each RPC frame. Use the same
/// limit for both ends of a connection.
pub struct Tcp<const MAX_FRAME_SIZE: usize> {}

impl<const MAX_FRAME_SIZE: usize> Tcp<MAX_FRAME_SIZE> {
    /// Bind a TCP listener and start serving requests in the background.
    ///
    /// The returned [`server::ServerHandle`] owns the server task. Dropping it
    /// stops the task; use `wait_until_stopped` to keep the server running.
    ///
    /// # Example
    /// ```rust
    /// use std::{sync::Arc, marker::PhantomData};
    ///
    /// #[rpc_genie::service]
    /// mod service {
    ///     pub struct Server<RequestSender> {}
    ///     pub struct Client<RequestSender> {}
    /// }
    ///
    /// const MAX_FRAME_SIZE: usize = 1024 * 1024;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let _server_handle = rpc_genie::Tcp::<MAX_FRAME_SIZE>::start_server(
    ///         "127.0.0.1:12345",
    ///         Arc::new(service::Server {
    ///             _request_sender: PhantomData
    ///         })
    ///     ).await;
    /// }
    /// ```
    pub async fn start_server<
        Server,
        RequestHandler,
        ClientStubArcHandle,
        ClientStubWeakHandle,
        SubServices,
    >(
        addr: &str,
        server_state: Arc<Server>,
    ) -> Result<server::ServerHandle<Server, ClientStubArcHandle, TcpListener>, server::Error>
    where
        Server: crate::Server<
                MAX_FRAME_SIZE,
                TcpStream,
                Weak<stream_handler::Handle<MAX_FRAME_SIZE, TcpStream>>,
                ClientStubArcHandle = ClientStubArcHandle,
            > + IntoRequestHandler<RequestHandler, SubServices>,
        RequestHandler: HandleRequest<ClientStubWeakHandle>,
        ClientStubArcHandle:
            crate::Stub<Arc<stream_handler::Handle<MAX_FRAME_SIZE, TcpStream>>> + SubscribableStub,
        ClientStubWeakHandle:
            crate::Stub<Weak<stream_handler::Handle<MAX_FRAME_SIZE, TcpStream>>> + SubscribableStub,
        SubServices: SubServicesFromState<Server>,
    {
        server::start_server::<
            MAX_FRAME_SIZE,
            TcpListener,
            TcpStream,
            Server,
            RequestHandler,
            ClientStubArcHandle,
            ClientStubWeakHandle,
            SubServices,
        >(addr, server_state)
        .await
    }

    /// Connect to a TCP server and start the client request routine.
    ///
    /// # Example
    /// ```rust
    /// use std::{sync::Arc, marker::PhantomData};
    ///
    /// #[rpc_genie::service]
    /// mod service {
    ///     pub struct Server<RequestSender> {}
    ///     pub struct Client<RequestSender> {}
    /// }
    ///
    /// const MAX_FRAME_SIZE: usize = 1024 * 1024;
    ///
    /// #[tokio::main]
    /// async fn main() {
    ///     let _client_handle = rpc_genie::Tcp::<MAX_FRAME_SIZE>::connect_client(
    ///         "127.0.0.1:12345",
    ///         Arc::new(service::Client {
    ///             _request_sender: PhantomData
    ///         })
    ///     ).await;
    /// }
    /// ```
    pub async fn connect_client<
        Addr,
        Client,
        RequestHandler,
        ServerStubArcHandle,
        ServerStubWeakHandle,
        SubServices,
    >(
        addr: Addr,
        client_state: Arc<Client>,
    ) -> Result<ClientHandle<ServerStubArcHandle, Client>, client::Error>
    where
        Addr: ToSocketAddrs + Into<String>,
        Client: crate::Client<
                MAX_FRAME_SIZE,
                TcpStream,
                Weak<stream_handler::Handle<MAX_FRAME_SIZE, TcpStream>>,
                ServerStubArcHandle = ServerStubArcHandle,
            > + IntoRequestHandler<RequestHandler, SubServices>,
        RequestHandler: HandleRequest<ServerStubWeakHandle>,
        ServerStubArcHandle: crate::Stub<Arc<stream_handler::Handle<MAX_FRAME_SIZE, TcpStream>>>,
        ServerStubWeakHandle:
            crate::Stub<Weak<stream_handler::Handle<MAX_FRAME_SIZE, TcpStream>>> + SubscribableStub,
        SubServices: SubServicesFromState<Client>,
    {
        client::connect_client::<
            MAX_FRAME_SIZE,
            Addr,
            TcpStream,
            Client,
            RequestHandler,
            ServerStubArcHandle,
            ServerStubWeakHandle,
            SubServices,
        >(addr, client_state)
        .await
    }
}
