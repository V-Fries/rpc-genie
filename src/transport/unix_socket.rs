use std::sync::{Arc, Weak};

use tokio::net::{UnixListener, UnixStream};

use crate::{
    HandleRequest, IntoRequestHandler, SubServicesFromState, SubscribableStub,
    client::{self, ClientHandle},
    server, stream_handler,
};

/// Unix-domain socket transport for RPC clients and servers.
///
/// `MAX_FRAME_SIZE` limits the serialized size of each RPC frame. Use the same
/// limit for both ends of a connection.
pub struct UnixSocket<const MAX_FRAME_SIZE: usize> {}

impl<const MAX_FRAME_SIZE: usize> UnixSocket<MAX_FRAME_SIZE> {
    /// Bind a Unix-domain socket and start serving requests in the background.
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
    ///     let _result = rpc_genie::UnixSocket::<MAX_FRAME_SIZE>::start_server(
    ///         "socket_file_path.sock",
    ///         Arc::new(service::Server {
    ///             _request_sender: PhantomData
    ///         })
    ///     ).await;
    /// }
    /// ```
    pub async fn start_server<
        Path,
        Server,
        RequestHandler,
        ClientStubArcHandle,
        ClientStubWeakHandle,
        SubServices,
    >(
        socket_path: Path,
        server_state: Arc<Server>,
    ) -> Result<server::ServerHandle<Server, ClientStubArcHandle>, server::Error>
    where
        Path: AsRef<std::path::Path> + Into<String>,
        Server: crate::Server<
                MAX_FRAME_SIZE,
                UnixStream,
                Weak<stream_handler::Handle<MAX_FRAME_SIZE, UnixStream>>,
                ClientStubArcHandle = ClientStubArcHandle,
            > + IntoRequestHandler<RequestHandler, SubServices>,
        RequestHandler: HandleRequest<ClientStubWeakHandle>,
        ClientStubArcHandle:
            crate::Stub<Arc<stream_handler::Handle<MAX_FRAME_SIZE, UnixStream>>> + SubscribableStub,
        ClientStubWeakHandle: crate::Stub<Weak<stream_handler::Handle<MAX_FRAME_SIZE, UnixStream>>>
            + SubscribableStub,
        SubServices: SubServicesFromState<Server>,
    {
        server::start_server::<
            MAX_FRAME_SIZE,
            UnixListener,
            Path,
            UnixStream,
            Server,
            RequestHandler,
            ClientStubArcHandle,
            ClientStubWeakHandle,
            SubServices,
        >(socket_path, server_state)
        .await
    }

    /// Connect to a Unix-domain socket server and start the client request routine.
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
    ///     let _result = rpc_genie::UnixSocket::<MAX_FRAME_SIZE>::connect_client(
    ///         "socket_file_path.sock",
    ///         Arc::new(service::Client {
    ///             _request_sender: PhantomData
    ///         })
    ///     ).await;
    /// }
    /// ```
    pub async fn connect_client<
        Path,
        Client,
        RequestHandler,
        ServerStubArcHandle,
        ServerStubWeakHandle,
        SubServices,
    >(
        socket_path: Path,
        client_state: Arc<Client>,
    ) -> Result<ClientHandle<ServerStubArcHandle, Client>, client::Error>
    where
        Path: AsRef<std::path::Path> + Into<String>,
        Client: crate::Client<
                MAX_FRAME_SIZE,
                UnixStream,
                Weak<stream_handler::Handle<MAX_FRAME_SIZE, UnixStream>>,
                ServerStubArcHandle = ServerStubArcHandle,
            > + IntoRequestHandler<RequestHandler, SubServices>,
        RequestHandler: HandleRequest<ServerStubWeakHandle>,
        ServerStubArcHandle: crate::Stub<Arc<stream_handler::Handle<MAX_FRAME_SIZE, UnixStream>>>,
        ServerStubWeakHandle: crate::Stub<Weak<stream_handler::Handle<MAX_FRAME_SIZE, UnixStream>>>
            + SubscribableStub,
        SubServices: SubServicesFromState<Client>,
    {
        client::connect_client::<
            MAX_FRAME_SIZE,
            Path,
            UnixStream,
            Client,
            RequestHandler,
            ServerStubArcHandle,
            ServerStubWeakHandle,
            SubServices,
        >(socket_path, client_state)
        .await
    }
}
