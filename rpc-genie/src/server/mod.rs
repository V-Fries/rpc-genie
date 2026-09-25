mod listener;
pub use listener::Listener;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    task::{JoinError, JoinHandle},
};

use std::{
    marker::PhantomData,
    sync::{Arc, Weak},
};

use crate::{
    HandleRequest, IntoRequestHandler, SubServicesFromState, SubscribableStub, Topic,
    stream_handler::{self, StreamId},
};

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Failed to bind listener to \"{addr}\"")]
    BindListener {
        addr: String,
        source: std::io::Error,
    },
}

/// Handle to a running RPC server.
///
/// Dropping the `ServerHandle` aborts the server task and disconnects its clients.
///
/// The [`service`](crate::service) macro generates an alias for the current service so you never
/// have to type out the full type.
///
/// # Examples
/// ```rust
/// use std::{marker::PhantomData, sync::Arc};
/// use tokio::net::{TcpStream, TcpListener};
///
/// #[rpc_genie::service]
/// mod service {
///     pub struct Server<RequestSender> {}
///     pub struct Client<RequestSender> {}
/// }
///
/// const MAX_FRAME_SIZE: usize = 1024;
///
/// #[tokio::main]
/// async fn main() {
///     // use the generated alias not the original ServerHandle type (it's way too long)
///     let server_handle: service::ServerHandle<TcpListener> =
///         rpc_genie::server::start_server(
///             "127.0.0.1:12323",
///             MAX_FRAME_SIZE,
///             Arc::new(service::Server {
///                 _request_sender: PhantomData,
///             }
///         ))
///         .await
///         .unwrap();
/// }
/// ```
pub struct ServerHandle<ServerState, Stub, Listener>
where
    Stub: SubscribableStub,
    Listener: listener::Listener,
{
    join_handle: Option<JoinHandle<()>>,
    addr: String,
    topic: Arc<Topic<Stub>>,
    pub state: Arc<ServerState>,
    _listener: PhantomData<fn() -> Listener>,
}

impl<ServerState, Stub, Listener, Stream> Drop for ServerHandle<ServerState, Stub, Listener>
where
    Stub: SubscribableStub,
    Listener: listener::Listener<Stream = Stream>,
{
    fn drop(&mut self) {
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.abort();
        }
        Listener::on_routine_end(&self.addr)
    }
}

impl<ServerState, Stub, Listener, Stream> ServerHandle<ServerState, Stub, Listener>
where
    Stub: SubscribableStub,
    Listener: listener::Listener<Stream = Stream>,
{
    /// Wait for the server task to stop and return its join result.
    pub async fn wait_until_stopped(mut self) -> Result<(), JoinError> {
        self.join_handle
            .take()
            .expect("join_handle should never be None")
            .await
    }

    /// Stops the server.
    ///
    /// Dropping the `ServerHandle` has the same effect as calling this method.
    pub fn stop(self) {
        // routine will be aborted automatically on drop
    }
}

impl<ServerState, Stub, Listener, Stream> ServerHandle<ServerState, Stub, Listener>
where
    Stub: SubscribableStub,
    Listener: listener::Listener<Stream = Stream>,
{
    /// Run a callback for every currently connected client concurrently and
    /// collect each callback's result.
    pub async fn map_each_client<CallbackFuture, FutureOutput>(
        &self,
        callback: impl FnMut(Arc<Stub>) -> CallbackFuture,
    ) -> Vec<FutureOutput>
    where
        CallbackFuture: Future<Output = FutureOutput>,
    {
        self.topic.map(callback).await
    }

    /// Run a callback for every currently connected client concurrently,
    /// waiting until all callbacks finish.
    pub async fn for_each_client<CallbackFuture>(
        &self,
        callback: impl FnMut(Arc<Stub>) -> CallbackFuture,
    ) where
        CallbackFuture: Future<Output = ()> + Send,
    {
        self.topic.for_each(callback).await
    }
}

/// Start a server using a listener implementation and return its lifecycle handle.
///
/// You probably never want to call this directly, use
/// [`Tcp::start_server`](crate::Tcp::start_server) /
/// [`UnixSocket::start_server`](crate::UnixSocket::start_server) instead.
///
/// This function is only useful to call manually if you plan on implementing the [`Listener`] and
/// [`Stream`](crate::client::Stream) traits for a custom transport.
pub async fn start_server<
    Listener,
    Stream,
    Server,
    RequestHandler,
    ClientStubArcHandle,
    ClientStubWeakHandle,
    SubServices,
>(
    addr: &str,
    max_frame_size: usize,
    server_state: Arc<Server>,
) -> Result<ServerHandle<Server, ClientStubArcHandle, Listener>, Error>
where
    Listener: listener::Listener<Stream = Stream>,
    Stream: AsyncWrite + AsyncRead + Send + 'static,
    Server: crate::Server<
            Stream,
            Weak<stream_handler::Handle<Stream>>,
            ClientStubArcHandle = ClientStubArcHandle,
        > + IntoRequestHandler<RequestHandler, SubServices>,
    RequestHandler: HandleRequest<ClientStubWeakHandle>,
    ClientStubArcHandle: crate::Stub<Arc<stream_handler::Handle<Stream>>> + SubscribableStub,
    ClientStubWeakHandle: crate::Stub<Weak<stream_handler::Handle<Stream>>> + SubscribableStub,
    SubServices: SubServicesFromState<Server>,
{
    let listener = Listener::bind(addr)
        .await
        .map_err(|error| Error::BindListener {
            addr: addr.to_owned(),
            source: error,
        })?;

    let topic = Arc::new(Topic::new().await);

    let topic_weak_ref = Arc::downgrade(&topic);
    let server_state_clone = Arc::clone(&server_state);
    let join_handle = tokio::spawn(async move {
        server_routine::<
            Listener,
            Server,
            Stream,
            RequestHandler,
            ClientStubArcHandle,
            ClientStubWeakHandle,
            SubServices,
        >(listener, max_frame_size, server_state_clone, topic_weak_ref)
        .await
    });

    Ok(ServerHandle {
        join_handle: Some(join_handle),
        addr: addr.to_owned(),
        topic,
        state: server_state,
        _listener: PhantomData,
    })
}

async fn server_routine<
    Listener,
    Server,
    Stream,
    RequestHandler,
    ClientStubArcHandle,
    ClientStubWeakHandle,
    SubServices,
>(
    listener: Listener,
    max_frame_size: usize,
    server_state: Arc<Server>,
    topic: Weak<Topic<ClientStubArcHandle>>,
) where
    Listener: listener::Listener<Stream = Stream>,
    Server: crate::Server<
            Stream,
            Weak<stream_handler::Handle<Stream>>,
            ClientStubArcHandle = ClientStubArcHandle,
        > + IntoRequestHandler<RequestHandler, SubServices>,
    RequestHandler: HandleRequest<ClientStubWeakHandle>,
    ClientStubArcHandle: crate::Stub<Arc<stream_handler::Handle<Stream>>> + SubscribableStub,
    ClientStubWeakHandle: crate::Stub<Weak<stream_handler::Handle<Stream>>> + SubscribableStub,
    SubServices: SubServicesFromState<Server>,
    Stream: AsyncWrite + AsyncRead + Send + 'static,
{
    let request_handler = server_state.into_request_handler(None);

    loop {
        let client_stream = listener.accept().await;

        let request_handler_clone = Arc::clone(&request_handler);

        let client_id = StreamId::next();
        println!("Client connected: {}", client_id);

        let Some(topic) = topic.upgrade() else {
            return;
        };

        let weak_topic = Arc::downgrade(&topic);
        topic
            .subscribe(
                stream_handler::spawn_server_routine::<
                    Stream,
                    RequestHandler,
                    ClientStubArcHandle,
                    ClientStubWeakHandle,
                >(
                    max_frame_size,
                    client_stream,
                    client_id,
                    request_handler_clone,
                    weak_topic,
                )
                .await,
            )
            .await
    }
}
