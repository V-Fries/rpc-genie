I deleted the tests.

I've decided we'll first code the final mock feature I wanted first before adding test and deciding if this trait system is fine.


So here's the mock system I want:
Now that the service generates a trait for each stub, the user can write:
async fn foo(client_stub: impl service::ClientStubTrait) {
    // ...
}

but that is is not enough to make writing unit tests really really simple.
So here's what I want now :D

I want a ServerMock and a ClientMock struct to be added to each service.
The user will be able to use them as such

async fn foo(client: impl service::ClientStubTrait) {
    assert_eq!(0, client.ping(0).call().await.unwrap());

    assert_eq!(
        "hey you".to_owned(),
        client
            .add_str("hey ".to_owned(), "you".to_owned())
            .call()
            .await
            .unwrap()
    );

    client.bar(4).notify().await.unwrap();

    client.sub_service().foo_bar(4).notify().await.unwrap();
}

#[tokio::test]
async fn foo_test() {
    let mock = service::ClientMock::new(|client| {
        client.ping(0).call().returns(0);

        client
            .add_str("hey ".to_owned(), "you".to_owned())
            .call()
            .returns("hey you".to_owned());

        client.bar(4).notify();

        client.sub_service().foo_bar(4).notify();
    });

    foo(mock).await;

    // expectations are checked when mock is dropped
}
