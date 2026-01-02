//! Timer state models

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{PomodoroConfig, TimerMode};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ActiveTimer {
    pub profile_id: String,
    pub task_id: Option<String>,
    pub mode: TimerMode,
    pub state: TimerState,
    pub start_time: DateTime<Utc>,
    pub pause_time: Option<DateTime<Utc>>,
    pub elapsed_seconds: u64,
    pub pomodoro_state: Option<PomodoroState>,
    pub target_duration: Option<u64>, // For countdown timers (in seconds)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimerState {
    Running,
    Paused,
    Stopped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PomodoroState {
    pub config: PomodoroConfig,
    pub current_session: u32,
    pub phase: PomodoroPhase,
    pub phase_start_time: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PomodoroPhase {
    Work,
    ShortBreak,
    LongBreak,
}

impl ActiveTimer {
    /// Create a new manual timer
    pub fn new_manual(profile_id: String, task_id: Option<String>) -> Self {
        Self {
            profile_id,
            task_id,
            mode: TimerMode::Manual,
            state: TimerState::Running,
            start_time: Utc::now(),
            pause_time: None,
            elapsed_seconds: 0,
            pomodoro_state: None,
            target_duration: None,
        }
    }

    /// Create a new pomodoro timer
    pub fn new_pomodoro(
        profile_id: String,
        task_id: Option<String>,
        config: PomodoroConfig,
    ) -> Self {
        let now = Utc::now();
        Self {
            profile_id,
            task_id,
            mode: TimerMode::Pomodoro,
            state: TimerState::Running,
            start_time: now,
            pause_time: None,
            elapsed_seconds: 0,
            pomodoro_state: Some(PomodoroState {
                config,
                current_session: 1,
                phase: PomodoroPhase::Work,
                phase_start_time: now,
            }),
            target_duration: None,
        }
    }

    /// Create a new countdown timer
    pub fn new_countdown(
        profile_id: String,
        task_id: Option<String>,
        duration_minutes: u64,
    ) -> Self {
        Self {
            profile_id,
            task_id,
            mode: TimerMode::Countdown,
            state: TimerState::Running,
            start_time: Utc::now(),
            pause_time: None,
            elapsed_seconds: 0,
            pomodoro_state: None,
            target_duration: Some(duration_minutes * 60),
        }
    }

    /// Pause the timer
    pub fn pause(&mut self) -> Result<()> {
        if self.state != TimerState::Running {
            return Err(Error::InvalidData("Timer is not running".to_string()));
        }

        self.pause_time = Some(Utc::now());
        self.state = TimerState::Paused;
        Ok(())
    }

    /// Resume the timer
    pub fn resume(&mut self) -> Result<()> {
        if self.state != TimerState::Paused {
            return Err(Error::InvalidData("Timer is not paused".to_string()));
        }

        if let Some(pause_time) = self.pause_time {
            let pause_duration = Utc::now()
                .signed_duration_since(pause_time)
                .num_seconds()
                .max(0) as u64;

            // Adjust start time to account for pause
            self.start_time += chrono::Duration::seconds(pause_duration as i64);

            // Adjust pomodoro phase start time
            if let Some(ref mut pomo) = self.pomodoro_state {
                pomo.phase_start_time += chrono::Duration::seconds(pause_duration as i64);
            }
        }

        self.pause_time = None;
        self.state = TimerState::Running;
        Ok(())
    }

    /// Stop the timer
    pub fn stop(&mut self) {
        self.elapsed_seconds = self.current_elapsed();
        self.state = TimerState::Stopped;
    }

    /// Get current elapsed time in seconds
    pub fn current_elapsed(&self) -> u64 {
        match self.state {
            TimerState::Running => Utc::now()
                .signed_duration_since(self.start_time)
                .num_seconds()
                .max(0) as u64,
            TimerState::Paused => {
                if let Some(pause_time) = self.pause_time {
                    pause_time
                        .signed_duration_since(self.start_time)
                        .num_seconds()
                        .max(0) as u64
                } else {
                    self.elapsed_seconds
                }
            }
            TimerState::Stopped => self.elapsed_seconds,
        }
    }

    /// Get remaining time for pomodoro timers (None for manual timers)
    pub fn remaining_seconds(&self) -> Option<u64> {
        if let Some(ref pomo) = self.pomodoro_state {
            let phase_duration = pomo.phase.duration(&pomo.config);
            let phase_elapsed = Utc::now()
                .signed_duration_since(pomo.phase_start_time)
                .num_seconds()
                .max(0) as u64;

            Some(phase_duration.saturating_sub(phase_elapsed))
        } else {
            None
        }
    }

    /// Check if pomodoro phase is complete
    pub fn is_phase_complete(&self) -> bool {
        if let Some(0) = self.remaining_seconds() {
            return true;
        }
        false
    }

    /// Advance to next pomodoro phase
    pub fn next_phase(&mut self) -> Result<()> {
        let pomo = self
            .pomodoro_state
            .as_mut()
            .ok_or_else(|| Error::InvalidData("Not a pomodoro timer".to_string()))?;

        let (next_phase, next_session) = match pomo.phase {
            PomodoroPhase::Work => {
                if pomo.current_session >= pomo.config.sessions_until_long_break {
                    (PomodoroPhase::LongBreak, pomo.current_session)
                } else {
                    (PomodoroPhase::ShortBreak, pomo.current_session)
                }
            }
            PomodoroPhase::ShortBreak => (PomodoroPhase::Work, pomo.current_session + 1),
            PomodoroPhase::LongBreak => {
                (PomodoroPhase::Work, 1) // Reset session count
            }
        };

        pomo.phase = next_phase;
        pomo.current_session = next_session;
        pomo.phase_start_time = Utc::now();

        Ok(())
    }

    /// Check if this is a pomodoro timer
    pub fn is_pomodoro(&self) -> bool {
        self.pomodoro_state.is_some()
    }

    /// Check if timer is running
    pub fn is_running(&self) -> bool {
        self.state == TimerState::Running
    }

    /// Check if timer is paused
    pub fn is_paused(&self) -> bool {
        self.state == TimerState::Paused
    }

    /// Check if timer is stopped
    pub fn is_stopped(&self) -> bool {
        self.state == TimerState::Stopped
    }
}

impl PomodoroPhase {
    /// Get the duration for this phase based on config
    pub fn duration(&self, config: &PomodoroConfig) -> u64 {
        match self {
            PomodoroPhase::Work => config.work_duration,
            PomodoroPhase::ShortBreak => config.short_break,
            PomodoroPhase::LongBreak => config.long_break,
        }
    }

    /// Get a human-readable string for the phase
    pub fn as_str(&self) -> &'static str {
        match self {
            PomodoroPhase::Work => "Work",
            PomodoroPhase::ShortBreak => "Short Break",
            PomodoroPhase::LongBreak => "Long Break",
        }
    }

    /// Check if this is a work phase
    pub fn is_work(&self) -> bool {
        matches!(self, PomodoroPhase::Work)
    }

    /// Check if this is a break phase
    pub fn is_break(&self) -> bool {
        !self.is_work()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_manual_timer() {
        let timer = ActiveTimer::new_manual("test_profile".to_string(), None);
        assert_eq!(timer.profile_id, "test_profile");
        assert_eq!(timer.mode, TimerMode::Manual);
        assert!(timer.is_running());
        assert!(!timer.is_pomodoro());
    }

    #[test]
    fn test_new_pomodoro_timer() {
        let config = PomodoroConfig::default();
        let timer = ActiveTimer::new_pomodoro(
            "test_profile".to_string(),
            Some("task-123".to_string()),
            config,
        );

        assert!(timer.is_pomodoro());
        assert!(timer.is_running());
        assert!(timer.pomodoro_state.is_some());

        if let Some(ref pomo) = timer.pomodoro_state {
            assert_eq!(pomo.phase, PomodoroPhase::Work);
            assert_eq!(pomo.current_session, 1);
        }
    }

    #[test]
    fn test_pause_resume() {
        let mut timer = ActiveTimer::new_manual("test".to_string(), None);

        assert!(timer.pause().is_ok());
        assert!(timer.is_paused());

        assert!(timer.resume().is_ok());
        assert!(timer.is_running());
    }

    #[test]
    fn test_pause_not_running() {
        let mut timer = ActiveTimer::new_manual("test".to_string(), None);
        timer.stop();

        assert!(timer.pause().is_err());
    }

    #[test]
    fn test_stop_timer() {
        let mut timer = ActiveTimer::new_manual("test".to_string(), None);
        std::thread::sleep(std::time::Duration::from_millis(1100));

        timer.stop();
        assert!(timer.is_stopped());
        assert!(timer.elapsed_seconds > 0);
    }

    #[test]
    fn test_pomodoro_phases() {
        let config = PomodoroConfig::default();
        assert_eq!(PomodoroPhase::Work.duration(&config), 1500);
        assert_eq!(PomodoroPhase::ShortBreak.duration(&config), 300);
        assert_eq!(PomodoroPhase::LongBreak.duration(&config), 900);
    }

    #[test]
    fn test_pomodoro_phase_strings() {
        assert_eq!(PomodoroPhase::Work.as_str(), "Work");
        assert_eq!(PomodoroPhase::ShortBreak.as_str(), "Short Break");
        assert_eq!(PomodoroPhase::LongBreak.as_str(), "Long Break");

        assert!(PomodoroPhase::Work.is_work());
        assert!(PomodoroPhase::ShortBreak.is_break());
    }

    #[test]
    fn test_next_phase() {
        let config = PomodoroConfig {
            work_duration: 1,
            short_break: 1,
            long_break: 1,
            sessions_until_long_break: 2,
        };

        let mut timer = ActiveTimer::new_pomodoro("test".to_string(), None, config);

        // First work -> short break
        timer.next_phase().unwrap();
        assert_eq!(
            timer.pomodoro_state.as_ref().unwrap().phase,
            PomodoroPhase::ShortBreak
        );
        assert_eq!(timer.pomodoro_state.as_ref().unwrap().current_session, 1);

        // Short break -> second work
        timer.next_phase().unwrap();
        assert_eq!(
            timer.pomodoro_state.as_ref().unwrap().phase,
            PomodoroPhase::Work
        );
        assert_eq!(timer.pomodoro_state.as_ref().unwrap().current_session, 2);

        // Second work -> long break
        timer.next_phase().unwrap();
        assert_eq!(
            timer.pomodoro_state.as_ref().unwrap().phase,
            PomodoroPhase::LongBreak
        );

        // Long break -> reset to first work
        timer.next_phase().unwrap();
        assert_eq!(
            timer.pomodoro_state.as_ref().unwrap().phase,
            PomodoroPhase::Work
        );
        assert_eq!(timer.pomodoro_state.as_ref().unwrap().current_session, 1);
    }
}
