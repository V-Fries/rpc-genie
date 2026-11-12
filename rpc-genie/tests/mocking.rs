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

use rpc_genie::SendSingleRequest as _;

use crate::child_service::ClientStub as _;
use crate::service::ClientStub as _;

#[tokio::test]
async fn success() {
    let mock = service::ClientStubMock::new(|expectation_setter| {
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

    assert_eq!((), mock._empty().call().await.unwrap());

    mock._do_stuff(3).notify().await.unwrap();

    // When mock is dropped it will check that all expected methods have been called and panic
    // otherwise
    drop(mock);

    println!("Success");
}

#[tokio::test]
#[should_panic(expected = "Expected mock._do_stuff(1).call(), found mock._do_stuff(2).call()")]
async fn invocation_with_wrong_args() {
    let mock = service::ClientStubMock::new(|expectation_setter| {
        expectation_setter
            ._do_stuff(1)
            .call()
            .returns("hey".to_owned());
    });

    mock._do_stuff(2).call().await.unwrap();
}

#[tokio::test]
#[should_panic(
    expected = "Found mock._do_stuff(1).call() when no more usages of the mock were expected"
)]
async fn invocation_with_after_expectations_were_exhausted() {
    let mock = service::ClientStubMock::new(|expectation_setter| {
        expectation_setter
            ._do_stuff(1)
            .call()
            .returns("hey".to_owned());
    });

    assert_eq!("hey".to_string(), mock._do_stuff(1).call().await.unwrap());

    mock._do_stuff(1).call().await.unwrap();
}

#[tokio::test]
#[should_panic(expected = "Mock was dropped with the following expectations unmet: [\
                              mock._do_stuff(2).notify(), \
                              mock.child_service()._ping(\"ping\").call()\
                          ]")]
async fn missing_invocation() {
    let mock = service::ClientStubMock::new(|expectation_setter| {
        expectation_setter
            ._do_stuff(1)
            .call()
            .returns("hey".to_owned());

        expectation_setter._do_stuff(2).notify();

        expectation_setter
            .child_service()
            ._ping("ping".to_owned())
            .call()
            .returns("ping".to_owned());
    });

    assert_eq!("hey".to_string(), mock._do_stuff(1).call().await.unwrap());
}
