
use chrono::{DateTime, Utc};
use mootimer_core::models::{Entry, Profile, Task};
use serde::{Deserialize, Serialize};

use crate::timer::TimerEvent;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "category", rename_all = "snake_case")]
pub enum DaemonEvent {
    Timer(TimerEvent),
    Task(TaskEvent),
    Entry(EntryEvent),
    Profile(ProfileEvent),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskEvent {
    pub event_type: TaskEventType,
    pub profile_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub task: Option<Task>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum TaskEventType {
    Created,
    Updated,
    Deleted { task_id: String },
}

impl TaskEvent {
    pub fn created(profile_id: String, task: Task) -> Self {
        Self {
            event_type: TaskEventType::Created,
            profile_id,
            task: Some(task),
            timestamp: Utc::now(),
        }
    }

    pub fn updated(profile_id: String, task: Task) -> Self {
        Self {
            event_type: TaskEventType::Updated,
            profile_id,
            task: Some(task),
            timestamp: Utc::now(),
        }
    }

    pub fn deleted(profile_id: String, task_id: String) -> Self {
        Self {
            event_type: TaskEventType::Deleted { task_id },
            profile_id,
            task: None,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntryEvent {
    pub event_type: EntryEventType,
    pub profile_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub entry: Option<Entry>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum EntryEventType {
    Added,
    Updated,
    Deleted { entry_id: String },
}

impl EntryEvent {
    pub fn added(profile_id: String, entry: Entry) -> Self {
        Self {
            event_type: EntryEventType::Added,
            profile_id,
            entry: Some(entry),
            timestamp: Utc::now(),
        }
    }

    pub fn updated(profile_id: String, entry: Entry) -> Self {
        Self {
            event_type: EntryEventType::Updated,
            profile_id,
            entry: Some(entry),
            timestamp: Utc::now(),
        }
    }

    pub fn deleted(profile_id: String, entry_id: String) -> Self {
        Self {
            event_type: EntryEventType::Deleted { entry_id },
            profile_id,
            entry: None,
            timestamp: Utc::now(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileEvent {
    pub event_type: ProfileEventType,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile: Option<Profile>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ProfileEventType {
    Created,
    Updated,
    Deleted { profile_id: String },
}

impl ProfileEvent {
    pub fn created(profile: Profile) -> Self {
        Self {
            event_type: ProfileEventType::Created,
            profile: Some(profile),
            timestamp: Utc::now(),
        }
    }

    pub fn updated(profile: Profile) -> Self {
        Self {
            event_type: ProfileEventType::Updated,
            profile: Some(profile),
            timestamp: Utc::now(),
        }
    }

    pub fn deleted(profile_id: String) -> Self {
        Self {
            event_type: ProfileEventType::Deleted { profile_id },
            profile: None,
            timestamp: Utc::now(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mootimer_core::models::{TaskSource, TaskStatus, TimerMode};

    #[test]
    fn test_task_event_serialization() {
        let task = Task {
            id: "task1".to_string(),
            title: "Test Task".to_string(),
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
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: TaskEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.profile_id, "profile1");
        assert!(matches!(deserialized.event_type, TaskEventType::Created));
    }

    #[test]
    fn test_entry_event_serialization() {
        let entry = Entry {
            id: "entry1".to_string(),
            start_time: Utc::now(),
            end_time: Some(Utc::now()),
            duration_seconds: 60,
            mode: TimerMode::Manual,
            task_id: None,
            description: None,
            tags: vec![],
        };

        let event = EntryEvent::added("profile1".to_string(), entry);
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: EntryEvent = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.profile_id, "profile1");
        assert!(matches!(deserialized.event_type, EntryEventType::Added));
    }

    #[test]
    fn test_profile_event_serialization() {
        let profile = Profile {
            id: "profile1".to_string(),
            name: "Test Profile".to_string(),
            description: None,
            color: None,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };

        let event = ProfileEvent::created(profile);
        let json = serde_json::to_string(&event).unwrap();
        let deserialized: ProfileEvent = serde_json::from_str(&json).unwrap();

        assert!(matches!(deserialized.event_type, ProfileEventType::Created));
    }
}
