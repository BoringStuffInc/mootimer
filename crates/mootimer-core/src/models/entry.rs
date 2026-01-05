use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Entry {
    pub id: String,
    pub task_id: Option<String>,
    pub task_title: Option<String>,
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
    pub fn new(task_id: Option<String>, task_title: Option<String>, mode: TimerMode) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            task_id,
            task_title,
            start_time: Utc::now(),
            end_time: None,
            duration_seconds: 0,
            mode,
            description: None,
            tags: Vec::new(),
        }
    }

    pub fn create_completed(
        task_id: Option<String>,
        task_title: Option<String>,
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
            task_title,
            start_time,
            end_time: Some(end_time),
            duration_seconds: duration,
            mode,
            description: None,
            tags: Vec::new(),
        })
    }

    pub fn finish(&mut self) {
        let end_time = Utc::now();
        let duration = end_time
            .signed_duration_since(self.start_time)
            .num_seconds()
            .max(0) as u64;

        self.end_time = Some(end_time);
        self.duration_seconds = duration;
    }

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

    pub fn validate(&self) -> Result<()> {
        if let Some(end_time) = self.end_time
            && end_time <= self.start_time
        {
            return Err(Error::Validation(
                "End time must be after start time".to_string(),
            ));
        }

        Ok(())
    }

    pub fn is_completed(&self) -> bool {
        self.end_time.is_some()
    }

    pub fn is_active(&self) -> bool {
        self.end_time.is_none()
    }

    pub fn duration_formatted(&self) -> String {
        let hours = self.duration_seconds / 3600;
        let minutes = (self.duration_seconds % 3600) / 60;
        let seconds = self.duration_seconds % 60;
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    pub fn duration_minutes(&self) -> u64 {
        (self.duration_seconds + 30) / 60
    }

    pub fn duration_hours(&self) -> f64 {
        (self.duration_seconds as f64 / 3600.0 * 100.0).round() / 100.0
    }

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

    pub fn update_description(&mut self, description: Option<String>) {
        self.description = description;
    }

    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
        }
    }

    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
        }
    }

    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }
}

impl TimerMode {
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
        let entry = Entry::new(
            Some("task-123".to_string()),
            Some("Task Title".to_string()),
            TimerMode::Manual,
        );
        assert_eq!(entry.task_id, Some("task-123".to_string()));
        assert_eq!(entry.task_title, Some("Task Title".to_string()));
        assert_eq!(entry.mode, TimerMode::Manual);
        assert!(entry.is_active());
        assert!(!entry.is_completed());
    }

    #[test]
    fn test_finish_entry() {
        let mut entry = Entry::new(None, None, TimerMode::Pomodoro);
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

        let entry = Entry::create_completed(
            Some("task-123".to_string()),
            Some("Title".to_string()),
            start,
            end,
            TimerMode::Manual,
        )
        .unwrap();

        assert_eq!(entry.duration_seconds, 7200);
        assert!(entry.is_completed());
    }

    #[test]
    fn test_create_completed_invalid_times() {
        let start = Utc::now();
        let end = start - ChronoDuration::hours(1);

        let result = Entry::create_completed(None, None, start, end, TimerMode::Manual);

        assert!(result.is_err());
    }

    #[test]
    fn test_duration_formatting() {
        let start = Utc::now();
        let end = start + ChronoDuration::seconds(3665);

        let entry = Entry::create_completed(None, None, start, end, TimerMode::Manual).unwrap();
        assert_eq!(entry.duration_formatted(), "01:01:05");
        assert_eq!(entry.duration_minutes(), 61);
        assert_eq!(entry.duration_hours(), 1.02);
    }

    #[test]
    fn test_tags() {
        let mut entry = Entry::new(None, None, TimerMode::Manual);

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
        let entry = Entry::new(None, None, TimerMode::Manual);
        std::thread::sleep(std::time::Duration::from_secs(1));

        let elapsed = entry.current_elapsed_seconds();
        assert!(elapsed >= 1);
    }
}
