use std::{
    assert_matches,
    collections::{HashMap, hash_map},
    sync::Arc,
};

use crate::{
    SubscribableStub,
    stream_handler::StreamId,
    topic::{TopicId, routine::StubDiedNotificationSender},
};

pub struct SubscribedStubs<Stub>
where
    Stub: SubscribableStub,
{
    stubs: Vec<(StreamId, Arc<Stub>)>,
    positions: HashMap<StreamId, usize>,
    topic_id: TopicId,
}

impl<Stub> Drop for SubscribedStubs<Stub>
where
    Stub: SubscribableStub,
{
    fn drop(&mut self) {
        for (_stream_id, stub) in self.stubs.iter() {
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

    pub fn iter(&self) -> impl Iterator<Item = &Arc<Stub>> {
        self.stubs.iter().map(|(_stream_id, stub)| stub)
    }

    pub(super) async fn add(
        &mut self,
        stub: Arc<Stub>,
        topic_id: TopicId,
        stub_died_notification_sender: StubDiedNotificationSender,
    ) {
        let stream_id = match stub.stream_id() {
            Some(stream_id) => stream_id,
            None => return,
        };

        match self.positions.entry(stream_id) {
            hash_map::Entry::Occupied(_) => {}
            hash_map::Entry::Vacant(entry) => {
                if !stub
                    .add_registered_topic(topic_id, stub_died_notification_sender)
                    .await
                {
                    return;
                }

                entry.insert(self.stubs.len());
                self.stubs.push((stream_id, stub));
            }
        }
    }

    pub(super) fn remove<const SHOULD_CALL_REMOVE_REGISTERED_TOPIC: bool>(
        &mut self,
        stream_id: StreamId,
        topic_id: TopicId,
    ) {
        let Some(index) = self.positions.remove(&stream_id) else {
            return;
        };

        let last_index = self.stubs.len() - 1;
        let (_stream_id, removed_stub) = self.stubs.swap_remove(index);

        if index != last_index {
            let moved_stream_id = self.stubs[index].0;

            assert_matches!(
                self.positions.insert(moved_stream_id, index),
                Some(overwritten_index) if overwritten_index == last_index,
            );
        }

        if SHOULD_CALL_REMOVE_REGISTERED_TOPIC {
            removed_stub.remove_registered_topic(topic_id);
        }
    }
}
