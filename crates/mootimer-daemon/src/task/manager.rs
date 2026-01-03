//! Task manager for CRUD operations

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::event_manager::EventManager;
use crate::events::TaskEvent;
use mootimer_core::{
    Result as CoreResult,
    models::Task,
    storage::{TaskStorage, init_data_dir},
};

/// Task manager error
#[derive(Debug, thiserror::Error)]
pub enum TaskManagerError {
    #[error("Task not found: {0}")]
    NotFound(String),

    #[error("Storage error: {0}")]
    Storage(#[from] mootimer_core::Error),

    #[error("Invalid task: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, TaskManagerError>;

/// Manages tasks with caching and persistence (per profile)
pub struct TaskManager {
    storage: TaskStorage,
    /// Cache: profile_id -> task_id -> Task
    cache: Arc<RwLock<HashMap<String, HashMap<String, Task>>>>,
    /// Event manager for broadcasting events
    event_manager: Arc<EventManager>,
}

impl TaskManager {
    pub fn new(event_manager: Arc<EventManager>) -> CoreResult<Self> {
        let data_dir = init_data_dir()?;
        let storage = TaskStorage::new(data_dir);

        Ok(Self {
            storage,
            cache: Arc::new(RwLock::new(HashMap::new())),
            event_manager,
        })
    }

    pub async fn load_profile(&self, profile_id: &str) -> Result<()> {
        let tasks = self.storage.load(profile_id)?;
        let mut cache = self.cache.write().await;

        let profile_tasks: HashMap<String, Task> = tasks
            .into_iter()
            .map(|task| (task.id.clone(), task))
            .collect();

        cache.insert(profile_id.to_string(), profile_tasks);

        Ok(())
    }

    pub async fn create(&self, profile_id: &str, task: Task) -> Result<Task> {
        // Validate task
        task.validate()
            .map_err(|e| TaskManagerError::Invalid(e.to_string()))?;

        // Get all tasks for profile (loads from storage if not cached)
        let tasks = self.get_all(profile_id).await?;
        let mut task_list: Vec<Task> = tasks.values().cloned().collect();
        task_list.push(task.clone());

        // Save to storage
        self.storage.save(profile_id, &task_list)?;

        // Add to cache
        {
            let mut cache = self.cache.write().await;
            if let Some(profile_tasks) = cache.get_mut(profile_id) {
                profile_tasks.insert(task.id.clone(), task.clone());
            }
        }

        // Emit task created event
        let event = TaskEvent::created(profile_id.to_string(), task.clone());
        self.event_manager.emit_task(event);

        Ok(task)
    }

    pub async fn get(&self, profile_id: &str, task_id: &str) -> Result<Task> {
        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(profile_tasks) = cache.get(profile_id)
                && let Some(task) = profile_tasks.get(task_id)
            {
                return Ok(task.clone());
            }
        }

        // Load from storage
        self.load_profile(profile_id).await?;

        // Try cache again
        {
            let cache = self.cache.read().await;
            if let Some(profile_tasks) = cache.get(profile_id)
                && let Some(task) = profile_tasks.get(task_id)
            {
                return Ok(task.clone());
            }
        }

        Err(TaskManagerError::NotFound(task_id.to_string()))
    }

    pub async fn get_all(&self, profile_id: &str) -> Result<HashMap<String, Task>> {
        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(profile_tasks) = cache.get(profile_id) {
                return Ok(profile_tasks.clone());
            }
        }

        // Load from storage
        self.load_profile(profile_id).await?;

        // Get from cache
        let cache = self.cache.read().await;
        Ok(cache.get(profile_id).cloned().unwrap_or_default())
    }

    pub async fn list(&self, profile_id: &str) -> Result<Vec<Task>> {
        let tasks = self.get_all(profile_id).await?;
        Ok(tasks.values().cloned().collect())
    }

    pub async fn update(&self, profile_id: &str, mut task: Task) -> Result<Task> {
        // Validate task
        task.validate()
            .map_err(|e| TaskManagerError::Invalid(e.to_string()))?;

        // Update timestamp
        task.touch();

        // Get all tasks
        let mut tasks = self.get_all(profile_id).await?;

        // Check if exists
        if !tasks.contains_key(&task.id) {
            return Err(TaskManagerError::NotFound(task.id.clone()));
        }

        // Update in list
        tasks.insert(task.id.clone(), task.clone());
        let task_list: Vec<Task> = tasks.values().cloned().collect();

        // Save to storage
        self.storage.save(profile_id, &task_list)?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            if let Some(profile_tasks) = cache.get_mut(profile_id) {
                profile_tasks.insert(task.id.clone(), task.clone());
            }
        }

        // Emit task updated event
        let event = TaskEvent::updated(profile_id.to_string(), task.clone());
        self.event_manager.emit_task(event);

        Ok(task)
    }

    pub async fn delete(&self, profile_id: &str, task_id: &str) -> Result<()> {
        // Get all tasks
        let mut tasks = self.get_all(profile_id).await?;

        // Check if exists
        if !tasks.contains_key(task_id) {
            return Err(TaskManagerError::NotFound(task_id.to_string()));
        }

        // Remove from list
        tasks.remove(task_id);
        let task_list: Vec<Task> = tasks.values().cloned().collect();

        // Save to storage
        self.storage.save(profile_id, &task_list)?;

        // Remove from cache
        {
            let mut cache = self.cache.write().await;
            if let Some(profile_tasks) = cache.get_mut(profile_id) {
                profile_tasks.remove(task_id);
            }
        }

        // Emit task deleted event
        let event = TaskEvent::deleted(profile_id.to_string(), task_id.to_string());
        self.event_manager.emit_task(event);

        Ok(())
    }

    pub async fn search(&self, profile_id: &str, query: &str) -> Result<Vec<Task>> {
        let tasks = self.list(profile_id).await?;
        let query_lower = query.to_lowercase();

        Ok(tasks
            .into_iter()
            .filter(|task| {
                task.title.to_lowercase().contains(&query_lower)
                    || task
                        .description
                        .as_ref()
                        .is_some_and(|d| d.to_lowercase().contains(&query_lower))
                    || task
                        .tags
                        .iter()
                        .any(|t| t.to_lowercase().contains(&query_lower))
            })
            .collect())
    }
}

impl Default for TaskManager {
    fn default() -> Self {
        Self::new(Arc::new(crate::event_manager::EventManager::new()))
            .expect("Failed to create TaskManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_manager::EventManager;
    use mootimer_core::models::Task;
    use std::sync::Arc;

    const TEST_PROFILE: &str = "test_profile";

    fn create_manager() -> TaskManager {
        let event_manager = Arc::new(EventManager::new());
        TaskManager::new(event_manager).unwrap()
    }

    #[tokio::test]
    async fn test_create_task() {
        let manager = create_manager();
        let task = Task::new("Test Task".to_string()).unwrap();
        let created_task = manager.create(TEST_PROFILE, task).await.unwrap();

        assert_eq!(created_task.title, "Test Task");
    }

    #[tokio::test]
    async fn test_get_task() {
        let manager = create_manager();
        let task = Task::new("Test Get".to_string()).unwrap();
        let created_task = manager.create(TEST_PROFILE, task).await.unwrap();

        let retrieved = manager.get(TEST_PROFILE, &created_task.id).await.unwrap();
        assert_eq!(retrieved.title, "Test Get");
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let manager = create_manager();
        let task1 = Task::new("Task 1".to_string()).unwrap();
        let task2 = Task::new("Task 2".to_string()).unwrap();
        manager.create(TEST_PROFILE, task1).await.unwrap();
        manager.create(TEST_PROFILE, task2).await.unwrap();

        let tasks = manager.list(TEST_PROFILE).await.unwrap();
        assert!(tasks.len() >= 2);
    }

    #[tokio::test]
    async fn test_update_task() {
        let manager = create_manager();
        let mut task = Task::new("Old Title".to_string()).unwrap();
        task = manager.create(TEST_PROFILE, task).await.unwrap();

        task.title = "New Title".to_string();
        let updated = manager.update(TEST_PROFILE, task).await.unwrap();

        assert_eq!(updated.title, "New Title");
    }

    #[tokio::test]
    async fn test_delete_task() {
        let manager = create_manager();
        let task = Task::new("Delete Me".to_string()).unwrap();
        let created_task = manager.create(TEST_PROFILE, task).await.unwrap();

        manager
            .delete(TEST_PROFILE, &created_task.id)
            .await
            .unwrap();

        let result = manager.get(TEST_PROFILE, &created_task.id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_tasks() {
        let manager = create_manager();
        let task1 = Task::new("My First Task".to_string()).unwrap();
        let task2 = Task::new("Another Task".to_string()).unwrap();
        manager.create(TEST_PROFILE, task1).await.unwrap();
        manager.create(TEST_PROFILE, task2).await.unwrap();

        let results = manager.search(TEST_PROFILE, "First").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "My First Task");
    }
}
