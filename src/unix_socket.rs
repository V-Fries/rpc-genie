use std::sync::{Arc, Weak};

use tokio::net::{UnixListener, UnixStream};

use crate::{
    HandleRequest, IntoRequestHandler, SubServicesFromState, SubscribableStub,
    client::{self, ClientHandle},
    server, stream_handler,
};

pub struct UnixSocket<const MAX_FRAME_SIZE: usize> {}

impl<const MAX_FRAME_SIZE: usize> UnixSocket<MAX_FRAME_SIZE> {
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
    ) -> Result<server::ServerHandle<ClientStubArcHandle>, server::Error>
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
