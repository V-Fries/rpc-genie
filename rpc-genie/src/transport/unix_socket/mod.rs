#[cfg(test)]
mod test;
use std::sync::{Arc, Weak};

use tokio::net::{UnixListener, UnixStream};

use crate::{
    HandleRequest, IntoRequestHandler, SubServicesFromState, SubscribableStub,
    client::{self, ClientHandle},
    server, stream_handler,
};

/// Bind a Unix socket and start serving requests in the background.
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
///     std::fs::create_dir_all("/tmp/rpc-genie/tests/").unwrap();
///     rpc_genie::unix_socket::start_server(
///         "/tmp/rpc-genie/tests/unix_socket_start_server_doc_test.sock",
///         MAX_FRAME_SIZE,
///         Arc::new(service::Server {
///             _request_sender: PhantomData
///         })
///     )
///     .await
///     .unwrap();
/// }
/// ```
pub async fn start_server<
    Server,
    RequestHandler,
    ClientStubArcHandle,
    ClientStubWeakHandle,
    SubServices,
>(
    socket_path: &str,
    max_frame_size: usize,
    server_state: Arc<Server>,
) -> Result<server::ServerHandle<Server, ClientStubArcHandle, UnixListener>, server::Error>
where
    Server: crate::Server<
            UnixStream,
            Weak<stream_handler::Handle<UnixStream>>,
            ClientStubArcHandle = ClientStubArcHandle,
        > + IntoRequestHandler<RequestHandler, SubServices>,
    RequestHandler: HandleRequest<ClientStubWeakHandle>,
    ClientStubArcHandle: crate::Stub<Arc<stream_handler::Handle<UnixStream>>> + SubscribableStub,
    ClientStubWeakHandle: crate::Stub<Weak<stream_handler::Handle<UnixStream>>> + SubscribableStub,
    SubServices: SubServicesFromState<Server>,
{
    server::start_server::<
        UnixListener,
        UnixStream,
        Server,
        RequestHandler,
        ClientStubArcHandle,
        ClientStubWeakHandle,
        SubServices,
    >(socket_path, max_frame_size, server_state)
    .await
}

/// Connect to a Unix socket server and start the client request routine.
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
///     let _result = rpc_genie::unix_socket::connect_client(
///         "/tmp/rpc-genie/tests/unix_socket_connect_client_doc_test.sock",
///         MAX_FRAME_SIZE,
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
    max_frame_size: usize,
    client_state: Arc<Client>,
) -> Result<ClientHandle<ServerStubArcHandle, Client>, client::Error>
where
    Path: AsRef<std::path::Path> + Into<String>,
    Client: crate::Client<
            UnixStream,
            Weak<stream_handler::Handle<UnixStream>>,
            ServerStubArcHandle = ServerStubArcHandle,
        > + IntoRequestHandler<RequestHandler, SubServices>,
    RequestHandler: HandleRequest<ServerStubWeakHandle>,
    ServerStubArcHandle: crate::Stub<Arc<stream_handler::Handle<UnixStream>>>,
    ServerStubWeakHandle: crate::Stub<Weak<stream_handler::Handle<UnixStream>>> + SubscribableStub,
    SubServices: SubServicesFromState<Client>,
{
    client::connect_client::<
        Path,
        UnixStream,
        Client,
        RequestHandler,
        ServerStubArcHandle,
        ServerStubWeakHandle,
        SubServices,
    >(socket_path, max_frame_size, client_state)
    .await
}
