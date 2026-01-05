use tokio::sync::broadcast;

use crate::events::{DaemonEvent, EntryEvent, ProfileEvent, TaskEvent};
use crate::timer::TimerEvent;

pub struct EventManager {
    event_tx: broadcast::Sender<DaemonEvent>,
}

impl EventManager {
    pub fn new() -> Self {
        let (event_tx, _) = broadcast::channel(1000);
        Self { event_tx }
    }

    pub fn subscribe(&self) -> broadcast::Receiver<DaemonEvent> {
        self.event_tx.subscribe()
    }

    pub fn emit_timer(&self, event: TimerEvent) {
        let _ = self.event_tx.send(DaemonEvent::Timer(event));
    }

    pub fn emit_task(&self, event: TaskEvent) {
        tracing::info!(
            "EventManager: Broadcasting task event: {:?}",
            event.event_type
        );
        let subscriber_count = self.event_tx.receiver_count();
        tracing::info!("EventManager: {} active subscribers", subscriber_count);
        let result = self.event_tx.send(DaemonEvent::Task(event));
        match result {
            Ok(count) => tracing::info!("EventManager: Event sent to {} receivers", count),
            Err(e) => tracing::warn!("EventManager: Failed to send event: {:?}", e),
        }
    }

    pub fn emit_entry(&self, event: EntryEvent) {
        let _ = self.event_tx.send(DaemonEvent::Entry(event));
    }

    pub fn emit_profile(&self, event: ProfileEvent) {
        let _ = self.event_tx.send(DaemonEvent::Profile(event));
    }
}

impl Default for EventManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::TaskEvent;
    use chrono::Utc;
    use mootimer_core::models::{Task, TaskSource, TaskStatus};

    #[test]
    fn test_event_manager_creation() {
        let manager = EventManager::new();
        let _receiver = manager.subscribe();
    }

    #[tokio::test]
    async fn test_task_event_broadcasting() {
        let manager = EventManager::new();
        let mut receiver = manager.subscribe();

        let task = Task {
            id: "task1".to_string(),
            title: "Test".to_string(),
            description: None,
            status: TaskStatus::Todo,
            tags: vec![],
            url: None,
            source: TaskSource::Manual,
            source_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let event = TaskEvent::created("profile1".to_string(), task);
        manager.emit_task(event);

        let received = receiver.recv().await.unwrap();
        assert!(matches!(received, DaemonEvent::Task(_)));
    }

    #[tokio::test]
    async fn test_multiple_subscribers() {
        let manager = EventManager::new();
        let mut receiver1 = manager.subscribe();
        let mut receiver2 = manager.subscribe();

        let task = Task {
            id: "task1".to_string(),
            title: "Test".to_string(),
            description: None,
            status: TaskStatus::Todo,
            tags: vec![],
            url: None,
            source: TaskSource::Manual,
            source_id: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let event = TaskEvent::created("profile1".to_string(), task);
        manager.emit_task(event);

        let received1 = receiver1.recv().await.unwrap();
        let received2 = receiver2.recv().await.unwrap();

        assert!(matches!(received1, DaemonEvent::Task(_)));
        assert!(matches!(received2, DaemonEvent::Task(_)));
    }
}
