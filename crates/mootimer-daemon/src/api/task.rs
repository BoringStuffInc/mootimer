//! Task API methods

use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use super::{ApiError, Result};
use crate::task::TaskManager;
use mootimer_core::models::Task;

#[derive(Debug, Deserialize)]
struct CreateTaskParams {
    profile_id: String,
    title: String,
    description: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct TaskIdParams {
    profile_id: String,
    task_id: String,
}

#[derive(Debug, Deserialize)]
struct ListTasksParams {
    profile_id: String,
}

#[derive(Debug, Deserialize)]
struct UpdateTaskParams {
    profile_id: String,
    task: Task,
}

#[derive(Debug, Deserialize)]
struct SearchTasksParams {
    profile_id: String,
    query: String,
}

/// Create a new task
pub async fn create(manager: &Arc<TaskManager>, params: Option<Value>) -> Result<Value> {
    let params: CreateTaskParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let mut task = Task::new(params.title).map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    if let Some(desc) = params.description {
        task.update_description(Some(desc));
    }

    if let Some(tags) = params.tags {
        for tag in tags {
            task.add_tag(tag);
        }
    }

    let created = manager
        .create(&params.profile_id, task)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&created)?)
}

/// Get a task
pub async fn get(manager: &Arc<TaskManager>, params: Option<Value>) -> Result<Value> {
    let params: TaskIdParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let task = manager
        .get(&params.profile_id, &params.task_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&task)?)
}

/// List all tasks for a profile
pub async fn list(manager: &Arc<TaskManager>, params: Option<Value>) -> Result<Value> {
    let params: ListTasksParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let tasks = manager
        .list(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&tasks)?)
}

/// Update a task
pub async fn update(manager: &Arc<TaskManager>, params: Option<Value>) -> Result<Value> {
    let params: UpdateTaskParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let updated = manager
        .update(&params.profile_id, params.task)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&updated)?)
}

/// Delete a task
pub async fn delete(manager: &Arc<TaskManager>, params: Option<Value>) -> Result<Value> {
    let params: TaskIdParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .delete(&params.profile_id, &params.task_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({
        "status": "deleted"
    }))
}

/// Search tasks
pub async fn search(manager: &Arc<TaskManager>, params: Option<Value>) -> Result<Value> {
    let params: SearchTasksParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let tasks = manager
        .search(&params.profile_id, &params.query)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&tasks)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_PROFILE: &str = "test_task_api";

    #[tokio::test]
    async fn test_create_task() {
        let manager = Arc::new(TaskManager::new().unwrap());

        let params = json!({
            "profile_id": TEST_PROFILE,
            "title": "Test Task"
        });

        let result = create(&manager, Some(params)).await.unwrap();
        assert!(result.get("id").is_some());
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let manager = Arc::new(TaskManager::new().unwrap());

        let params = json!({
            "profile_id": TEST_PROFILE
        });

        let result = list(&manager, Some(params)).await.unwrap();
        assert!(result.is_array());
    }
}
