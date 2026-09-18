use std::sync::{Arc, Weak};

use tokio::net::{TcpListener, TcpStream, ToSocketAddrs};

use crate::{
    HandleRequest, IntoRequestHandler, SubServicesFromState, SubscribableStub,
    client::{self, ClientHandle},
    server, stream_handler,
};

pub struct Tcp<const MAX_FRAME_SIZE: usize> {}

impl<const MAX_FRAME_SIZE: usize> Tcp<MAX_FRAME_SIZE> {
    pub async fn start_server<
        Addr,
        Server,
        RequestHandler,
        ClientStubArcHandle,
        ClientStubWeakHandle,
        SubServices,
    >(
        addr: Addr,
        server_state: Arc<Server>,
    ) -> Result<server::ServerHandle<ClientStubArcHandle>, server::Error>
    where
        Addr: ToSocketAddrs + Into<String>,
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
            Addr,
            TcpStream,
            Server,
            RequestHandler,
            ClientStubArcHandle,
            ClientStubWeakHandle,
            SubServices,
        >(addr, server_state)
        .await
    }

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
