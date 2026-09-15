use std::marker::PhantomData;

use crate::{
    CallError, NotifyError,
    frame::rpc_request::{RpcRequestBuilder, RpcRequestBuilderUninit},
    send_request::SendRequest,
};

pub struct SingleRequestSender<RequestSender, ResponseType> {
    request_sender: RequestSender,
    request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    _response_type: PhantomData<fn() -> ResponseType>,
}

impl<RequestSender, ResponseType> SingleRequestSender<RequestSender, ResponseType>
where
    RequestSender: SendRequest,
    ResponseType: serde::de::DeserializeOwned,
{
    #[doc(hidden)]
    pub fn new(
        request_sender: RequestSender,
        request_builder: RpcRequestBuilder<RpcRequestBuilderUninit, String>,
    ) -> Self {
        Self {
            request_sender,
            request_builder,
            _response_type: PhantomData,
        }
    }

    pub async fn call(self) -> Result<ResponseType, CallError> {
        self.request_sender
            .call(self.request_builder)
            .await?
            .get_response()
            .map_err(Into::into)
    }

    pub async fn notify(self) -> Result<(), NotifyError> {
        self.request_sender.notify(self.request_builder).await
    }
}
