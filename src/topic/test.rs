use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::{
    SubscribableStub,
    stream_handler::StreamId,
    topic::{StubDiedNotificationSender, Topic},
};

#[derive(Clone)]
struct TestStub {
    state: Arc<Mutex<TestStubState>>,
    stream_id: StreamId,
}

#[derive(Default)]
struct TestStubState {
    registered_topic_count: usize,
    death_senders: Vec<StubDiedNotificationSender>,
}

impl TestStub {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(TestStubState::default())),
            stream_id: StreamId::next(),
        }
    }

    fn stream_id(&self) -> StreamId {
        self.stream_id
    }

    fn registered_topic_count(&self) -> usize {
        self.state.lock().unwrap().registered_topic_count
    }

    fn die(&self) {
        let mut state = self.state.lock().unwrap();
        state.registered_topic_count = 0;
        let senders = std::mem::take(&mut state.death_senders);
        drop(state);

        for sender in senders {
            let _ = sender.send(());
        }
    }
}

impl SubscribableStub for TestStub {
    async fn add_registered_topic(
        &self,
        _topic_id: crate::topic::TopicId,
        stub_died_notification_sender: StubDiedNotificationSender,
    ) -> bool {
        let mut state = self.state.lock().unwrap();
        state.registered_topic_count += 1;
        state.death_senders.push(stub_died_notification_sender);
        true
    }

    fn remove_registered_topic(&self, _topic_id: crate::topic::TopicId) {
        let mut state = self.state.lock().unwrap();
        state.registered_topic_count -= 1;
    }

    fn stream_id(&self) -> StreamId {
        self.stream_id
    }
}

async fn wait_until_empty(topic: &Topic<TestStub>) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if topic
                .map(|stub| async move { stub.stream_id() })
                .await
                .is_empty()
            {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("topic did not remove the dead stub");
}

async fn wait_until_registration_count(stub: &TestStub, expected: usize) {
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if stub.registered_topic_count() == expected {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("stub registration count did not reach the expected value");
}

#[tokio::test]
async fn subscribe_and_map_visits_each_stub() {
    let topic = Topic::new().await;
    let first = TestStub::new();
    let second = TestStub::new();
    let first_id = first.stream_id();
    let second_id = second.stream_id();

    topic.subscribe(first).await;
    topic.subscribe(second).await;

    let mut ids = topic.map(|stub| async move { stub.stream_id() }).await;
    ids.sort_by_key(|id| id.to_string());

    let mut expected = vec![first_id, second_id];
    expected.sort_by_key(|id| id.to_string());
    assert_eq!(ids, expected);
}

#[tokio::test]
async fn duplicate_stream_is_subscribed_only_once() {
    let topic = Topic::new().await;
    let stub = TestStub::new();

    topic.subscribe(stub.clone()).await;
    topic.subscribe(stub.clone()).await;

    assert_eq!(topic.map(|_| async {}).await.len(), 1);
    assert_eq!(stub.registered_topic_count(), 1);
}

#[tokio::test]
async fn unsubscribe_removes_registration_and_stub() {
    let topic = Topic::new().await;
    let stub = TestStub::new();
    let stream_id = stub.stream_id();

    topic.subscribe(stub.clone()).await;
    topic.unsubscribe(stream_id).await;

    assert!(topic.map(|_| async {}).await.is_empty());
    assert_eq!(stub.registered_topic_count(), 0);
}

#[tokio::test]
async fn stub_death_removes_stub_from_topic() {
    let topic = Topic::new().await;
    let stub = TestStub::new();

    topic.subscribe(stub.clone()).await;
    assert_eq!(topic.map(|_| async {}).await.len(), 1);

    stub.die();
    wait_until_empty(&topic).await;
    assert_eq!(stub.registered_topic_count(), 0);
}

#[tokio::test]
async fn for_each_skips_stub_after_it_dies() {
    let topic = Topic::new().await;
    let dropped_stub = TestStub::new();
    let remaining_stub = TestStub::new();
    let remaining_id = remaining_stub.stream_id();

    topic.subscribe(dropped_stub.clone()).await;
    topic.subscribe(remaining_stub).await;

    dropped_stub.die();
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if topic.map(|stub| async move { stub.stream_id() }).await == vec![remaining_id] {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("topic did not remove the dead stub");

    let visited_ids = Arc::new(Mutex::new(Vec::new()));
    let visited_ids_clone = Arc::clone(&visited_ids);
    topic
        .for_each(move |stub| {
            let visited_ids = Arc::clone(&visited_ids_clone);
            async move {
                visited_ids.lock().unwrap().push(stub.stream_id());
            }
        })
        .await;

    assert_eq!(*visited_ids.lock().unwrap(), vec![remaining_id]);
}

#[tokio::test]
async fn dropping_topic_unregisters_remaining_stubs() {
    let first = TestStub::new();
    let second = TestStub::new();

    {
        let topic = Topic::new().await;
        topic.subscribe(first.clone()).await;
        topic.subscribe(second.clone()).await;

        assert_eq!(first.registered_topic_count(), 1);
        assert_eq!(second.registered_topic_count(), 1);
    }

    wait_until_registration_count(&first, 0).await;
    wait_until_registration_count(&second, 0).await;
}
