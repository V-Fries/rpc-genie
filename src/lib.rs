use std::{marker::PhantomData, sync::Arc};

pub use rpc_genie_macros::service;

// TODO consider #[doc(hidden)]
pub enum Error {
    MethodNotFound,
    FailedToDeserializeArg { details: String },
}

// TODO consider #[doc(hidden)]
pub type Result<T, E = Error> = core::result::Result<T, E>;

#[doc(hidden)]
#[allow(dead_code)] // TODO remove allow dead_code
pub struct RequestHandler<'state, State, SubServices, Handle> {
    service_path: Arc<String>,
    pub state: &'state State,
    pub sub_services: SubServices,
    _handle: PhantomData<Handle>,
}

#[doc(hidden)]
#[allow(async_fn_in_trait)]
pub trait HandleRequest<Handle> {
    async fn handle_request(
        &self,
        method_name: &str,
        __rpc_handle__: Handle,
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

