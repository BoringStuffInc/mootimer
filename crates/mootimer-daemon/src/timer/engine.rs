//! Timer engine - manages individual timer state and lifecycle

use chrono::Utc;
use mootimer_core::models::{ActiveTimer, Entry, PomodoroConfig};
use std::sync::Arc;
use tokio::sync::{broadcast, RwLock};
use tokio::time::{interval, Duration};

use super::events::{TimerEvent, TimerEventType};

/// Timer engine error
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

/// Timer engine manages the lifecycle of a single timer
pub struct TimerEngine {
    timer: Arc<RwLock<ActiveTimer>>,
    event_tx: broadcast::Sender<TimerEvent>,
    tick_interval: Duration,
}

impl TimerEngine {
    /// Create a new timer engine with a manual timer
    pub fn new_manual(
        profile_id: String,
        task_id: Option<String>,
        event_tx: broadcast::Sender<TimerEvent>,
    ) -> Self {
        let timer = ActiveTimer::new_manual(profile_id, task_id);
        Self {
            timer: Arc::new(RwLock::new(timer)),
            event_tx,
            tick_interval: Duration::from_secs(1),
        }
    }

    /// Create a new timer engine with a pomodoro timer
    pub fn new_pomodoro(
        profile_id: String,
        task_id: Option<String>,
        config: PomodoroConfig,
        event_tx: broadcast::Sender<TimerEvent>,
    ) -> Self {
        let timer = ActiveTimer::new_pomodoro(profile_id, task_id, config);
        Self {
            timer: Arc::new(RwLock::new(timer)),
            event_tx,
            tick_interval: Duration::from_secs(1),
        }
    }

    /// Create a new timer engine with a countdown timer
    pub fn new_countdown(
        profile_id: String,
        task_id: Option<String>,
        duration_minutes: u64,
        event_tx: broadcast::Sender<TimerEvent>,
    ) -> Self {
        let timer = ActiveTimer::new_countdown(profile_id, task_id, duration_minutes);
        Self {
            timer: Arc::new(RwLock::new(timer)),
            event_tx,
            tick_interval: Duration::from_secs(1),
        }
    }

    /// Get the timer ID
    pub async fn timer_id(&self) -> String {
        // For now, use profile_id as timer_id (we'll improve this later)
        let timer = self.timer.read().await;
        timer.profile_id.clone()
    }

    /// Get the timer state
    pub async fn get_timer(&self) -> ActiveTimer {
        tracing::debug!("engine.get_timer: acquiring read lock");
        let timer = self.timer.read().await;
        tracing::debug!("engine.get_timer: got read lock");
        let mut timer_copy = timer.clone();
        // Update elapsed_seconds to current value
        timer_copy.elapsed_seconds = timer.current_elapsed();
        drop(timer);
        tracing::debug!("engine.get_timer: released read lock");
        timer_copy
    }

    /// Start the timer tick loop
    pub async fn start_tick_loop(self: Arc<Self>) {
        let mut tick_interval = interval(self.tick_interval);
        let timer_id = self.timer_id().await;
        tracing::debug!("start_tick_loop: starting for timer {}", timer_id);

        loop {
            tracing::debug!("start_tick_loop: waiting for tick");
            tick_interval.tick().await;
            tracing::debug!("start_tick_loop: got tick");

            let timer = self.timer.read().await;

            // Only tick if running
            if !timer.is_running() {
                continue;
            }

            let elapsed = timer.current_elapsed();
            let remaining = timer.remaining_seconds();
            let profile_id = timer.profile_id.clone();
            let timer_id = profile_id.clone(); // TODO: Use actual timer ID

            drop(timer); // Release read lock before emitting event

            // Emit tick event
            let event = TimerEvent::tick(profile_id.clone(), timer_id.clone(), elapsed, remaining);
            let _ = self.event_tx.send(event);

            // Check for pomodoro phase completion
            let timer = self.timer.read().await;
            if timer.is_pomodoro() && timer.is_phase_complete() {
                let pomo_state = timer.pomodoro_state.as_ref().unwrap();
                let current_phase = pomo_state.phase;
                let current_session = pomo_state.current_session;

                drop(timer);

                // Emit phase completed event
                let event = TimerEvent::new(
                    TimerEventType::PhaseCompleted {
                        phase: current_phase,
                        session_number: current_session,
                    },
                    profile_id.clone(),
                    timer_id.clone(),
                );
                let _ = self.event_tx.send(event);

                // Transition to next phase
                let mut timer = self.timer.write().await;
                if let Err(e) = timer.next_phase() {
                    tracing::error!("Failed to transition to next phase: {}", e);
                    continue;
                }

                let new_phase = timer.pomodoro_state.as_ref().unwrap().phase;
                let new_session = timer.pomodoro_state.as_ref().unwrap().current_session;

                drop(timer);

                // Emit phase changed event
                let event = TimerEvent::phase_changed(profile_id, timer_id, new_phase, new_session);
                let _ = self.event_tx.send(event);
            } else {
                // Drop the read lock if we didn't enter the if block
                // This prevents deadlock when countdown timer tries to acquire write lock
                drop(timer);
            }

            // Check for countdown timer completion
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
                let countdown_timer_id = countdown_profile.clone();
                let target = timer.target_duration.unwrap();
                let elapsed = timer.current_elapsed();
                drop(timer);

                tracing::info!("Countdown completed for {}, elapsed={} target={}", countdown_profile, elapsed, target);

                // Emit countdown completed event
                let event = TimerEvent::new(
                    TimerEventType::CountdownCompleted,
                    countdown_profile.clone(),
                    countdown_timer_id.clone(),
                );
                let _ = self.event_tx.send(event);

                tracing::debug!("Acquiring write lock for timer stop");
                // Auto-stop the timer
                let mut timer_write = self.timer.write().await;
                tracing::debug!("Got write lock, calling stop()");
                timer_write.stop();
                let duration = timer_write.elapsed_seconds;
                drop(timer_write);
                tracing::debug!("Released write lock");

                // Emit stopped event so manager can clean up
                let stopped_event = TimerEvent::stopped(
                    countdown_profile,
                    countdown_timer_id,
                    duration,
                );
                let _ = self.event_tx.send(stopped_event);

                tracing::info!("Countdown tick loop breaking");
                // Exit the tick loop - timer is done
                break;
            }
        }
        tracing::info!("Countdown tick loop ended");
    }

    /// Pause the timer
    pub async fn pause(&self) -> Result<()> {
        let mut timer = self.timer.write().await;
        timer.pause()?;

        let elapsed = timer.current_elapsed();
        let event = TimerEvent::new(
            TimerEventType::Paused {
                elapsed_seconds: elapsed,
            },
            timer.profile_id.clone(),
            timer.profile_id.clone(), // TODO: Use actual timer ID
        );
        let _ = self.event_tx.send(event);

        Ok(())
    }

    /// Resume the timer
    pub async fn resume(&self) -> Result<()> {
        let mut timer = self.timer.write().await;
        timer.resume()?;

        let event = TimerEvent::new(
            TimerEventType::Resumed,
            timer.profile_id.clone(),
            timer.profile_id.clone(), // TODO: Use actual timer ID
        );
        let _ = self.event_tx.send(event);

        Ok(())
    }

    /// Stop the timer and create an entry
    pub async fn stop(&self) -> Result<Entry> {
        let mut timer = self.timer.write().await;
        timer.stop();

        let duration = timer.elapsed_seconds;

        // Create entry from timer
        let entry = Entry::create_completed(
            timer.task_id.clone(),
            timer.start_time,
            Utc::now(),
            timer.mode,
        )?;

        let event = TimerEvent::stopped(
            timer.profile_id.clone(),
            timer.profile_id.clone(), // TODO: Use actual timer ID
            duration,
        );
        let _ = self.event_tx.send(event);

        Ok(entry)
    }

    /// Cancel the timer without creating an entry
    pub async fn cancel(&self) -> Result<()> {
        let mut timer = self.timer.write().await;
        timer.stop();

        let event = TimerEvent::new(
            TimerEventType::Cancelled,
            timer.profile_id.clone(),
            timer.profile_id.clone(), // TODO: Use actual timer ID
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
        let engine =
            TimerEngine::new_manual("test_profile".to_string(), Some("task1".to_string()), tx);

        let timer = engine.get_timer().await;
        assert_eq!(timer.profile_id, "test_profile");
        assert_eq!(timer.task_id, Some("task1".to_string()));
        assert_eq!(timer.mode, TimerMode::Manual);
        assert!(timer.is_running());
    }

    #[tokio::test]
    async fn test_pomodoro_timer_creation() {
        let (tx, _rx) = broadcast::channel(100);
        let config = PomodoroConfig::default();
        let engine = TimerEngine::new_pomodoro("test_profile".to_string(), None, config, tx);

        let timer = engine.get_timer().await;
        assert_eq!(timer.mode, TimerMode::Pomodoro);
        assert!(timer.is_pomodoro());
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let (tx, _rx) = broadcast::channel(100);
        let engine = TimerEngine::new_manual("test".to_string(), None, tx);

        // Pause
        engine.pause().await.unwrap();
        let timer = engine.get_timer().await;
        assert!(timer.is_paused());

        // Resume
        engine.resume().await.unwrap();
        let timer = engine.get_timer().await;
        assert!(timer.is_running());
    }

    #[tokio::test]
    async fn test_stop_creates_entry() {
        let (tx, _rx) = broadcast::channel(100);
        let engine = TimerEngine::new_manual("test".to_string(), Some("task1".to_string()), tx);

        sleep(Duration::from_millis(1100)).await;

        let entry = engine.stop().await.unwrap();
        assert_eq!(entry.task_id, Some("task1".to_string()));
        assert!(entry.is_completed());
        assert!(entry.duration_seconds >= 1);
    }

    #[tokio::test]
    async fn test_timer_events() {
        let (tx, mut rx) = broadcast::channel(100);
        let engine = TimerEngine::new_manual("test".to_string(), None, tx);

        // Pause should emit event
        engine.pause().await.unwrap();
        let event = rx.recv().await.unwrap();
        match event.event_type {
            TimerEventType::Paused { .. } => {}
            _ => panic!("Expected Paused event"),
        }

        // Resume should emit event
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
            60, // 60 minutes - long enough that it won't complete during test
            tx,
        ));

        let engine_clone = engine.clone();

        // Spawn the tick loop
        let tick_task = tokio::spawn(async move {
            engine_clone.start_tick_loop().await;
        });

        // Give it some time to start ticking
        sleep(Duration::from_millis(100)).await;

        // Try to get timer while tick loop is running (this should not deadlock)
        let timer_result = tokio::time::timeout(
            Duration::from_secs(5),
            engine.get_timer(),
        ).await;

        assert!(timer_result.is_ok(), "get_timer should not timeout/deadlock");
        let timer = timer_result.unwrap();
        assert_eq!(timer.mode, mootimer_core::models::TimerMode::Countdown);

        // Try multiple times
        for _ in 0..5 {
            sleep(Duration::from_millis(50)).await;
            let result = tokio::time::timeout(
                Duration::from_secs(5),
                engine.get_timer(),
            ).await;
            assert!(result.is_ok(), "get_timer should not deadlock");
        }

        // Clean up - cancel the timer to stop the tick loop
        engine.cancel().await.unwrap();
        
        let _ = tokio::time::timeout(
            Duration::from_secs(5),
            tick_task,
        ).await;
    }
}
