#[cfg(test)]
mod test;
use std::sync::{Arc, Weak};

use tokio::net::{UnixListener, UnixStream};

use crate::{
    HandleRequest, IntoRequestHandler, SubServicesFromState, SubscribableStub,
    client::{self, ClientHandle},
    server, stream_handler,
};

pub struct UnixSocketServerHandle<ServerHandle> {
    server_handle: ServerHandle,
    socket_path: String,
}

impl<ServerHandle> std::ops::Deref for UnixSocketServerHandle<ServerHandle> {
    type Target = ServerHandle;

    fn deref(&self) -> &Self::Target {
        &self.server_handle
    }
}

impl<ServerHandle> Drop for UnixSocketServerHandle<ServerHandle> {
    fn drop(&mut self) {
        if let Err(err) = std::fs::remove_file(&self.socket_path) {
            eprintln!(
                "Error: Failed to delete unix socket file \"{:?}\": {err}",
                self.socket_path
            )
        }
    }
}

/// Unix socket transport for RPC clients and servers.
///
/// `MAX_FRAME_SIZE` limits the serialized size of each RPC frame. Use the same
/// limit for both ends of a connection.
pub struct UnixSocket<const MAX_FRAME_SIZE: usize> {}

impl<const MAX_FRAME_SIZE: usize> UnixSocket<MAX_FRAME_SIZE> {
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
    ///     rpc_genie::UnixSocket::<MAX_FRAME_SIZE>::start_server(
    ///         "/tmp/rpc-genie/tests/unix_socket_start_server_doc_test.sock",
    ///         Arc::new(service::Server {
    ///             _request_sender: PhantomData
    ///         })
    ///     )
    ///     .await
    ///     .unwrap();
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
    ) -> Result<
        UnixSocketServerHandle<server::ServerHandle<Server, ClientStubArcHandle>>,
        server::Error,
    >
    where
        Path: AsRef<std::path::Path> + ToString,
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
        let socket_path_as_string = socket_path.to_string();

        let server_handle = server::start_server::<
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
        .await?;

        Ok(UnixSocketServerHandle {
            server_handle,
            socket_path: socket_path_as_string,
        })
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
    ///     let _result = rpc_genie::UnixSocket::<MAX_FRAME_SIZE>::connect_client(
    ///         "/tmp/rpc-genie/tests/unix_socket_connect_client_doc_test.sock",
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
