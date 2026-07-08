use std::sync::Arc;

pub use rpc_genie_macros::service;

pub enum Error {
    MethodNotFound,
    FailedToDeserializeArg { details: String },
}

pub type Result<T, E = Error> = core::result::Result<T, E>;

#[doc(hidden)]
#[allow(dead_code)] // TODO remove allow dead_code
pub struct RequestHandler<'state, State, SubServices> {
    service_path: Option<Arc<String>>,
    pub state: &'state State,
    pub sub_services: SubServices,
}

#[doc(hidden)]
#[allow(async_fn_in_trait)]
pub trait HandleRequest<Stub> {
    async fn handle_request(
        &self,
        method_name: &str,
        __rpc_stub__: Stub,
        __rpc_args__: Args,
    ) -> Result<ReturnValue>;
}

#[doc(hidden)]
pub struct Args {}

impl Args {
    pub fn read_arg<Arg>(&mut self) -> Result<Arg> {
        todo!()
    }
}

#[doc(hidden)]
pub struct ReturnValue {}

impl ReturnValue {
    pub fn new<T>(_value: T) -> Self {
        todo!()
    }
}
