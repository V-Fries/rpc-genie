#![doc = include_str!("../../README.md")]

mod single_request_sender;
pub use single_request_sender::SingleRequestSender;

#[doc(hidden)]
pub mod stream_handler;

#[doc(hidden)]
pub mod send_request;
#[doc(hidden)]
pub use send_request::SendRequest;

#[doc(hidden)]
pub mod topic;
pub use topic::Topic;

mod transport;
pub use transport::{Tcp, UnixSocket};

#[doc(hidden)]
pub mod frame;

pub mod client;
pub mod server;

use std::sync::Arc;

/// Generate RPC handlers, client stubs, and state wiring for a service module.
///
/// The annotated module must declare `Server<RequestSender>` and `Client<RequestSender>` structs.
/// Methods marked with `#[remote_method]` are exposed to the opposite side of the connection.
/// Use `sub_services!` inside the module to compose services.
///
/// # Example:
/// ```rust
/// // The example module is not necessary in your code. It is here due to a limitation in doc tests
/// mod example {
///     #[rpc_genie::service]
///     mod service_a {
///         // Define other services to include in the current service
///         sub_services! {
///             // Can have any number of sub services
///             printer: super::printer,
///         }
///
///         struct Server<RequestSender> {
///             // Server will have one field per sub_services added automatically
///             server_name: String,
///         }
///
///         impl<RequestSender> Server<RequestSender> {
///             // The #[remote_method] attribute marks this method as being callable from the
///             // client
///             #[remote_method]
///             fn get_server_name(&self) -> String {
///                 self.server_name.clone()
///             }
///
///             #[remote_method]
///             async fn foo(client: ClientStub) {
///                 // When an argument has the type ClientStub, it allows to call remote methods
///                 // on the client.
///                 let _addition_result = client.add(4, 2).call().await.unwrap();
///
///                 // sub services can also be accessed through the stub
///                 client.printer.print("foo".to_owned()).notify().await.unwrap();
///             }
///         }
///
///         struct Client<RequestSender> {
///             // Client will have one field per sub_services added automatically
///         }
///
///         impl<RequestSender> Client<RequestSender> {
///             // Clients can also have methods with #[remote_method], making the method callable
///             // from the server
///             #[remote_method]
///             fn add(a: u32, b: u32) -> u32 {
///                 a + b
///             }
///
///             #[remote_method]
///             async fn bar(server: ServerStub) {
///                 // The client can also get a stub to the server as an argument to a remote
///                 // method
///                 let _server_name = server.get_server_name().call().await.unwrap();
///                 server.printer.print("bar".to_owned()).notify().await.unwrap();
///             }
///         }
///     }
///
///     #[rpc_genie::service]
///     mod printer {
///         pub struct Server<RequestSender> {}
///
///         impl<RequestSender> Server<RequestSender> {
///             #[remote_method]
///             pub fn print(msg: String) {
///                 println!("{msg}")
///             }
///         }
///
///         pub struct Client<RequestSender> {}
///
///         impl<RequestSender> Client<RequestSender> {
///             #[remote_method]
///             pub fn print(msg: String) {
///                 println!("{msg}")
///             }
///         }
///     }
/// }
/// ```
pub use rpc_genie_macros::service;

use crate::{
    frame::{
        rpc_request::RpcRequestArgReader,
        rpc_response::{self, RpcResponseBuilder},
    },
    stream_handler::StreamId,
};

#[derive(Debug, thiserror::Error)]
pub enum CallError {
    #[error("Stream handler routine is stopped")]
    RoutineIsStopped(#[from] stream_handler::StopReason),
    #[error("Failed to write frame")]
    FailedToWriteFrame(#[from] frame::WriteError),
    #[error("Method not found")]
    MethodNotFound,
    #[error("Failed to deserialize args: {deserialize_error}")]
    FailedToDeserializeArg { deserialize_error: String },
    #[error("Failed to deserialize response: {deserialize_error}")]
    FailedToDeserializeResponse { deserialize_error: String },
}

#[derive(Debug, thiserror::Error)]
pub enum NotifyError {
    #[error("Stream handler routine is stopped")]
    RoutineIsStopped(#[from] stream_handler::StopReason),
    #[error("Failed to write frame")]
    FailedToWriteFrame(#[from] frame::WriteError),
}

#[doc(hidden)]
#[allow(async_fn_in_trait)]
pub trait HandleRequest<OppositeStubWeakHandle>: Sync + Send + 'static {
    fn handle_request(
        &self,
        method_path: &str,
        __rpc_opposite_stub_weak_handle__: &Arc<OppositeStubWeakHandle>,
        __rpc_request_arg_reader__: RpcRequestArgReader,
    ) -> impl Future<
        Output = RpcResponseBuilder<
            rpc_response::builder::Uninit,
            rpc_response::builder::ResponseInit,
        >,
    > + Send;
}

#[doc(hidden)]
pub trait IntoRequestHandler<RequestHandler, SubServices: SubServicesFromState<Self>>
where
    Self: Sized,
{
    fn into_request_handler(self: Arc<Self>, service_path: Option<String>) -> Arc<RequestHandler>;
}

#[doc(hidden)]
pub trait SubServicesFromState<State> {
    fn from_state(state: Arc<State>, service_path: Option<&str>) -> Self;
}

#[doc(hidden)]
pub trait State: Send + Sync + 'static {}

/// Every Client structs in rpc-genie services will automatically implement this trait.
///
/// It is used to identify a struct as a Client to check at compile time that you don't pass a
/// client state to a function that expects a server state.
pub trait Client<const MAX_FRAME_SIZE: usize, Stream, ServerStubWeakHandle>: State {
    /// The generated, owning stub used to make requests to the server.
    type ServerStubArcHandle;
}

/// Every Server structs in rpc-genie services will automatically implement this trait.
///
/// It is used to identify a struct as a Server to check at compile time that you don't pass a
/// server state to a function that expects a client state.
pub trait Server<const MAX_FRAME_SIZE: usize, Stream, ClientStubWeakHandle>: State {
    /// The generated, owning stub used to make requests to a connected client.
    type ClientStubArcHandle;
}

#[doc(hidden)]
pub trait Stub<RequestSender>
where
    Self: Clone + Send + 'static + Sync,
    RequestSender: send_request::SendRequest,
{
    fn new(request_sender: RequestSender, service_path: Option<String>) -> Self;
}

#[doc(hidden)]
pub trait SubscribableStub: Sized {
    /// Returns false is the stub is already dead, true otherwise
    fn add_registered_topic(
        &self,
        topic_id: topic::TopicId,
        stub_died_notification_sender: topic::StubDiedNotificationSender,
    ) -> impl Future<Output = bool> + Send;

    fn remove_registered_topic(&self, topic_id: topic::TopicId);

    /// Returns None if the stream identity is unavailable, Some(stream_id) otherwise
    fn stream_id(&self) -> Option<StreamId>;
}
