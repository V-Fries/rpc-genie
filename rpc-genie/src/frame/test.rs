use tokio::io::DuplexStream;

use super::rpc_request::{RpcRequestArgReader, RpcResponseMode};
use super::*;

const MAX_FRAME_SIZE: usize = 1024;

#[derive(Debug, PartialEq, Eq, serde::Deserialize, serde::Serialize)]
struct Foo {
    i: i32,
    str: String,
}

fn create_duplex_buf_stream() -> (BufWriter<DuplexStream>, BufReader<DuplexStream>) {
    let (stream_1, stream_2) = tokio::io::duplex(1024);
    (BufWriter::new(stream_1), BufReader::new(stream_2))
}

mod rpc_request {
    use super::*;

    #[tokio::test]
    async fn method_with_params() {
        let (mut stream_1, mut stream_2) = create_duplex_buf_stream();

        let rpc_request = RpcRequest::builder()
            .method_path("method_path")
            .response_mode(RpcResponseMode::ExpectsResponseWithId(0.into()))
            .add_param(&42u32)
            .add_param(&Foo {
                i: 42,
                str: "hey".to_owned(),
            })
            .add_param(&"string")
            .build();
        Frame::RpcRequest(rpc_request)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        let rpc_request = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut stream_2)
            .await
            .unwrap()
        {
            Frame::RpcRequest(rpc_request) => rpc_request,
            _ => panic!("Received unexpected frame"),
        };
        let mut rpc_request_reader = RpcRequestArgReader::from(rpc_request.args);
        assert_eq!("method_path", rpc_request.method_path);
        assert_eq!(
            RpcResponseMode::ExpectsResponseWithId(0.into()),
            rpc_request.response_mode
        );
        assert_eq!(42u32, rpc_request_reader.read_arg().unwrap());
        assert_eq!(
            Foo {
                i: 42,
                str: "hey".to_owned()
            },
            rpc_request_reader.read_arg().unwrap()
        );
        assert_eq!("string", rpc_request_reader.read_arg::<String>().unwrap());
    }

    #[tokio::test]
    async fn method_without_params() {
        let (mut stream_1, mut stream_2) = create_duplex_buf_stream();

        let rpc_request = RpcRequest::builder()
            .method_path("method_path")
            .response_mode(RpcResponseMode::ExpectsResponseWithId(1.into()))
            .build();
        Frame::RpcRequest(rpc_request)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        let rpc_request = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut stream_2)
            .await
            .unwrap()
        {
            Frame::RpcRequest(rpc_request) => rpc_request,
            _ => panic!("Received unexpected frame"),
        };
        assert_eq!("method_path", rpc_request.method_path);
        assert_eq!(
            RpcResponseMode::ExpectsResponseWithId(1.into()),
            rpc_request.response_mode
        );
    }

    #[tokio::test]
    async fn multiple_methods_at_the_same_time() {
        let (mut stream_1, mut stream_2) = create_duplex_buf_stream();

        let rpc_request = RpcRequest::builder()
            .method_path("method_path")
            .response_mode(RpcResponseMode::NoResponse)
            .add_param(&42u32)
            .add_param(&546u128)
            .add_param(&"string")
            .build();
        Frame::RpcRequest(rpc_request)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        let rpc_request = RpcRequest::builder()
            .method_path("method_path")
            .response_mode(RpcResponseMode::ExpectsResponseWithId(1.into()))
            .build();
        Frame::RpcRequest(rpc_request)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        let rpc_request_1 = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut stream_2)
            .await
            .unwrap()
        {
            Frame::RpcRequest(rpc_request) => rpc_request,
            _ => panic!("Received unexpected frame"),
        };
        let mut rpc_request_reader_1 = RpcRequestArgReader::from(rpc_request_1.args);

        let rpc_request_2 = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut stream_2)
            .await
            .unwrap()
        {
            Frame::RpcRequest(rpc_request) => rpc_request,
            _ => panic!("Received unexpected frame"),
        };

        assert_eq!("method_path", rpc_request_1.method_path);
        assert_eq!(RpcResponseMode::NoResponse, rpc_request_1.response_mode);
        assert_eq!(42u32, rpc_request_reader_1.read_arg().unwrap());
        assert_eq!(546u128, rpc_request_reader_1.read_arg().unwrap());
        assert_eq!("string", rpc_request_reader_1.read_arg::<String>().unwrap());
        assert_eq!("method_path", rpc_request_2.method_path);
        assert_eq!(
            RpcResponseMode::ExpectsResponseWithId(1.into()),
            rpc_request_2.response_mode
        );
    }
}

mod rpc_response {
    use crate::frame::rpc_response::RpcResponseError;

    use super::*;

    #[tokio::test]
    async fn response_with_return_value() {
        let (mut stream_1, mut stream_2) = create_duplex_buf_stream();

        let return_value = Foo {
            i: 42,
            str: "test".to_owned(),
        };
        let response = RpcResponse::builder()
            .id(0.into())
            .response(&return_value)
            .build();
        Frame::RpcResponse(response)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        let rpc_response = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut stream_2)
            .await
            .unwrap()
        {
            Frame::RpcResponse(response) => response,
            _ => panic!("Received unexpected frame"),
        };

        assert_eq!(rpc_response.id(), 0);
        assert_eq!(return_value, rpc_response.get_response::<Foo>().unwrap());
    }

    #[tokio::test]
    async fn response_without_return_value() {
        let (mut stream_1, mut stream_2) = create_duplex_buf_stream();

        let response = RpcResponse::builder().id(1.into()).response(&()).build();
        Frame::RpcResponse(response)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        let rpc_response = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut stream_2)
            .await
            .unwrap()
        {
            Frame::RpcResponse(response) => response,
            _ => panic!("Received unexpected frame"),
        };

        assert_eq!(rpc_response.id(), 1);
        assert_eq!((), rpc_response.get_response::<()>().unwrap());
    }

    #[tokio::test]
    async fn multiple_responses_at_the_same_time() {
        let (mut stream_1, mut stream_2) = create_duplex_buf_stream();

        let response = RpcResponse::builder()
            .id(0.into())
            .response(&"test")
            .build();
        Frame::RpcResponse(response)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        let response = RpcResponse::builder().id(1.into()).response(&()).build();
        Frame::RpcResponse(response)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        let rpc_response_1: RpcResponse = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut stream_2)
            .await
            .unwrap()
        {
            Frame::RpcResponse(response) => response,
            _ => panic!("Received unexpected frame"),
        };

        let rpc_response_2: RpcResponse = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut stream_2)
            .await
            .unwrap()
        {
            Frame::RpcResponse(response) => response,
            _ => panic!("Received unexpected frame"),
        };

        assert_eq!(rpc_response_1.id(), 0);
        assert_eq!("test", rpc_response_1.get_response::<String>().unwrap());

        assert_eq!(rpc_response_2.id(), 1);
        assert_eq!((), rpc_response_2.get_response::<()>().unwrap());
    }

    #[tokio::test]
    async fn error_response() {
        let (mut stream_1, mut stream_2) = create_duplex_buf_stream();

        let response = RpcResponse::builder()
            .id(1.into())
            .error(RpcResponseError::MethodNotFound)
            .build();
        Frame::RpcResponse(response)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        let rpc_response: RpcResponse = match Frame::read_frame::<MAX_FRAME_SIZE>(&mut stream_2)
            .await
            .unwrap()
        {
            Frame::RpcResponse(response) => response,
            _ => panic!("Received unexpected frame"),
        };

        assert_eq!(rpc_response.id(), 1);
        assert_eq!(
            RpcResponseError::MethodNotFound,
            rpc_response.get_response::<()>().unwrap_err()
        );
    }
}

mod max_size_check {
    use super::*;

    #[tokio::test]
    async fn write_to_big_request() {
        let (mut stream_1, _stream_2) = create_duplex_buf_stream();

        let request = RpcRequest::builder()
            .response_mode(RpcResponseMode::ExpectsResponseWithId(0.into()))
            .method_path("test")
            .add_param(&42)
            .build();
        match Frame::RpcRequest(request)
            .write_frame::<1>(&mut stream_1)
            .await
            .unwrap_err()
        {
            WriteError::FrameTooBig { size: _, max_size } => assert_eq!(max_size, 1),
            other => println!("Unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn read_to_big_request() {
        let (mut stream_1, mut stream_2) = create_duplex_buf_stream();

        let request = RpcRequest::builder()
            .response_mode(RpcResponseMode::ExpectsResponseWithId(0.into()))
            .method_path("test")
            .add_param(&42)
            .build();
        Frame::RpcRequest(request)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        match Frame::read_frame::<3>(&mut stream_2).await.unwrap_err() {
            ReadError::FrameTooBig { size: _, max_size } => assert_eq!(max_size, 3),
            other => println!("Unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn write_to_big_response() {
        let (mut stream_1, _stream_2) = create_duplex_buf_stream();

        let response = RpcResponse::builder()
            .id(0.into())
            .response(&"test")
            .build();
        match Frame::RpcResponse(response)
            .write_frame::<1>(&mut stream_1)
            .await
            .unwrap_err()
        {
            WriteError::FrameTooBig { size: _, max_size } => assert_eq!(max_size, 1),
            other => println!("Unexpected error: {other:?}"),
        }
    }

    #[tokio::test]
    async fn read_to_big_response() {
        let (mut stream_1, mut stream_2) = create_duplex_buf_stream();

        let response = RpcResponse::builder()
            .id(0.into())
            .response(&"test")
            .build();
        Frame::RpcResponse(response)
            .write_frame::<MAX_FRAME_SIZE>(&mut stream_1)
            .await
            .unwrap();

        match Frame::read_frame::<3>(&mut stream_2).await.unwrap_err() {
            ReadError::FrameTooBig { size: _, max_size } => assert_eq!(max_size, 3),
            other => println!("Unexpected error: {other:?}"),
        }
    }
}
