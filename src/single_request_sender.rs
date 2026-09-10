use std::marker::PhantomData;

use crate::{CallError, NotifyError};

// TODO consider better name
pub struct SingleRequestSender<ResponseType> {
    // TODO maybe we use a lifetime here
    _method_path: String,
    _response_type: PhantomData<fn() -> ResponseType>,
}

impl<ResponseType> SingleRequestSender<ResponseType> {
    pub async fn call() -> Result<ResponseType, CallError> {
        todo!()
    }

    pub async fn notify() -> Result<(), NotifyError> {
        todo!()
    }
}
