use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::event_manager::EventManager;
use crate::events::TaskEvent;
use mootimer_core::{
    Result as CoreResult, models::Task, storage::TaskStorage, storage::init_data_dir,
};

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

pub struct TaskManager {
    storage: TaskStorage,
    cache: Arc<RwLock<HashMap<String, HashMap<String, Task>>>>,
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
        task.validate()
            .map_err(|e| TaskManagerError::Invalid(e.to_string()))?;

        let tasks = self.get_all(profile_id).await?;
        let mut task_list: Vec<Task> = tasks.values().cloned().collect();
        task_list.push(task.clone());

        self.storage.save(profile_id, &task_list)?;

        {
            let mut cache = self.cache.write().await;
            if let Some(profile_tasks) = cache.get_mut(profile_id) {
                profile_tasks.insert(task.id.clone(), task.clone());
            }
        }

        let event = TaskEvent::created(profile_id.to_string(), task.clone());
        self.event_manager.emit_task(event);

        Ok(task)
    }

    pub async fn get(&self, profile_id: &str, task_id: &str) -> Result<Task> {
        {
            let cache = self.cache.read().await;
            if let Some(profile_tasks) = cache.get(profile_id)
                && let Some(task) = profile_tasks.get(task_id)
            {
                return Ok(task.clone());
            }
        }

        self.load_profile(profile_id).await?;

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
        {
            let cache = self.cache.read().await;
            if let Some(profile_tasks) = cache.get(profile_id) {
                return Ok(profile_tasks.clone());
            }
        }

        self.load_profile(profile_id).await?;

        let cache = self.cache.read().await;
        Ok(cache.get(profile_id).cloned().unwrap_or_default())
    }

    pub async fn list(&self, profile_id: &str) -> Result<Vec<Task>> {
        let tasks = self.get_all(profile_id).await?;
        Ok(tasks.values().cloned().collect())
    }

    pub async fn update(&self, profile_id: &str, mut task: Task) -> Result<Task> {
        task.validate()
            .map_err(|e| TaskManagerError::Invalid(e.to_string()))?;

        task.touch();

        let mut tasks = self.get_all(profile_id).await?;

        if !tasks.contains_key(&task.id) {
            return Err(TaskManagerError::NotFound(task.id.clone()));
        }

        tasks.insert(task.id.clone(), task.clone());
        let task_list: Vec<Task> = tasks.values().cloned().collect();

        self.storage.save(profile_id, &task_list)?;

        {
            let mut cache = self.cache.write().await;
            if let Some(profile_tasks) = cache.get_mut(profile_id) {
                profile_tasks.insert(task.id.clone(), task.clone());
            }
        }

        let event = TaskEvent::updated(profile_id.to_string(), task.clone());
        self.event_manager.emit_task(event);

        Ok(task)
    }

    pub async fn delete(&self, profile_id: &str, task_id: &str) -> Result<()> {
        let mut tasks = self.get_all(profile_id).await?;

        if !tasks.contains_key(task_id) {
            return Err(TaskManagerError::NotFound(task_id.to_string()));
        }

        tasks.remove(task_id);
        let task_list: Vec<Task> = tasks.values().cloned().collect();

        self.storage.save(profile_id, &task_list)?;

        {
            let mut cache = self.cache.write().await;
            if let Some(profile_tasks) = cache.get_mut(profile_id) {
                profile_tasks.remove(task_id);
            }
        }

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

    pub async fn move_task(
        &self,
        source_profile_id: &str,
        target_profile_id: &str,
        task_id: &str,
        move_entries: bool,
        entry_manager: &crate::entry::EntryManager,
        timer_manager: &crate::timer::TimerManager,
    ) -> Result<(Task, usize)> {
        if source_profile_id == target_profile_id {
            return Err(TaskManagerError::Invalid(
                "Source and target profiles must be different".to_string(),
            ));
        }

        let timers = timer_manager.get_all_timers().await;
        let has_active_timer = timers
            .values()
            .any(|t| t.task_id.as_deref() == Some(task_id));

        if has_active_timer {
            return Err(TaskManagerError::Invalid(
                "Task has an active timer. Stop the timer first.".to_string(),
            ));
        }

        let task = self.get(source_profile_id, task_id).await?;

        {
            let mut source_tasks = self.get_all(source_profile_id).await?;
            source_tasks.remove(task_id);
            let task_list: Vec<Task> = source_tasks.values().cloned().collect();
            self.storage.save(source_profile_id, &task_list)?;

            let mut cache = self.cache.write().await;
            cache.insert(source_profile_id.to_string(), source_tasks);
        }

        {
            let mut target_tasks = self.get_all(target_profile_id).await.unwrap_or_default();
            target_tasks.insert(task.id.clone(), task.clone());
            let task_list: Vec<Task> = target_tasks.values().cloned().collect();
            self.storage.save(target_profile_id, &task_list)?;

            let mut cache = self.cache.write().await;
            cache.insert(target_profile_id.to_string(), target_tasks);
        }

        let entries_moved = if move_entries {
            entry_manager
                .move_entries_for_task(source_profile_id, target_profile_id, task_id)
                .await
                .unwrap_or(0)
        } else {
            0
        };

        let event = TaskEvent::moved(
            source_profile_id.to_string(),
            target_profile_id.to_string(),
            task.clone(),
            entries_moved,
        );
        self.event_manager.emit_task(event);

        Ok((task, entries_moved))
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
    use serial_test::serial;
    use std::sync::Arc;
    use tempfile::TempDir;

    const TEST_PROFILE: &str = "test_profile";

    fn create_manager(_temp_dir: &TempDir) -> TaskManager {
        let event_manager = Arc::new(EventManager::new());
        unsafe {
            std::env::set_var("HOME", _temp_dir.path());
            std::env::set_var("XDG_DATA_HOME", _temp_dir.path().join("data"));
            std::env::set_var("XDG_CONFIG_HOME", _temp_dir.path().join("config"));
        }
        TaskManager::new(event_manager).unwrap()
    }

    #[tokio::test]
    #[serial]
    async fn test_create_task() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let task = Task::new("Test Task".to_string()).unwrap();
        let created_task = manager.create(TEST_PROFILE, task).await.unwrap();

        assert_eq!(created_task.title, "Test Task");
    }

    #[tokio::test]
    #[serial]
    async fn test_get_task() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let task = Task::new("Test Get".to_string()).unwrap();
        let created_task = manager.create(TEST_PROFILE, task).await.unwrap();

        let retrieved = manager.get(TEST_PROFILE, &created_task.id).await.unwrap();
        assert_eq!(retrieved.title, "Test Get");
    }

    #[tokio::test]
    #[serial]
    async fn test_list_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let task1 = Task::new("Task 1".to_string()).unwrap();
        let task2 = Task::new("Task 2".to_string()).unwrap();
        manager.create(TEST_PROFILE, task1).await.unwrap();
        manager.create(TEST_PROFILE, task2).await.unwrap();

        let tasks = manager.list(TEST_PROFILE).await.unwrap();
        assert!(tasks.len() >= 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_task() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let mut task = Task::new("Old Title".to_string()).unwrap();
        task = manager.create(TEST_PROFILE, task).await.unwrap();

        task.title = "New Title".to_string();
        let updated = manager.update(TEST_PROFILE, task).await.unwrap();

        assert_eq!(updated.title, "New Title");
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_task() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
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
    #[serial]
    async fn test_search_tasks() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let task1 = Task::new("My First Task".to_string()).unwrap();
        let task2 = Task::new("Another Task".to_string()).unwrap();
        manager.create(TEST_PROFILE, task1).await.unwrap();
        manager.create(TEST_PROFILE, task2).await.unwrap();

        let results = manager.search(TEST_PROFILE, "First").await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "My First Task");
    }

    #[tokio::test]
    #[serial]
    async fn test_move_task() {
        let temp_dir = TempDir::new().unwrap();
        let event_manager = Arc::new(EventManager::new());
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
            std::env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));
            std::env::set_var("XDG_CONFIG_HOME", temp_dir.path().join("config"));
        }
        let task_manager = TaskManager::new(Arc::clone(&event_manager)).unwrap();
        let entry_manager = crate::entry::EntryManager::new(Arc::clone(&event_manager)).unwrap();
        let timer_manager = crate::timer::TimerManager::new(Arc::clone(&event_manager));

        let source = "source_profile";
        let target = "target_profile";

        let task = Task::new("Move Me".to_string()).unwrap();
        let created = task_manager.create(source, task).await.unwrap();

        let (moved_task, entries_moved) = task_manager
            .move_task(
                source,
                target,
                &created.id,
                false,
                &entry_manager,
                &timer_manager,
            )
            .await
            .unwrap();

        assert_eq!(moved_task.id, created.id);
        assert_eq!(moved_task.title, "Move Me");
        assert_eq!(entries_moved, 0);

        let source_result = task_manager.get(source, &created.id).await;
        assert!(source_result.is_err());

        let target_task = task_manager.get(target, &created.id).await.unwrap();
        assert_eq!(target_task.title, "Move Me");
    }

    #[tokio::test]
    #[serial]
    async fn test_move_task_same_profile_error() {
        let temp_dir = TempDir::new().unwrap();
        let event_manager = Arc::new(EventManager::new());
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
            std::env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));
            std::env::set_var("XDG_CONFIG_HOME", temp_dir.path().join("config"));
        }
        let task_manager = TaskManager::new(Arc::clone(&event_manager)).unwrap();
        let entry_manager = crate::entry::EntryManager::new(Arc::clone(&event_manager)).unwrap();
        let timer_manager = crate::timer::TimerManager::new(Arc::clone(&event_manager));

        let profile = "same_profile";
        let task = Task::new("Stuck Task".to_string()).unwrap();
        let created = task_manager.create(profile, task).await.unwrap();

        let result = task_manager
            .move_task(
                profile,
                profile,
                &created.id,
                false,
                &entry_manager,
                &timer_manager,
            )
            .await;

        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string()
                .contains("Source and target profiles must be different")
        );
    }
}
