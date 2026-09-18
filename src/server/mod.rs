mod listener;
pub use listener::Listener;

use tokio::{
    io::{AsyncRead, AsyncWrite},
    task::{JoinError, JoinHandle},
};

use std::sync::{Arc, Weak};

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

pub struct ServerHandle<Stub>
where
    Stub: SubscribableStub,
{
    join_handle: Option<JoinHandle<()>>,
    topic: Arc<Topic<Stub>>,
}

impl<Stub> Drop for ServerHandle<Stub>
where
    Stub: SubscribableStub,
{
    fn drop(&mut self) {
        if let Some(join_handle) = self.join_handle.take() {
            join_handle.abort();
        }
    }
}

impl<Stub> ServerHandle<Stub>
where
    Stub: SubscribableStub,
{
    pub async fn wait_until_stopped(mut self) -> Result<(), JoinError> {
        self.join_handle
            .take()
            .expect("join_handle should never be None")
            .await
    }

    pub fn stop(self) {
        // routine will be aborted automatically on drop
    }
}

impl<Stub> ServerHandle<Stub>
where
    Stub: SubscribableStub,
{
    pub async fn map_each_client<CallbackFuture, FutureOutput>(
        &self,
        callback: impl FnMut(Arc<Stub>) -> CallbackFuture,
    ) -> Vec<FutureOutput>
    where
        CallbackFuture: Future<Output = FutureOutput>,
    {
        self.topic.map(callback).await
    }

    pub async fn for_each_client<CallbackFuture>(
        &self,
        callback: impl FnMut(Arc<Stub>) -> CallbackFuture,
    ) where
        CallbackFuture: Future<Output = ()> + Send,
    {
        self.topic.for_each(callback).await
    }
}

pub async fn start_server<
    const MAX_FRAME_SIZE: usize,
    Listener,
    Addr,
    Stream,
    Server,
    RequestHandler,
    ClientStubArcHandle,
    ClientStubWeakHandle,
    SubServices,
>(
    addr: Addr,
    server_state: Arc<Server>,
) -> Result<ServerHandle<ClientStubArcHandle>, Error>
where
    Listener: listener::Listener<Addr, Stream>,
    Addr: Into<String>,
    Stream: AsyncWrite + AsyncRead + Send + 'static,
    Server: crate::Server<
            MAX_FRAME_SIZE,
            Stream,
            Weak<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>,
            ClientStubArcHandle = ClientStubArcHandle,
        > + IntoRequestHandler<RequestHandler, SubServices>,
    RequestHandler: HandleRequest<ClientStubWeakHandle>,
    ClientStubArcHandle:
        crate::Stub<Arc<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>> + SubscribableStub,
    ClientStubWeakHandle:
        crate::Stub<Weak<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>> + SubscribableStub,
    SubServices: SubServicesFromState<Server>,
{
    let listener = Listener::bind(&addr)
        .await
        .map_err(|error| Error::BindListener {
            addr: addr.into(),
            source: error,
        })?;

    let topic = Arc::new(Topic::new().await);

    let topic_weak_ref = Arc::downgrade(&topic);
    let join_handle = tokio::spawn(async move {
        server_routine::<
            MAX_FRAME_SIZE,
            Listener,
            Addr,
            Server,
            Stream,
            RequestHandler,
            ClientStubArcHandle,
            ClientStubWeakHandle,
            SubServices,
        >(listener, server_state, topic_weak_ref)
        .await
    });

    Ok(ServerHandle {
        join_handle: Some(join_handle),
        topic,
    })
}

pub async fn server_routine<
    const MAX_FRAME_SIZE: usize,
    Listener,
    Addr,
    Server,
    Stream,
    RequestHandler,
    ClientStubArcHandle,
    ClientStubWeakHandle,
    SubServices,
>(
    listener: Listener,
    server_state: Arc<Server>,
    topic: Weak<Topic<ClientStubArcHandle>>,
) where
    Listener: listener::Listener<Addr, Stream>,
    Server: crate::Server<
            MAX_FRAME_SIZE,
            Stream,
            Weak<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>,
            ClientStubArcHandle = ClientStubArcHandle,
        > + IntoRequestHandler<RequestHandler, SubServices>,
    RequestHandler: HandleRequest<ClientStubWeakHandle>,
    ClientStubArcHandle:
        crate::Stub<Arc<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>> + SubscribableStub,
    ClientStubWeakHandle:
        crate::Stub<Weak<stream_handler::Handle<MAX_FRAME_SIZE, Stream>>> + SubscribableStub,
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
                    MAX_FRAME_SIZE,
                    Stream,
                    RequestHandler,
                    ClientStubArcHandle,
                    ClientStubWeakHandle,
                >(client_stream, client_id, request_handler_clone, weak_topic)
                .await,
            )
            .await
    }
}
