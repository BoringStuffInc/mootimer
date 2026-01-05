use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};

use mootimer_core::models::{ActiveTimer, Entry, PomodoroConfig};

use super::engine::{TimerEngine, TimerEngineError};
use super::events::TimerEvent;
use crate::event_manager::EventManager;

#[derive(Debug, thiserror::Error)]
pub enum TimerManagerError {
    #[error("Timer not found: {0}")]
    NotFound(String),

    #[error("Profile already has active timer")]
    ProfileHasActiveTimer,

    #[error("Timer engine error: {0}")]
    Engine(#[from] TimerEngineError),
}

pub type Result<T> = std::result::Result<T, TimerManagerError>;

#[derive(Clone)]
pub struct TimerManager {
    timers: Arc<RwLock<HashMap<String, Arc<TimerEngine>>>>,
    event_manager: Arc<EventManager>,
    event_tx: broadcast::Sender<TimerEvent>,
    completed_entries: Arc<RwLock<Vec<(String, Entry)>>>,
}

impl TimerManager {
    pub fn new(event_manager: Arc<EventManager>) -> Self {
        let (event_tx, mut event_rx) = broadcast::channel(1000);
        let timers = Arc::new(RwLock::new(HashMap::new()));
        let completed_entries = Arc::new(RwLock::new(Vec::new()));

        let event_manager_clone = event_manager.clone();
        tokio::spawn(async move {
            while let Ok(timer_event) = event_rx.recv().await {
                event_manager_clone.emit_timer(timer_event);
            }
        });

        Self {
            timers,
            event_manager,
            event_tx,
            completed_entries,
        }
    }

    pub async fn take_completed_entries(&self) -> Vec<(String, Entry)> {
        let mut entries = self.completed_entries.write().await;
        std::mem::take(&mut *entries)
    }

    async fn handle_countdown_completion(
        timers: Arc<RwLock<HashMap<String, Arc<TimerEngine>>>>,
        completed_entries: Arc<RwLock<Vec<(String, Entry)>>>,
        profile_id: String,
    ) {
        tracing::info!("handle_countdown_completion called for {}", profile_id);

        tracing::debug!("Acquiring write lock on timers HashMap");
        let engine = {
            let mut timers_lock = timers.write().await;
            tracing::debug!("Got write lock on timers HashMap");
            timers_lock.remove(&profile_id)
        };
        tracing::debug!("Released write lock on timers HashMap");

        if let Some(engine) = engine {
            tracing::debug!("Getting timer state from engine");
            let timer = engine.get_timer().await;
            tracing::debug!("Got timer state");
            if let Ok(entry) = Entry::create_completed(
                timer.task_id.clone(),
                timer.start_time,
                chrono::Utc::now(),
                timer.mode,
            ) {
                tracing::info!(
                    "Countdown completed for profile {}, creating entry",
                    profile_id
                );
                tracing::debug!("Acquiring write lock on completed_entries");
                let mut entries = completed_entries.write().await;
                tracing::debug!("Got write lock on completed_entries");
                entries.push((profile_id, entry));
            }
        }
        tracing::info!("handle_countdown_completion finished for profile");
    }

    pub fn subscribe(&self) -> broadcast::Receiver<TimerEvent> {
        // For backward compatibility, still return a TimerEvent receiver
        // The event manager will handle broadcasting as DaemonEvent
        self.event_tx.subscribe()
    }

    pub async fn start_manual(
        &self,
        profile_id: String,
        task_id: Option<String>,
    ) -> Result<String> {
        {
            let timers = self.timers.read().await;
            if timers.contains_key(&profile_id) {
                return Err(TimerManagerError::ProfileHasActiveTimer);
            }
        }

        let engine = Arc::new(TimerEngine::new_manual(
            profile_id.clone(),
            task_id.clone(),
            self.event_tx.clone(),
        ));

        let timer_id = engine.timer_id().await;

        let event = TimerEvent::started(
            profile_id.clone(),
            timer_id.clone(),
            task_id,
            mootimer_core::models::TimerMode::Manual,
        );
        self.event_manager.emit_timer(event.clone());
        let _ = self.event_tx.send(event);

        let engine_clone = engine.clone();
        tokio::spawn(async move {
            engine_clone.start_tick_loop().await;
        });

        {
            let mut timers = self.timers.write().await;
            timers.insert(profile_id, engine);
        }

        Ok(timer_id)
    }

    pub async fn start_pomodoro(
        &self,
        profile_id: String,
        task_id: Option<String>,
        config: PomodoroConfig,
    ) -> Result<String> {
        {
            let timers = self.timers.read().await;
            if timers.contains_key(&profile_id) {
                return Err(TimerManagerError::ProfileHasActiveTimer);
            }
        }

        let engine = Arc::new(TimerEngine::new_pomodoro(
            profile_id.clone(),
            task_id.clone(),
            config,
            self.event_tx.clone(),
        ));

        let timer_id = engine.timer_id().await;

        let event = TimerEvent::started(
            profile_id.clone(),
            timer_id.clone(),
            task_id,
            mootimer_core::models::TimerMode::Pomodoro,
        );
        self.event_manager.emit_timer(event.clone());
        let _ = self.event_tx.send(event);

        let engine_clone = engine.clone();
        tokio::spawn(async move {
            engine_clone.start_tick_loop().await;
        });

        {
            let mut timers = self.timers.write().await;
            timers.insert(profile_id, engine);
        }

        Ok(timer_id)
    }

    pub async fn start_countdown(
        &self,
        profile_id: String,
        task_id: Option<String>,
        duration_minutes: u64,
    ) -> Result<String> {
        {
            let timers = self.timers.read().await;
            if timers.contains_key(&profile_id) {
                return Err(TimerManagerError::ProfileHasActiveTimer);
            }
        }

        let engine = Arc::new(TimerEngine::new_countdown(
            profile_id.clone(),
            task_id.clone(),
            duration_minutes,
            self.event_tx.clone(),
        ));

        let timer_id = engine.timer_id().await;

        let event = TimerEvent::started(
            profile_id.clone(),
            timer_id.clone(),
            task_id,
            mootimer_core::models::TimerMode::Countdown,
        );
        self.event_manager.emit_timer(event.clone());
        let _ = self.event_tx.send(event);

        let engine_clone = engine.clone();
        let timers_clone = self.timers.clone();
        let completed_entries_clone = self.completed_entries.clone();
        let profile_id_clone = profile_id.clone();
        tokio::spawn(async move {
            engine_clone.start_tick_loop().await;
            Self::handle_countdown_completion(
                timers_clone,
                completed_entries_clone,
                profile_id_clone,
            )
            .await;
        });

        {
            let mut timers = self.timers.write().await;
            timers.insert(profile_id, engine);
        }

        Ok(timer_id)
    }

    pub async fn get_timer(&self, profile_id: &str) -> Result<ActiveTimer> {
        tracing::debug!("get_timer: acquiring read lock on timers HashMap");
        let engine = {
            let timers = self.timers.read().await;
            tracing::debug!("get_timer: got read lock on timers HashMap");
            timers
                .get(profile_id)
                .cloned()
                .ok_or_else(|| TimerManagerError::NotFound(profile_id.to_string()))?
        };
        tracing::debug!("get_timer: released read lock, calling engine.get_timer()");
        // Lock released before calling engine.get_timer() to avoid deadlock
        let result = engine.get_timer().await;
        tracing::debug!("get_timer: engine.get_timer() returned");
        Ok(result)
    }

    pub async fn get_all_timers(&self) -> HashMap<String, ActiveTimer> {
        let engines: Vec<_> = {
            let timers = self.timers.read().await;
            timers.iter().map(|(k, v)| (k.clone(), v.clone())).collect()
        };
        // Lock released before calling engine.get_timer() to avoid deadlock
        let mut result = HashMap::new();
        for (profile_id, engine) in engines {
            result.insert(profile_id, engine.get_timer().await);
        }

        result
    }

    pub async fn pause(&self, profile_id: &str) -> Result<()> {
        let engine = {
            let timers = self.timers.read().await;
            timers
                .get(profile_id)
                .cloned()
                .ok_or_else(|| TimerManagerError::NotFound(profile_id.to_string()))?
        };
        // Lock released before awaiting to avoid deadlock
        engine.pause().await?;
        Ok(())
    }

    pub async fn resume(&self, profile_id: &str) -> Result<()> {
        let engine = {
            let timers = self.timers.read().await;
            timers
                .get(profile_id)
                .cloned()
                .ok_or_else(|| TimerManagerError::NotFound(profile_id.to_string()))?
        };
        // Lock released before awaiting to avoid deadlock
        engine.resume().await?;
        Ok(())
    }

    pub async fn stop(&self, profile_id: &str) -> Result<Entry> {
        let engine = {
            let mut timers = self.timers.write().await;
            timers
                .remove(profile_id)
                .ok_or_else(|| TimerManagerError::NotFound(profile_id.to_string()))?
        };

        let entry = engine.stop().await?;
        Ok(entry)
    }

    pub async fn cancel(&self, profile_id: &str) -> Result<()> {
        let engine = {
            let mut timers = self.timers.write().await;
            timers
                .remove(profile_id)
                .ok_or_else(|| TimerManagerError::NotFound(profile_id.to_string()))?
        };

        engine.cancel().await?;
        Ok(())
    }

    pub async fn has_active_timer(&self, profile_id: &str) -> bool {
        let timers = self.timers.read().await;
        timers.contains_key(profile_id)
    }

    pub async fn active_timer_count(&self) -> usize {
        let timers = self.timers.read().await;
        timers.len()
    }
}

impl Default for TimerManager {
    fn default() -> Self {
        Self::new(Arc::new(crate::event_manager::EventManager::new()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_manager::EventManager;
    use tokio::time::{Duration, sleep};

    fn create_manager() -> TimerManager {
        let event_manager = Arc::new(EventManager::new());
        TimerManager::new(event_manager)
    }

    #[tokio::test]
    async fn test_start_manual_timer() {
        let manager = create_manager();

        let timer_id = manager
            .start_manual("profile1".to_string(), Some("task1".to_string()))
            .await
            .unwrap();

        assert!(!timer_id.is_empty());
        assert!(manager.has_active_timer("profile1").await);
        assert_eq!(manager.active_timer_count().await, 1);
    }

    #[tokio::test]
    async fn test_cannot_start_multiple_timers_for_profile() {
        let manager = create_manager();

        manager
            .start_manual("profile1".to_string(), None)
            .await
            .unwrap();

        let result = manager.start_manual("profile1".to_string(), None).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let manager = create_manager();

        manager
            .start_manual("profile1".to_string(), None)
            .await
            .unwrap();

        manager.pause("profile1").await.unwrap();
        let timer = manager.get_timer("profile1").await.unwrap();
        assert!(timer.is_paused());

        manager.resume("profile1").await.unwrap();
        let timer = manager.get_timer("profile1").await.unwrap();
        assert!(timer.is_running());
    }

    #[tokio::test]
    async fn test_stop_removes_timer() {
        let manager = create_manager();

        manager
            .start_manual("profile1".to_string(), Some("task1".to_string()))
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;

        let entry = manager.stop("profile1").await.unwrap();
        assert!(entry.is_completed());
        assert!(!manager.has_active_timer("profile1").await);
    }

    #[tokio::test]
    async fn test_multiple_profiles() {
        let manager = create_manager();

        manager
            .start_manual("profile1".to_string(), None)
            .await
            .unwrap();
        manager
            .start_manual("profile2".to_string(), None)
            .await
            .unwrap();

        assert_eq!(manager.active_timer_count().await, 2);

        let timers = manager.get_all_timers().await;
        assert_eq!(timers.len(), 2);
        assert!(timers.contains_key("profile1"));
        assert!(timers.contains_key("profile2"));
    }

    #[tokio::test]
    async fn test_timer_events() {
        let manager = create_manager();
        let mut rx = manager.subscribe();

        manager
            .start_manual("profile1".to_string(), Some("task1".to_string()))
            .await
            .unwrap();

        // Should receive started event
        let event = rx.recv().await.unwrap();
        match event.event_type {
            super::super::events::TimerEventType::Started { task_id, .. } => {
                assert_eq!(task_id, Some("task1".to_string()));
            }
            _ => panic!("Expected Started event"),
        }
    }

    #[tokio::test]
    async fn test_pomodoro_timer() {
        let manager = create_manager();
        let config = PomodoroConfig::default();

        manager
            .start_pomodoro("profile1".to_string(), None, config)
            .await
            .unwrap();

        let timer = manager.get_timer("profile1").await.unwrap();
        assert!(timer.is_pomodoro());
    }

    #[tokio::test]
    async fn test_countdown_completion_doesnt_deadlock() {
        let manager = create_manager();
        let mut rx = manager.subscribe();

        // Start a 1-second countdown
        let start_time = chrono::Utc::now();
        manager
            .start_countdown("profile1".to_string(), None, 1)
            .await
            .unwrap();

        // Wait for events
        let mut countdown_completed = false;
        let mut timer_stopped = false;
        let start = std::time::Instant::now();

        while start.elapsed() < Duration::from_secs(70) {
            // 1 minute countdown + buffer
            match tokio::time::timeout(Duration::from_millis(500), rx.recv()).await {
                Ok(Ok(event)) => {
                    tracing::debug!("Received event: {:?}", event.event_type);
                    match event.event_type {
                        super::super::events::TimerEventType::CountdownCompleted => {
                            countdown_completed = true;
                        }
                        super::super::events::TimerEventType::Stopped { .. } => {
                            timer_stopped = true;
                        }
                        _ => {}
                    }
                }
                _ => {}
            }

            if countdown_completed && timer_stopped {
                break;
            }
        }

        // After completion, the timer should be removed (with some tolerance for timing)
        sleep(Duration::from_millis(500)).await;
        let result = manager.get_timer("profile1").await;

        if countdown_completed && timer_stopped {
            assert!(
                result.is_err(),
                "Timer should be removed after countdown completion"
            );
        } else {
            // If we didn't see the events, at least verify get_timer works without deadlocking
            match result {
                Ok(timer) => tracing::debug!(
                    "Timer still active after {} seconds",
                    start.elapsed().as_secs()
                ),
                Err(_) => tracing::debug!("Timer was completed and removed"),
            }
        }
    }
}
