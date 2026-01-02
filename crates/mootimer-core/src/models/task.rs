//! Task data model

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Task {
    pub id: String,
    pub title: String,
    pub description: Option<String>,
    pub source: TaskSource,
    pub source_id: Option<String>,
    pub url: Option<String>,
    pub status: TaskStatus,
    pub tags: Vec<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TaskSource {
    Manual,
    Jira,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Todo,
    InProgress,
    Done,
}

impl Task {
    /// Create a new manual task with the given title
    pub fn new(title: String) -> Result<Self> {
        let now = Utc::now();
        let task = Self {
            id: Uuid::new_v4().to_string(),
            title,
            description: None,
            source: TaskSource::Manual,
            source_id: None,
            url: None,
            status: TaskStatus::Todo,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        task.validate()?;
        Ok(task)
    }

    /// Create a new task from JIRA issue
    pub fn from_jira(jira_key: String, title: String) -> Result<Self> {
        let now = Utc::now();
        let task = Self {
            id: jira_key.clone(),
            title,
            description: None,
            source: TaskSource::Jira,
            source_id: Some(jira_key),
            url: None,
            status: TaskStatus::Todo,
            tags: Vec::new(),
            created_at: now,
            updated_at: now,
        };
        task.validate()?;
        Ok(task)
    }

    /// Validate the task data
    pub fn validate(&self) -> Result<()> {
        if self.title.trim().is_empty() {
            return Err(Error::Validation("Task title cannot be empty".to_string()));
        }

        if self.id.trim().is_empty() {
            return Err(Error::Validation("Task ID cannot be empty".to_string()));
        }

        // Validate URL format if present
        if let Some(ref url) = self.url {
            if !url.starts_with("http://") && !url.starts_with("https://") {
                return Err(Error::Validation(
                    "Task URL must start with http:// or https://".to_string(),
                ));
            }
        }

        Ok(())
    }

    /// Update the task's title
    pub fn update_title(&mut self, title: String) -> Result<()> {
        if title.trim().is_empty() {
            return Err(Error::Validation("Task title cannot be empty".to_string()));
        }
        self.title = title;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update the task's description
    pub fn update_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    /// Update the task's status
    pub fn update_status(&mut self, status: TaskStatus) {
        self.status = status;
        self.updated_at = Utc::now();
    }

    /// Update the task's URL
    pub fn update_url(&mut self, url: Option<String>) -> Result<()> {
        if let Some(ref u) = url {
            if !u.starts_with("http://") && !u.starts_with("https://") {
                return Err(Error::Validation(
                    "Task URL must start with http:// or https://".to_string(),
                ));
            }
        }
        self.url = url;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Add a tag to the task
    pub fn add_tag(&mut self, tag: String) {
        if !self.tags.contains(&tag) {
            self.tags.push(tag);
            self.updated_at = Utc::now();
        }
    }

    /// Remove a tag from the task
    pub fn remove_tag(&mut self, tag: &str) {
        if let Some(pos) = self.tags.iter().position(|t| t == tag) {
            self.tags.remove(pos);
            self.updated_at = Utc::now();
        }
    }

    /// Check if the task has a specific tag
    pub fn has_tag(&self, tag: &str) -> bool {
        self.tags.iter().any(|t| t == tag)
    }

    /// Mark task as in progress
    pub fn start(&mut self) {
        self.status = TaskStatus::InProgress;
        self.updated_at = Utc::now();
    }

    /// Mark task as done
    pub fn complete(&mut self) {
        self.status = TaskStatus::Done;
        self.updated_at = Utc::now();
    }

    /// Mark task as todo
    pub fn reset(&mut self) {
        self.status = TaskStatus::Todo;
        self.updated_at = Utc::now();
    }

    /// Check if the task is from JIRA
    pub fn is_jira_task(&self) -> bool {
        self.source == TaskSource::Jira
    }

    /// Check if the task is completed
    pub fn is_completed(&self) -> bool {
        self.status == TaskStatus::Done
    }

    /// Mark the task as updated (updates the timestamp)
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl TaskStatus {
    /// Get a human-readable string for the status
    pub fn as_str(&self) -> &'static str {
        match self {
            TaskStatus::Todo => "To Do",
            TaskStatus::InProgress => "In Progress",
            TaskStatus::Done => "Done",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_task() {
        let task = Task::new("Test Task".to_string()).unwrap();
        assert_eq!(task.title, "Test Task");
        assert_eq!(task.source, TaskSource::Manual);
        assert_eq!(task.status, TaskStatus::Todo);
        assert!(task.tags.is_empty());
    }

    #[test]
    fn test_new_task_empty_title() {
        let result = Task::new("".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_from_jira() {
        let task = Task::from_jira("PROJ-123".to_string(), "JIRA Task".to_string()).unwrap();
        assert_eq!(task.id, "PROJ-123");
        assert_eq!(task.title, "JIRA Task");
        assert_eq!(task.source, TaskSource::Jira);
        assert_eq!(task.source_id, Some("PROJ-123".to_string()));
        assert!(task.is_jira_task());
    }

    #[test]
    fn test_update_title() {
        let mut task = Task::new("Test".to_string()).unwrap();
        task.update_title("Updated Title".to_string()).unwrap();
        assert_eq!(task.title, "Updated Title");
    }

    #[test]
    fn test_update_status() {
        let mut task = Task::new("Test".to_string()).unwrap();
        assert_eq!(task.status, TaskStatus::Todo);

        task.start();
        assert_eq!(task.status, TaskStatus::InProgress);

        task.complete();
        assert_eq!(task.status, TaskStatus::Done);
        assert!(task.is_completed());

        task.reset();
        assert_eq!(task.status, TaskStatus::Todo);
    }

    #[test]
    fn test_tags() {
        let mut task = Task::new("Test".to_string()).unwrap();

        task.add_tag("backend".to_string());
        task.add_tag("urgent".to_string());
        assert_eq!(task.tags.len(), 2);
        assert!(task.has_tag("backend"));
        assert!(task.has_tag("urgent"));

        // Adding duplicate tag should not increase count
        task.add_tag("backend".to_string());
        assert_eq!(task.tags.len(), 2);

        task.remove_tag("backend");
        assert_eq!(task.tags.len(), 1);
        assert!(!task.has_tag("backend"));
    }

    #[test]
    fn test_update_url_valid() {
        let mut task = Task::new("Test".to_string()).unwrap();
        task.update_url(Some("https://example.com".to_string()))
            .unwrap();
        assert_eq!(task.url, Some("https://example.com".to_string()));
    }

    #[test]
    fn test_update_url_invalid() {
        let mut task = Task::new("Test".to_string()).unwrap();
        let result = task.update_url(Some("not-a-url".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_task_status_as_str() {
        assert_eq!(TaskStatus::Todo.as_str(), "To Do");
        assert_eq!(TaskStatus::InProgress.as_str(), "In Progress");
        assert_eq!(TaskStatus::Done.as_str(), "Done");
    }
}
