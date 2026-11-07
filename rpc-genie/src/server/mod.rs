mod listener;
pub use listener::Listener;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    sync::mpsc,
    task::JoinHandle,
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

type ServerListenRoutineShutdownMsgSender = mpsc::UnboundedSender<()>;
type ServerListenRoutineShutdownMsgReceiver = mpsc::UnboundedReceiver<()>;

struct ServerListenRoutineJoinHandle {
    raw_join_handle: JoinHandle<()>,
    shutdown_msg_sender: ServerListenRoutineShutdownMsgSender,
}

impl ServerListenRoutineJoinHandle {
    const LISTEN_ROUTINE_IS_DOWN_ERROR_MSG: &'static str = "server listen routine is down";

    fn stop(self) {
        self.shutdown_msg_sender
            .send(())
            .expect(Self::LISTEN_ROUTINE_IS_DOWN_ERROR_MSG)
    }

    async fn wait_until_stopped(self) {
        self.raw_join_handle
            .await
            .expect(Self::LISTEN_ROUTINE_IS_DOWN_ERROR_MSG)
    }

    async fn stop_and_join(self) {
        self.shutdown_msg_sender
            .send(())
            .expect(Self::LISTEN_ROUTINE_IS_DOWN_ERROR_MSG);

        self.raw_join_handle
            .await
            .expect(Self::LISTEN_ROUTINE_IS_DOWN_ERROR_MSG)
    }
}

/// Handle to a running RPC server.
///
/// Dropping the `ServerHandle` requests shutdown of the server listener task and disconnects
/// connected clients. Use [`ServerHandle::stop`] to request shutdown and wait for the listener
/// task to exit.
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
///     pub struct Server<Stream> {}
///     pub struct Client<Stream> {}
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
///                 _stream: PhantomData,
///             }
///         ))
///         .await
///         .unwrap();
/// }
/// ```
#[must_use = "The server stops when it is dropped"]
pub struct ServerHandle<ServerState, Stub, Listener>
where
    Stub: SubscribableStub,
    Listener: listener::Listener,
{
    join_handle: Option<ServerListenRoutineJoinHandle>,
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
            join_handle.stop();
        }
        Listener::on_routine_end(&self.addr)
    }
}

impl<ServerState, Stub, Listener, Stream> ServerHandle<ServerState, Stub, Listener>
where
    Stub: SubscribableStub,
    Listener: listener::Listener<Stream = Stream>,
{
    /// Wait for the server listener task to stop.
    pub async fn wait_until_stopped(mut self) {
        self.join_handle
            .take()
            .expect("join_handle should never be None")
            .wait_until_stopped()
            .await
    }

    /// Requests that the server stop accepting new clients, then waits for its listener task to
    /// exit. Dropping the handle also disconnects connected clients.
    pub async fn stop(mut self) {
        self.join_handle
            .take()
            .expect("join_handle should never be None")
            .stop_and_join()
            .await
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
/// [`tcp::start_server`](crate::tcp::start_server) /
/// [`unix_socket::start_server`](crate::unix_socket::start_server) instead.
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
    Server: crate::Server<Stream, ClientStubArcHandle = ClientStubArcHandle>
        + IntoRequestHandler<RequestHandler, SubServices>,
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

    let (shutdown_msg_sender, shutdown_msg_receiver) = mpsc::unbounded_channel();

    let topic_weak_ref = Arc::downgrade(&topic);
    let server_state_clone = Arc::clone(&server_state);
    let raw_join_handle = tokio::spawn(async move {
        server_listen_routine::<
            Listener,
            Server,
            Stream,
            RequestHandler,
            ClientStubArcHandle,
            ClientStubWeakHandle,
            SubServices,
        >(
            listener,
            shutdown_msg_receiver,
            max_frame_size,
            server_state_clone,
            topic_weak_ref,
        )
        .await
    });

    Ok(ServerHandle {
        join_handle: Some(ServerListenRoutineJoinHandle {
            raw_join_handle,
            shutdown_msg_sender,
        }),
        addr: addr.to_owned(),
        topic,
        state: server_state,
        _listener: PhantomData,
    })
}

async fn server_listen_routine<
    Listener,
    Server,
    Stream,
    RequestHandler,
    ClientStubArcHandle,
    ClientStubWeakHandle,
    SubServices,
>(
    listener: Listener,
    mut shutdown_msg_receiver: ServerListenRoutineShutdownMsgReceiver,
    max_frame_size: usize,
    server_state: Arc<Server>,
    topic: Weak<Topic<ClientStubArcHandle>>,
) where
    Listener: listener::Listener<Stream = Stream>,
    Server: crate::Server<Stream, ClientStubArcHandle = ClientStubArcHandle>
        + IntoRequestHandler<RequestHandler, SubServices>,
    RequestHandler: HandleRequest<ClientStubWeakHandle>,
    ClientStubArcHandle: crate::Stub<Arc<stream_handler::Handle<Stream>>> + SubscribableStub,
    ClientStubWeakHandle: crate::Stub<Weak<stream_handler::Handle<Stream>>> + SubscribableStub,
    SubServices: SubServicesFromState<Server>,
    Stream: AsyncWrite + AsyncRead + Send + 'static,
{
    let request_handler = server_state.into_request_handler(None);

    loop {
        let client_stream = tokio::select! {
            biased;

            _ = shutdown_msg_receiver.recv() => break,

            client_stream = listener.accept() => client_stream,
        };

        let Some(topic) = topic.upgrade() else {
            break;
        };

        let request_handler_clone = Arc::clone(&request_handler);

        let client_id = StreamId::next();
        // TODO use a logging crate
        // println!("Client connected: {}", client_id);

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
