use std::{
    assert_matches,
    collections::{HashMap, hash_map},
};

use tokio::sync::oneshot;

use crate::{
    SubscribableStub,
    stream_handler::StreamId,
    topic::{TopicId, routine::StubDiedNotificationReceiver},
};

pub struct SubscribedStubs<Stub>
where
    Stub: SubscribableStub,
{
    stubs: Vec<Stub>,
    positions: HashMap<StreamId, usize>,
    topic_id: TopicId,
}

impl<Stub> Drop for SubscribedStubs<Stub>
where
    Stub: SubscribableStub,
{
    fn drop(&mut self) {
        for stub in self.stubs.iter() {
            stub.remove_registered_topic(self.topic_id);
        }
    }
}

impl<Stub> SubscribedStubs<Stub>
where
    Stub: SubscribableStub,
{
    pub fn new(topic_id: TopicId) -> Self {
        Self {
            stubs: Default::default(),
            positions: Default::default(),
            topic_id,
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = &Stub> {
        self.stubs.iter()
    }

    /// Returns Some(StubDiedNotificationReceiver) if add was successful, None if it failed or if
    /// the stub was already present
    pub(super) async fn add(
        &mut self,
        stub: Stub,
        topic_id: TopicId,
    ) -> Option<StubDiedNotificationReceiver> {
        let (sender, receiver) = oneshot::channel();

        match self.positions.entry(stub.stream_id()) {
            hash_map::Entry::Occupied(_) => None,
            hash_map::Entry::Vacant(entry) => {
                if !stub.add_registered_topic(topic_id, sender).await {
                    return None;
                }

                entry.insert(self.stubs.len());
                self.stubs.push(stub);

                Some(receiver)
            }
        }
    }

    /// Returns true if it was removed, false otherwise
    pub(super) fn remove<const SHOULD_CALL_REMOVE_REGISTERED_TOPIC: bool>(
        &mut self,
        stream_id: StreamId,
        topic_id: TopicId,
    ) -> bool {
        let Some(index) = self.positions.remove(&stream_id) else {
            return false;
        };

        let last_index = self.stubs.len() - 1;
        let removed_stub = self.stubs.swap_remove(index);

        if index != last_index {
            let moved_stub_id = self.stubs[index].stream_id();

            assert_matches!(
                self.positions.insert(moved_stub_id, index),
                Some(overwritten_index) if overwritten_index == last_index,
            );
        }

        if SHOULD_CALL_REMOVE_REGISTERED_TOPIC {
            removed_stub.remove_registered_topic(topic_id);
        }

        true
    }
}
