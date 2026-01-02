//! Time entry data model

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub id: String,
    pub task_id: Option<String>,
    pub start_time: DateTime<Utc>,
    pub end_time: Option<DateTime<Utc>>,
    pub duration_seconds: u64,
    pub mode: TimerMode,
    pub description: Option<String>,
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TimerMode {
    Manual,
    Pomodoro,
    Countdown,
}

impl Entry {
    /// Create a new time entry
    pub fn new(task_id: Option<String>, mode: TimerMode) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            start_time: Utc::now(),
            end_time: None,
            duration_seconds: 0,
            mode,
            description: None,
            tags: Vec::new(),
        }
    }

    /// Create a completed entry with specific times
    pub fn create_completed(
        task_id: Option<String>,
        start_time: DateTime<Utc>,
        end_time: DateTime<Utc>,
        mode: TimerMode,
    ) -> Result<Self> {
        if end_time <= start_time {
            return Err(Error::Validation(
                "End time must be after start time".to_string(),
            ));
        }

        let duration = end_time
            .signed_duration_since(start_time)
            .num_seconds()
            .max(0) as u64;

        Ok(Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            start_time,
            end_time: Some(end_time),
            duration_seconds: duration,
            mode,
            description: None,
            tags: Vec::new(),
        })
    }

    /// Finish the entry (set end time and calculate duration)
    pub fn finish(&mut self) {
        let end_time = Utc::now();
        let duration = end_time
            .signed_duration_since(self.start_time)
            .num_seconds()
            .max(0) as u64;

        self.end_time = Some(end_time);
        self.duration_seconds = duration;
    }

    /// Finish the entry at a specific time
    pub fn finish_at(&mut self, end_time: DateTime<Utc>) -> Result<()> {
        if end_time <= self.start_time {
            return Err(Error::Validation(
                "End time must be after start time".to_string(),
            ));
        }

        let duration = end_time
            .signed_duration_since(self.start_time)
            .num_seconds()
            .max(0) as u64;

        self.end_time = Some(end_time);
        self.duration_seconds = duration;
        Ok(())
    }

    /// Validate the entry data
    pub fn validate(&self) -> Result<()> {
        if let Some(end_time) = self.end_time {
            if end_time <= self.start_time {
                return Err(Error::Validation(
                    "End time must be after start time".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Check if the entry is completed (has end time)
    pub fn is_completed(&self) -> bool {
        self.end_time.is_some()
    }

    /// Check if the entry is currently active
    pub fn is_active(&self) -> bool {
        self.end_time.is_none()
    }

    /// Get the duration as a formatted string (HH:MM:SS)
    pub fn duration_formatted(&self) -> String {
        let hours = self.duration_seconds / 3600;
        let minutes = (self.duration_seconds % 3600) / 60;
        let seconds = self.duration_seconds % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    /// Get the duration in minutes (rounded)
    pub fn duration_minutes(&self) -> u64 {
        (self.duration_seconds + 30) / 60 // Round to nearest minute
    }

    /// Get the duration in hours (rounded to 2 decimal places)
    pub fn duration_hours(&self) -> f64 {
        (self.duration_seconds as f64 / 3600.0 * 100.0).round() / 100.0
    }

    /// Calculate current elapsed time for active entries
    pub fn current_elapsed_seconds(&self) -> u64 {
        if let Some(end_time) = self.end_time {
            end_time
                .signed_duration_since(self.start_time)
                .num_seconds()
                .max(0) as u64
        } else {
            Utc::now()
                .signed_duration_since(self.start_time)
                .num_seconds()
                .max(0) as u64
        }
    }

    /// Update the entry's description
    pub fn update_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    /// Add a tag to the entry
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    /// Remove a tag from the entry
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
        }
    }

    /// Check if the entry has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

impl TimerMode {
    /// Get a human-readable string for the timer mode
    pub fn as_str(&self) -> &'static str {
        match self {
            TimerMode::Manual => "Manual",
            TimerMode::Pomodoro => "Pomodoro",
            TimerMode::Countdown => "Countdown",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Duration as ChronoDuration;

    #[test]
    fn test_new_entry() {
        let entry = Entry::new(Some("task-123".to_string()), TimerMode::Manual);
        assert_eq!(entry.task_id, Some("task-123".to_string()));
        assert_eq!(entry.mode, TimerMode::Manual);
        assert!(entry.is_active());
        assert!(!entry.is_completed());
    }

    #[test]
    fn test_finish_entry() {
        let mut entry = Entry::new(None, TimerMode::Pomodoro);
        std::thread::sleep(std::time::Duration::from_secs(1));
        entry.finish();

        assert!(entry.is_completed());
        assert!(!entry.is_active());
        assert!(entry.duration_seconds > 0);
        assert!(entry.end_time.is_some());
    }

    #[test]
    fn test_create_completed() {
        let start = Utc::now();
        let end = start + ChronoDuration::hours(2);

        let entry =
            Entry::create_completed(Some("task-123".to_string()), start, end, TimerMode::Manual)
                .unwrap();

        assert_eq!(entry.duration_seconds, 7200); // 2 hours
        assert!(entry.is_completed());
    }

    #[test]
    fn test_create_completed_invalid_times() {
        let start = Utc::now();
        let end = start - ChronoDuration::hours(1); // End before start

        let result = Entry::create_completed(None, start, end, TimerMode::Manual);

        assert!(result.is_err());
    }

    #[test]
    fn test_duration_formatting() {
        let start = Utc::now();
        let end = start + ChronoDuration::seconds(3665); // 1h 1m 5s

        let entry = Entry::create_completed(None, start, end, TimerMode::Manual).unwrap();
        assert_eq!(entry.duration_formatted(), "01:01:05");
        assert_eq!(entry.duration_minutes(), 61);
        assert_eq!(entry.duration_hours(), 1.02);
    }

    #[test]
    fn test_tags() {
        let mut entry = Entry::new(None, TimerMode::Manual);

        entry.add_tag("focus".to_string());
        entry.add_tag("important".to_string());
        assert_eq!(entry.tags.len(), 2);
        assert!(entry.has_tag("focus"));

        entry.remove_tag("focus");
        assert_eq!(entry.tags.len(), 1);
        assert!(!entry.has_tag("focus"));
    }

    #[test]
    fn test_timer_mode_as_str() {
        assert_eq!(TimerMode::Manual.as_str(), "Manual");
        assert_eq!(TimerMode::Pomodoro.as_str(), "Pomodoro");
    }

    #[test]
    fn test_current_elapsed_seconds() {
        let entry = Entry::new(None, TimerMode::Manual);
        std::thread::sleep(std::time::Duration::from_secs(1));

        let elapsed = entry.current_elapsed_seconds();
        assert!(elapsed >= 1);
    }
}
