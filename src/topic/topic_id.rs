use std::sync::atomic::{self, AtomicU64};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct TopicId(u64);

impl TopicId {
    pub fn next() -> Self {
        static NEXT_TOPIC_ID: AtomicU64 = AtomicU64::new(0);
        Self(NEXT_TOPIC_ID.fetch_add(1, atomic::Ordering::Relaxed))
    }
}
