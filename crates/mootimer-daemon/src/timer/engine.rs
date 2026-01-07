use chrono::Utc;
use mootimer_core::models::{ActiveTimer, Entry, PomodoroConfig};
use std::sync::Arc;
use tokio::sync::{RwLock, broadcast};
use tokio::time::{Duration, interval};

use super::events::{TimerEvent, TimerEventType};

#[derive(Debug, thiserror::Error)]
pub enum TimerEngineError {
    #[error("Timer not found: {0}")]
    NotFound(String),

    #[error("Timer already running")]
    AlreadyRunning,

    #[error("Invalid operation: {0}")]
    InvalidOperation(String),

    #[error("Core error: {0}")]
    Core(#[from] mootimer_core::Error),
}

pub type Result<T> = std::result::Result<T, TimerEngineError>;

pub struct TimerEngine {
    timer: Arc<RwLock<ActiveTimer>>,
    event_tx: broadcast::Sender<TimerEvent>,
    tick_interval: Duration,
}

impl TimerEngine {
    pub fn new_manual(
        profile_id: String,
        task_id: Option<String>,
        task_title: Option<String>,
        event_tx: broadcast::Sender<TimerEvent>,
    ) -> Self {
        let timer = ActiveTimer::new_manual(profile_id, task_id, task_title);
        Self {
            timer: Arc::new(RwLock::new(timer)),
            event_tx,
            tick_interval: Duration::from_secs(1),
        }
    }

    pub fn new_pomodoro(
        profile_id: String,
        task_id: Option<String>,
        task_title: Option<String>,
        config: PomodoroConfig,
        event_tx: broadcast::Sender<TimerEvent>,
    ) -> Self {
        let timer = ActiveTimer::new_pomodoro(profile_id, task_id, task_title, config);
        Self {
            timer: Arc::new(RwLock::new(timer)),
            event_tx,
            tick_interval: Duration::from_secs(1),
        }
    }

    pub fn new_countdown(
        profile_id: String,
        task_id: Option<String>,
        task_title: Option<String>,
        duration_minutes: u64,
        event_tx: broadcast::Sender<TimerEvent>,
    ) -> Self {
        let timer = ActiveTimer::new_countdown(profile_id, task_id, task_title, duration_minutes);
        Self {
            timer: Arc::new(RwLock::new(timer)),
            event_tx,
            tick_interval: Duration::from_secs(1),
        }
    }

    pub async fn timer_id(&self) -> String {
        let timer = self.timer.read().await;
        timer.id.clone()
    }

    pub async fn profile_id(&self) -> String {
        let timer = self.timer.read().await;
        timer.profile_id.clone()
    }

    pub async fn get_timer(&self) -> ActiveTimer {
        let timer = self.timer.read().await;
        let mut timer_copy = timer.clone();
        timer_copy.elapsed_seconds = timer.current_elapsed();
        drop(timer);
        timer_copy
    }

    pub async fn start_tick_loop(self: Arc<Self>) {
        let mut tick_interval = interval(self.tick_interval);

        loop {
            tick_interval.tick().await;

            let timer = self.timer.read().await;

            if !timer.is_running() {
                continue;
            }

            let elapsed = timer.current_elapsed();
            let remaining = timer.remaining_seconds();
            let profile_id = timer.profile_id.clone();
            let timer_id = timer.id.clone();

            drop(timer);

            let event = TimerEvent::tick(profile_id.clone(), timer_id.clone(), elapsed, remaining);
            let _ = self.event_tx.send(event);

            let timer = self.timer.read().await;
            if timer.is_pomodoro() && timer.is_phase_complete() {
                let Some(pomo_state) = timer.pomodoro_state.as_ref() else {
                    drop(timer);
                    continue;
                };
                let current_phase = pomo_state.phase;
                let current_session = pomo_state.current_session;

                drop(timer);

                let event = TimerEvent::new(
                    TimerEventType::PhaseCompleted {
                        phase: current_phase,
                        session_number: current_session,
                    },
                    profile_id.clone(),
                    timer_id.clone(),
                );
                let _ = self.event_tx.send(event);

                let mut timer = self.timer.write().await;
                if let Err(e) = timer.next_phase() {
                    tracing::error!("Failed to transition to next phase: {}", e);
                    continue;
                }

                let Some(pomo_state) = timer.pomodoro_state.as_ref() else {
                    drop(timer);
                    continue;
                };
                let new_phase = pomo_state.phase;
                let new_session = pomo_state.current_session;

                drop(timer);

                let event = TimerEvent::phase_changed(profile_id, timer_id, new_phase, new_session);
                let _ = self.event_tx.send(event);
            } else {
                drop(timer);
            }

            let should_complete = {
                let timer = self.timer.read().await;
                if timer.mode == mootimer_core::models::TimerMode::Countdown {
                    if let Some(target) = timer.target_duration {
                        let elapsed = timer.current_elapsed();
                        elapsed >= target
                    } else {
                        false
                    }
                } else {
                    false
                }
            };

            if should_complete {
                let timer = self.timer.read().await;
                let countdown_profile = timer.profile_id.clone();
                let countdown_timer_id = timer.id.clone();
                let Some(target) = timer.target_duration else {
                    drop(timer);
                    continue;
                };
                let elapsed = timer.current_elapsed();
                drop(timer);

                tracing::info!(
                    "Countdown completed for {}, elapsed={} target={}",
                    countdown_profile,
                    elapsed,
                    target
                );

                let event = TimerEvent::new(
                    TimerEventType::CountdownCompleted,
                    countdown_profile.clone(),
                    countdown_timer_id.clone(),
                );
                let _ = self.event_tx.send(event);

                let mut timer_write = self.timer.write().await;
                timer_write.stop();
                let duration = timer_write.elapsed_seconds;
                drop(timer_write);

                let stopped_event =
                    TimerEvent::stopped(countdown_profile, countdown_timer_id, duration);
                let _ = self.event_tx.send(stopped_event);

                break;
            }
        }
    }

    pub async fn pause(&self) -> Result<()> {
        let mut timer = self.timer.write().await;
        timer.pause()?;

        let elapsed = timer.current_elapsed();
        let event = TimerEvent::new(
            TimerEventType::Paused {
                elapsed_seconds: elapsed,
            },
            timer.profile_id.clone(),
            timer.id.clone(),
        );
        let _ = self.event_tx.send(event);

        Ok(())
    }

    pub async fn resume(&self) -> Result<()> {
        let mut timer = self.timer.write().await;
        timer.resume()?;

        let event = TimerEvent::new(
            TimerEventType::Resumed,
            timer.profile_id.clone(),
            timer.id.clone(),
        );
        let _ = self.event_tx.send(event);

        Ok(())
    }

    pub async fn stop(&self) -> Result<Entry> {
        let mut timer = self.timer.write().await;
        timer.stop();

        let duration = timer.elapsed_seconds;

        let entry = Entry::create_completed(
            timer.task_id.clone(),
            timer.task_title.clone(),
            timer.start_time,
            Utc::now(),
            timer.mode,
        )?;

        let event = TimerEvent::stopped(timer.profile_id.clone(), timer.id.clone(), duration);
        let _ = self.event_tx.send(event);

        Ok(entry)
    }

    pub async fn cancel(&self) -> Result<()> {
        let mut timer = self.timer.write().await;
        timer.stop();

        let event = TimerEvent::new(
            TimerEventType::Cancelled,
            timer.profile_id.clone(),
            timer.id.clone(),
        );
        let _ = self.event_tx.send(event);

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mootimer_core::models::TimerMode;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_manual_timer_creation() {
        let (tx, _rx) = broadcast::channel(100);
        let engine = TimerEngine::new_manual(
            "test_profile".to_string(),
            Some("task1".to_string()),
            Some("Task Title".to_string()),
            tx,
        );

        let timer = engine.get_timer().await;
        assert_eq!(timer.profile_id, "test_profile");
        assert_eq!(timer.task_id, Some("task1".to_string()));
        assert_eq!(timer.task_title, Some("Task Title".to_string()));
        assert_eq!(timer.mode, TimerMode::Manual);
        assert!(timer.is_running());
    }

    #[tokio::test]
    async fn test_pomodoro_timer_creation() {
        let (tx, _rx) = broadcast::channel(100);
        let config = PomodoroConfig::default();
        let engine = TimerEngine::new_pomodoro("test_profile".to_string(), None, None, config, tx);

        let timer = engine.get_timer().await;
        assert_eq!(timer.mode, TimerMode::Pomodoro);
        assert!(timer.is_pomodoro());
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let (tx, _rx) = broadcast::channel(100);
        let engine = TimerEngine::new_manual("test".to_string(), None, None, tx);

        engine.pause().await.unwrap();
        let timer = engine.get_timer().await;
        assert!(timer.is_paused());

        engine.resume().await.unwrap();
        let timer = engine.get_timer().await;
        assert!(timer.is_running());
    }

    #[tokio::test]
    async fn test_stop_creates_entry() {
        let (tx, _rx) = broadcast::channel(100);
        let engine = TimerEngine::new_manual(
            "test".to_string(),
            Some("task1".to_string()),
            Some("Task Title".to_string()),
            tx,
        );

        sleep(Duration::from_millis(1100)).await;

        let entry = engine.stop().await.unwrap();
        assert_eq!(entry.task_id, Some("task1".to_string()));
        assert_eq!(entry.task_title, Some("Task Title".to_string()));
        assert!(entry.is_completed());
        assert!(entry.duration_seconds >= 1);
    }

    #[tokio::test]
    async fn test_timer_events() {
        let (tx, mut rx) = broadcast::channel(100);
        let engine = TimerEngine::new_manual("test".to_string(), None, None, tx);

        engine.pause().await.unwrap();
        let event = rx.recv().await.unwrap();
        match event.event_type {
            TimerEventType::Paused { .. } => {}
            _ => panic!("Expected Paused event"),
        }

        engine.resume().await.unwrap();
        let event = rx.recv().await.unwrap();
        match event.event_type {
            TimerEventType::Resumed => {}
            _ => panic!("Expected Resumed event"),
        }
    }

    #[tokio::test]
    async fn test_countdown_tick_loop_doesnt_deadlock_on_get_timer() {
        use std::sync::Arc;

        let (tx, _) = broadcast::channel(100);
        let engine = Arc::new(TimerEngine::new_countdown(
            "test_profile".to_string(),
            None,
            None,
            60,
            tx,
        ));

        let engine_clone = engine.clone();

        let tick_task = tokio::spawn(async move {
            engine_clone.start_tick_loop().await;
        });

        sleep(Duration::from_millis(100)).await;

        let timer_result = tokio::time::timeout(Duration::from_secs(5), engine.get_timer()).await;

        assert!(
            timer_result.is_ok(),
            "get_timer should not timeout/deadlock"
        );
        let timer = timer_result.unwrap();
        assert_eq!(timer.mode, mootimer_core::models::TimerMode::Countdown);

        for _ in 0..5 {
            sleep(Duration::from_millis(50)).await;
            let result = tokio::time::timeout(Duration::from_secs(5), engine.get_timer()).await;
            assert!(result.is_ok(), "get_timer should not deadlock");
        }

        engine.cancel().await.unwrap();

        let _ = tokio::time::timeout(Duration::from_secs(5), tick_task).await;
    }
}
