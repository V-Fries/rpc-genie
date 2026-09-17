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
    fn new() -> Arc<Self> {
        Arc::new(Self {
            state: Arc::new(Mutex::new(TestStubState::default())),
            stream_id: StreamId::next(),
        })
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

    fn stream_id(&self) -> Option<StreamId> {
        Some(self.stream_id)
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

    topic.subscribe(stub.clone()).await;
    topic.unsubscribe(&stub).await;

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

#[derive(Clone)]
struct NoStreamIdStub;

impl SubscribableStub for NoStreamIdStub {
    async fn add_registered_topic(
        &self,
        _topic_id: crate::topic::TopicId,
        _stub_died_notification_sender: StubDiedNotificationSender,
    ) -> bool {
        true
    }

    fn remove_registered_topic(&self, _topic_id: crate::topic::TopicId) {}

    fn stream_id(&self) -> Option<StreamId> {
        None
    }
}

#[tokio::test]
async fn subscribe_stub_with_no_stream_id_is_noop() {
    let topic = Topic::new().await;

    topic.subscribe(Arc::new(NoStreamIdStub)).await;

    assert!(topic.map(|_| async {}).await.is_empty());
}

#[tokio::test]
async fn stub_subscribed_to_multiple_topics_dies_removes_from_all() {
    let topic_a = Topic::new().await;
    let topic_b = Topic::new().await;
    let stub = TestStub::new();

    topic_a.subscribe(stub.clone()).await;
    topic_b.subscribe(stub.clone()).await;
    assert_eq!(stub.registered_topic_count(), 2);

    stub.die();
    wait_until_empty(&topic_a).await;
    wait_until_empty(&topic_b).await;
    assert_eq!(stub.registered_topic_count(), 0);
}

#[tokio::test]
async fn unsubscribe_then_resubscribe_same_stub() {
    let topic = Topic::new().await;
    let stub = TestStub::new();

    topic.subscribe(stub.clone()).await;
    assert_eq!(topic.map(|_| async {}).await.len(), 1);

    topic.unsubscribe(&stub).await;
    assert!(topic.map(|_| async {}).await.is_empty());
    assert_eq!(stub.registered_topic_count(), 0);

    topic.subscribe(stub.clone()).await;
    assert_eq!(topic.map(|_| async {}).await.len(), 1);
    assert_eq!(stub.registered_topic_count(), 1);
}

#[tokio::test]
async fn map_and_for_each_on_empty_topic() {
    let topic: Topic<TestStub> = Topic::new().await;

    let ids = topic.map(|stub| async move { stub.stream_id() }).await;
    assert!(ids.is_empty());

    let visited = Arc::new(Mutex::new(false));
    let visited_clone = Arc::clone(&visited);
    topic
        .for_each(move |_stub| {
            let visited = Arc::clone(&visited_clone);
            async move {
                *visited.lock().unwrap() = true;
            }
        })
        .await;

    assert!(!*visited.lock().unwrap());
}

#[tokio::test]
async fn concurrent_subscribes() {
    let topic = Topic::new().await;
    let stubs = (0..10).map(|_| TestStub::new()).collect::<Vec<_>>();

    let mut handles = Vec::new();
    for stub in &stubs {
        let topic_clone = Topic {
            subscribed_stubs: Arc::clone(&topic.subscribed_stubs),
            routine_command_sender: topic.routine_command_sender.clone(),
        };
        let stub_clone = stub.clone();
        handles.push(tokio::spawn(async move {
            topic_clone.subscribe(stub_clone).await;
        }));
    }

    for handle in handles {
        handle.await.unwrap();
    }

    let mut ids = topic.map(|stub| async move { stub.stream_id() }).await;
    ids.sort_by_key(|id| id.to_string());

    let mut expected: Vec<StreamId> = stubs.iter().map(|s| s.stream_id()).collect();
    expected.sort_by_key(|id| id.to_string());
    assert_eq!(ids, expected);

    for stub in &stubs {
        assert_eq!(stub.registered_topic_count(), 1);
    }
}
