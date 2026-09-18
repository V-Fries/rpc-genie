mod stream;
pub use stream::Stream;

use std::sync::{Arc, Weak};

use tokio::io::{AsyncRead, AsyncWrite};

use crate::{
    HandleRequest, IntoRequestHandler, SubServicesFromState,
    stream_handler::{self, StreamId},
};

pub struct ClientHandle<Stub, Client> {
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

pub async fn connect_client<
    const MAX_FRAME_SIZE: usize,
    Addr,
    Stream,
    Client,
    RequestHandler,
    ServerStubArcHandle,
    ServerStubWeakHandle,
    SubServices,
>(
    addr: Addr,
    client_state: Arc<Client>,
) -> Result<ClientHandle<ServerStubArcHandle, Client>, Error>
where
    Stream: stream::Stream<Addr> + AsyncWrite + AsyncRead + Send + 'static,
    Addr: Into<String>,
    Client: crate::Client<
            MAX_FRAME_SIZE,
            Stream,
            Weak<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>,
            ServerStubArcHandle = ServerStubArcHandle,
        > + IntoRequestHandler<RequestHandler, SubServices>,
    RequestHandler: HandleRequest<ServerStubWeakHandle>,
    ServerStubArcHandle: crate::Stub<Arc<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>>,
    ServerStubWeakHandle: crate::Stub<Weak<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>>,
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
        server_stub: ServerStubArcHandle::new(
            stream_handler::spawn_routine::<
                MAX_FRAME_SIZE,
                Stream,
                RequestHandler,
                ServerStubWeakHandle,
            >(stream, stream_id, request_handler)
            .await,
            None,
        ),
        state: client_state,
    })
}
