use chrono::{DateTime, Utc};
use mootimer_core::models::{PomodoroPhase, TimerMode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TimerEvent {
    pub event_type: TimerEventType,
    pub profile_id: String,
    pub timer_id: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TimerEventType {
    Started {
        task_id: Option<String>,
        mode: TimerMode,
    },
    Stopped {
        duration_seconds: u64,
    },
    Paused {
        elapsed_seconds: u64,
    },
    Resumed,
    Cancelled,
    Tick {
        elapsed_seconds: u64,
        remaining_seconds: Option<u64>,
    },
    PhaseChanged {
        new_phase: PomodoroPhase,
        session_number: u32,
    },
    PhaseCompleted {
        phase: PomodoroPhase,
        session_number: u32,
    },
    CountdownCompleted,
}

impl TimerEvent {
    pub fn new(event_type: TimerEventType, profile_id: String, timer_id: String) -> Self {
        Self {
            event_type,
            profile_id,
            timer_id,
            timestamp: Utc::now(),
        }
    }

    pub fn started(
        profile_id: String,
        timer_id: String,
        task_id: Option<String>,
        mode: TimerMode,
    ) -> Self {
        Self::new(
            TimerEventType::Started { task_id, mode },
            profile_id,
            timer_id,
        )
    }

    pub fn stopped(profile_id: String, timer_id: String, duration_seconds: u64) -> Self {
        Self::new(
            TimerEventType::Stopped { duration_seconds },
            profile_id,
            timer_id,
        )
    }

    pub fn tick(
        profile_id: String,
        timer_id: String,
        elapsed_seconds: u64,
        remaining_seconds: Option<u64>,
    ) -> Self {
        Self::new(
            TimerEventType::Tick {
                elapsed_seconds,
                remaining_seconds,
            },
            profile_id,
            timer_id,
        )
    }

    pub fn phase_changed(
        profile_id: String,
        timer_id: String,
        new_phase: PomodoroPhase,
        session_number: u32,
    ) -> Self {
        Self::new(
            TimerEventType::PhaseChanged {
                new_phase,
                session_number,
            },
            profile_id,
            timer_id,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timer_event_creation() {
        let event = TimerEvent::started(
            "profile1".to_string(),
            "timer1".to_string(),
            Some("task1".to_string()),
            TimerMode::Manual,
        );

        assert_eq!(event.profile_id, "profile1");
        assert_eq!(event.timer_id, "timer1");
        match event.event_type {
            TimerEventType::Started { task_id, mode } => {
                assert_eq!(task_id, Some("task1".to_string()));
                assert_eq!(mode, TimerMode::Manual);
            }
            _ => panic!("Wrong event type"),
        }
    }
}
