#[rpc_genie::service]
mod service {
    sub_services! {
        pub child_service: super::child_service,
    }

    pub struct Server<Stream> {}

    impl<Stream> Server<Stream> {
        #[remote_method]
        pub fn _do_stuff(_arg: u32) -> String {
            unreachable!("mocks don't use the real implementation");
        }
    }

    pub struct Client<Stream> {}

    impl<Stream> Client<Stream> {
        #[remote_method]
        pub fn _do_stuff(_arg: u32) -> String {
            unreachable!("mocks don't use the real implementation");
        }

        #[remote_method]
        pub fn _empty() {
            unreachable!("mocks don't use the real implementation");
        }
    }
}

#[rpc_genie::service]
mod child_service {
    pub struct Server<Stream> {}

    impl<Stream> Server<Stream> {
        #[remote_method]
        pub fn _ping(_arg: String) -> String {
            unreachable!("mocks don't use the real implementation");
        }
    }

    pub struct Client<Stream> {}

    impl<Stream> Client<Stream> {
        #[remote_method]
        pub fn _ping(_arg: String) -> String {
            unreachable!("mocks don't use the real implementation");
        }
    }
}

#[tokio::main]
async fn main() {
    let mock = service::ClientMock::new(|expectation_setter| {
        expectation_setter
            ._do_stuff(1)
            .call()
            .returns("hey".to_owned());

        expectation_setter
            ._do_stuff(2)
            .call()
            .returns("heyy".to_owned());

        expectation_setter
            .child_service()
            ._ping("foo".to_string())
            .call()
            .returns("foo".to_owned());

        expectation_setter._empty().call().returns(());

        expectation_setter._do_stuff(3).notify();
    });

    // mock._do_stuff(3).notify().await.unwrap();

    assert_eq!("hey".to_owned(), mock._do_stuff(1).call().await.unwrap());
    assert_eq!("heyy".to_owned(), mock._do_stuff(2).call().await.unwrap());

    assert_eq!(
        "foo".to_owned(),
        mock.child_service()
            ._ping("foo".to_owned())
            .call()
            .await
            .unwrap()
    );

    mock._empty().call().await.unwrap();

    mock._do_stuff(3).notify().await.unwrap();

    // When mock is dropped it will check that all expected methods have been called and panic
    // otherwise
    drop(mock);

    println!("Success");
}
