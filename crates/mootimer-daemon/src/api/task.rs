use serde::Deserialize;
use serde_json::{Value, json};
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
    use crate::event_manager::EventManager;
    use std::sync::Arc;

    const TEST_PROFILE: &str = "test_task_api";

    #[tokio::test]
    async fn test_create_and_get_task() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(TaskManager::new(event_manager).unwrap());

        let params = json!({
            "profile_id": TEST_PROFILE,
            "title": "Test API Task"
        });

        let result = create(&manager, Some(params)).await.unwrap();
        let task_id = result.get("id").unwrap().as_str().unwrap().to_string();

        let get_params = json!({
            "profile_id": TEST_PROFILE,
            "task_id": task_id
        });

        let get_result = get(&manager, Some(get_params)).await.unwrap();
        assert_eq!(
            get_result.get("title").unwrap().as_str().unwrap(),
            "Test API Task"
        );
    }

    #[tokio::test]
    async fn test_list_tasks() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(TaskManager::new(event_manager).unwrap());

        let params = json!({ "profile_id": TEST_PROFILE });
        let result = list(&manager, Some(params)).await.unwrap();
        assert!(result.is_array());
    }
}
