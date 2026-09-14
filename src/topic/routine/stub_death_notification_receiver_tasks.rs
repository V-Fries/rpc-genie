use std::{
    collections::{HashMap, hash_map},
    sync::{Arc, Mutex},
};

use tokio::{sync::oneshot, task::JoinHandle};

use crate::{
    stream_handler::StreamId,
    topic::routine::{RoutineCommand, RoutineCommandWeakSender},
};

pub struct StubDeathNotificationReceiverTasks<Stub> {
    join_handles: Arc<Mutex<HashMap<StreamId, JoinHandle<()>>>>,
    routine_command_sender: RoutineCommandWeakSender<Stub>,
}

pub type StubDiedNotificationSender = oneshot::Sender<()>;
pub type StubDiedNotificationReceiver = oneshot::Receiver<()>;

impl<Stub> Drop for StubDeathNotificationReceiverTasks<Stub> {
    fn drop(&mut self) {
        for join_handle in self
            .join_handles
            .lock()
            .unwrap_or_else(|poison_error| poison_error.into_inner())
            .values()
        {
            join_handle.abort();
        }
    }
}

impl<Stub> StubDeathNotificationReceiverTasks<Stub>
where
    Stub: Send + 'static,
{
    pub fn new(routine_command_sender: RoutineCommandWeakSender<Stub>) -> Self {
        Self {
            join_handles: Arc::new(Mutex::new(HashMap::new())),
            routine_command_sender,
        }
    }

    pub fn new_task(
        &mut self,
        stream_id: StreamId,
        stub_died_receiver: StubDiedNotificationReceiver,
    ) {
        match self
            .join_handles
            .lock()
            .unwrap_or_else(|poison_error| poison_error.into_inner())
            .entry(stream_id)
        {
            hash_map::Entry::Occupied(_) => {}
            hash_map::Entry::Vacant(vacant_entry) => {
                let sender = self.routine_command_sender.clone();

                let weak_join_handles_ref = Arc::downgrade(&self.join_handles);

                vacant_entry.insert(tokio::spawn(async move {
                    let Ok(()) = stub_died_receiver.await else {
                        return;
                    };

                    let Some(sender) = sender.upgrade() else {
                        return;
                    };

                    let _ = sender.send(RoutineCommand::StubDied(stream_id)).await;

                    if let Some(join_handles) = weak_join_handles_ref.upgrade() {
                        join_handles
                            .lock()
                            .unwrap_or_else(|poison_error| poison_error.into_inner())
                            .remove(&stream_id);
                    }
                }));
            }
        }
    }

    pub fn abort_task(&mut self, stream_id: StreamId) {
        if let Some(join_handle) = self
            .join_handles
            .lock()
            .unwrap_or_else(|poison_error| poison_error.into_inner())
            .remove(&stream_id)
        {
            join_handle.abort();
        }
    }
}
