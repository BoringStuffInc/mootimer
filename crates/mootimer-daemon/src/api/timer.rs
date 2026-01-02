//! Timer API methods

use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use super::{ApiError, Result};
use crate::config::ConfigManager;
use crate::entry::EntryManager;
use crate::sync::SyncManager;
use crate::timer::TimerManager;
use mootimer_core::models::PomodoroConfig;

#[derive(Debug, Deserialize)]
struct StartManualParams {
    profile_id: String,
    task_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct StartPomodoroParams {
    profile_id: String,
    task_id: Option<String>,
    #[serde(default)]
    config: Option<PomodoroConfig>,
}

#[derive(Debug, Deserialize)]
struct StartCountdownParams {
    profile_id: String,
    task_id: Option<String>,
    duration_minutes: u64,
}

#[derive(Debug, Deserialize)]
struct ProfileParams {
    profile_id: String,
}

/// Start a manual timer
pub async fn start_manual(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: StartManualParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let timer_id = manager
        .start_manual(params.profile_id, params.task_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "timer_id": timer_id,
        "status": "started"
    }))
}

/// Start a pomodoro timer
pub async fn start_pomodoro(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: StartPomodoroParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let config = params.config.unwrap_or_default();

    let timer_id = manager
        .start_pomodoro(params.profile_id, params.task_id, config)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "timer_id": timer_id,
        "status": "started"
    }))
}

/// Start a countdown timer
pub async fn start_countdown(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: StartCountdownParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let timer_id = manager
        .start_countdown(params.profile_id, params.task_id, params.duration_minutes)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "timer_id": timer_id,
        "status": "started",
        "duration_minutes": params.duration_minutes
    }))
}

/// Pause a timer
pub async fn pause(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: ProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .pause(&params.profile_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "status": "paused"
    }))
}

/// Resume a timer
pub async fn resume(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: ProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .resume(&params.profile_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "status": "resumed"
    }))
}

/// Stop a timer
pub async fn stop(
    timer_manager: &Arc<TimerManager>,
    entry_manager: &Arc<EntryManager>,
    sync_manager: &Arc<SyncManager>,
    config_manager: &Arc<ConfigManager>,
    params: Option<Value>,
) -> Result<Value> {
    let params: ProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let entry = timer_manager
        .stop(&params.profile_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    // Save the entry to storage
    entry_manager
        .add(&params.profile_id, entry.clone())
        .await
        .map_err(|e| ApiError::Timer(format!("Failed to save entry: {}", e)))?;

    // Auto-commit if configured
    let config = config_manager.get().await;
    if config.sync.auto_commit {
        // Initialize git repo if not already done
        if !sync_manager.is_initialized().await {
            let _ = sync_manager.init_repo().await;
        }

        // Commit with a descriptive message
        let task_info = entry
            .task_id
            .as_ref()
            .map(|id| format!("task {}", id))
            .unwrap_or_else(|| "no task".to_string());
        let duration_mins = entry.duration_seconds / 60;
        let commit_msg = format!(
            "Add entry: {} - {}m ({})",
            task_info,
            duration_mins,
            chrono::Local::now().format("%Y-%m-%d %H:%M")
        );

        if let Err(e) = sync_manager.auto_commit(&commit_msg).await {
            tracing::warn!("Failed to auto-commit: {}", e);
        }

        // Auto-sync if configured
        if config.sync.auto_push && config.sync.remote_url.is_some() {
            if let Err(e) = sync_manager.sync(&config.sync).await {
                tracing::warn!("Failed to auto-sync: {}", e);
            }
        }
    }

    Ok(serde_json::to_value(&entry)?)
}

/// Cancel a timer
pub async fn cancel(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: ProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .cancel(&params.profile_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "status": "cancelled"
    }))
}

/// Get timer status for a profile
pub async fn get(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: ProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    tracing::debug!("timer.get called for profile {}", params.profile_id);
    let timer = manager
        .get_timer(&params.profile_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;
    tracing::debug!("timer.get returning for profile {}", params.profile_id);

    Ok(serde_json::to_value(&timer)?)
}

/// List all active timers
pub async fn list(manager: &Arc<TimerManager>, _params: Option<Value>) -> Result<Value> {
    let timers = manager.get_all_timers().await;
    Ok(serde_json::to_value(&timers)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_manual() {
        let manager = Arc::new(TimerManager::new());

        let params = json!({
            "profile_id": "test",
            "task_id": "task1"
        });

        let result = start_manual(&manager, Some(params)).await.unwrap();
        assert!(result.get("timer_id").is_some());
        assert_eq!(result.get("status").unwrap(), "started");
    }

    #[tokio::test]
    async fn test_start_pomodoro() {
        let manager = Arc::new(TimerManager::new());

        let params = json!({
            "profile_id": "test"
        });

        let result = start_pomodoro(&manager, Some(params)).await.unwrap();
        assert!(result.get("timer_id").is_some());
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let manager = Arc::new(TimerManager::new());

        // Start a timer first
        let start_params = json!({"profile_id": "test"});
        start_manual(&manager, Some(start_params)).await.unwrap();

        // Pause
        let pause_params = json!({"profile_id": "test"});
        let result = pause(&manager, Some(pause_params)).await.unwrap();
        assert_eq!(result.get("status").unwrap(), "paused");

        // Resume
        let resume_params = json!({"profile_id": "test"});
        let result = resume(&manager, Some(resume_params)).await.unwrap();
        assert_eq!(result.get("status").unwrap(), "resumed");
    }

    #[tokio::test]
    async fn test_list_timers() {
        let manager = Arc::new(TimerManager::new());

        // Start two timers
        let params1 = json!({"profile_id": "test1"});
        let params2 = json!({"profile_id": "test2"});

        start_manual(&manager, Some(params1)).await.unwrap();
        start_manual(&manager, Some(params2)).await.unwrap();

        let result = list(&manager, None).await.unwrap();
        let timers = result.as_object().unwrap();
        assert_eq!(timers.len(), 2);
    }
}
