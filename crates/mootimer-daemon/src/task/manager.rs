//! Task manager for CRUD operations

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use mootimer_core::{
    models::Task,
    storage::{init_data_dir, TaskStorage},
    Result as CoreResult,
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
}

impl TaskManager {
    /// Create a new task manager
    pub fn new() -> CoreResult<Self> {
        let data_dir = init_data_dir()?;
        let storage = TaskStorage::new(data_dir);

        Ok(Self {
            storage,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Load all tasks for a profile
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

    /// Create a new task for a profile
    pub async fn create(&self, profile_id: &str, task: Task) -> Result<Task> {
        // Validate task
        task.validate()
            .map_err(|e| TaskManagerError::Invalid(e.to_string()))?;

        // Ensure profile cache exists
        {
            let mut cache = self.cache.write().await;
            cache
                .entry(profile_id.to_string())
                .or_insert_with(HashMap::new);
        }

        // Get all tasks for profile
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

        Ok(task)
    }

    /// Get a task by ID
    pub async fn get(&self, profile_id: &str, task_id: &str) -> Result<Task> {
        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(profile_tasks) = cache.get(profile_id) {
                if let Some(task) = profile_tasks.get(task_id) {
                    return Ok(task.clone());
                }
            }
        }

        // Load from storage
        self.load_profile(profile_id).await?;

        // Try cache again
        {
            let cache = self.cache.read().await;
            if let Some(profile_tasks) = cache.get(profile_id) {
                if let Some(task) = profile_tasks.get(task_id) {
                    return Ok(task.clone());
                }
            }
        }

        Err(TaskManagerError::NotFound(task_id.to_string()))
    }

    /// Get all tasks for a profile
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

    /// List all tasks for a profile
    pub async fn list(&self, profile_id: &str) -> Result<Vec<Task>> {
        let tasks = self.get_all(profile_id).await?;
        Ok(tasks.values().cloned().collect())
    }

    /// Update a task
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

        Ok(task)
    }

    /// Delete a task
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

        Ok(())
    }

    /// Search tasks by filter
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
        Self::new().expect("Failed to create TaskManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mootimer_core::models::Task;

    const TEST_PROFILE: &str = "test_task_profile";

    #[tokio::test]
    async fn test_create_task() {
        let manager = TaskManager::new().unwrap();

        let task = Task::new("Test Task".to_string()).unwrap();
        let created = manager.create(TEST_PROFILE, task).await.unwrap();

        assert_eq!(created.title, "Test Task");
    }

    #[tokio::test]
    async fn test_get_task() {
        let manager = TaskManager::new().unwrap();

        let task = Task::new("Get Test".to_string()).unwrap();
        let task_id = task.id.clone();
        manager.create(TEST_PROFILE, task).await.unwrap();

        let retrieved = manager.get(TEST_PROFILE, &task_id).await.unwrap();
        assert_eq!(retrieved.title, "Get Test");
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let manager = TaskManager::new().unwrap();

        let task1 = Task::new("Task 1".to_string()).unwrap();
        let task2 = Task::new("Task 2".to_string()).unwrap();

        manager.create(TEST_PROFILE, task1).await.unwrap();
        manager.create(TEST_PROFILE, task2).await.unwrap();

        let tasks = manager.list(TEST_PROFILE).await.unwrap();
        assert!(tasks.len() >= 2);
    }

    #[tokio::test]
    async fn test_update_task() {
        let manager = TaskManager::new().unwrap();

        let mut task = Task::new("Old Title".to_string()).unwrap();
        manager.create(TEST_PROFILE, task.clone()).await.unwrap();

        task.update_title("New Title".to_string()).unwrap();
        let updated = manager.update(TEST_PROFILE, task).await.unwrap();

        assert_eq!(updated.title, "New Title");
    }

    #[tokio::test]
    async fn test_delete_task() {
        let manager = TaskManager::new().unwrap();

        let task = Task::new("Delete Me".to_string()).unwrap();
        let task_id = task.id.clone();
        manager.create(TEST_PROFILE, task).await.unwrap();

        manager.delete(TEST_PROFILE, &task_id).await.unwrap();

        let result = manager.get(TEST_PROFILE, &task_id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_search_tasks() {
        let manager = TaskManager::new().unwrap();

        let mut task1 = Task::new("Backend Work".to_string()).unwrap();
        task1.add_tag("backend".to_string());

        let task2 = Task::new("Frontend Work".to_string()).unwrap();

        manager.create(TEST_PROFILE, task1).await.unwrap();
        manager.create(TEST_PROFILE, task2).await.unwrap();

        let results = manager.search(TEST_PROFILE, "backend").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Backend Work");
    }
}
