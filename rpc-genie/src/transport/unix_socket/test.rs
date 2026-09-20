use std::{marker::PhantomData, path::Path, sync::Arc};

#[crate::service]
mod service {
    use crate as rpc_genie;

    pub struct Server<RequestSender> {}
    pub struct Client<RequestSender> {}
}

#[tokio::test]
async fn clean_up_socket_file() {
    std::fs::create_dir_all("/tmp/rpc-genie/tests/").unwrap();
    let socket_file_path = "/tmp/rpc-genie/tests/unix_socket_clean_up_socket_file_test.sock";

    let server = super::UnixSocket::<1024>::start_server(
        socket_file_path,
        Arc::new(service::Server {
            _request_sender: PhantomData,
        }),
    )
    .await
    .unwrap();

    assert!(Path::new(socket_file_path).exists());

    drop(server);

    assert!(!Path::new(socket_file_path).exists());
}
