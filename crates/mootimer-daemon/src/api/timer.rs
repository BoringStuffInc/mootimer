use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use super::{ApiError, Result};
use crate::config::ConfigManager;
use crate::entry::EntryManager;
use crate::sync::SyncManager;
use crate::timer::TimerManager;

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
    config: Option<Value>,
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

#[derive(Debug, Deserialize)]
struct TimerParams {
    timer_id: String,
}

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

pub async fn start_pomodoro(
    manager: &Arc<TimerManager>,
    config_manager: &Arc<ConfigManager>,
    params: Option<Value>,
) -> Result<Value> {
    let params: StartPomodoroParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let global_config = config_manager.get().await;
    let mut config = global_config.pomodoro.clone();

    if let Some(overrides) = params.config
        && let Some(obj) = overrides.as_object()
    {
        if let Some(v) = obj.get("work_duration").and_then(|v| v.as_u64()) {
            config.work_duration = v;
        }
        if let Some(v) = obj.get("short_break").and_then(|v| v.as_u64()) {
            config.short_break = v;
        }
        if let Some(v) = obj.get("long_break").and_then(|v| v.as_u64()) {
            config.long_break = v;
        }
        if let Some(v) = obj
            .get("sessions_until_long_break")
            .and_then(|v| v.as_u64())
        {
            config.sessions_until_long_break = v as u32;
        }
    }

    let timer_id = manager
        .start_pomodoro(params.profile_id, params.task_id, config)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "timer_id": timer_id,
        "status": "started"
    }))
}

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

pub async fn pause(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: TimerParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .pause(&params.timer_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "status": "paused"
    }))
}

pub async fn resume(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: TimerParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .resume(&params.timer_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "status": "resumed"
    }))
}

pub async fn stop(
    timer_manager: &Arc<TimerManager>,
    entry_manager: &Arc<EntryManager>,
    sync_manager: &Arc<SyncManager>,
    config_manager: &Arc<ConfigManager>,
    params: Option<Value>,
) -> Result<Value> {
    let params: TimerParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let (profile_id, entry) = timer_manager
        .stop(&params.timer_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    entry_manager
        .add(&profile_id, entry.clone())
        .await
        .map_err(|e| ApiError::Timer(format!("Failed to save entry: {}", e)))?;

    let config = config_manager.get().await;
    if config.sync.auto_commit {
        if !sync_manager.is_initialized().await {
            let _ = sync_manager.init_repo().await;
        }

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

        if config.sync.auto_push
            && config.sync.remote_url.is_some()
            && let Err(e) = sync_manager.sync(&config.sync).await
        {
            tracing::warn!("Failed to auto-sync: {}", e);
        }
    }

    Ok(serde_json::to_value(&entry)?)
}

pub async fn cancel(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: TimerParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .cancel(&params.timer_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;

    Ok(json!({
        "status": "cancelled"
    }))
}

pub async fn get(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: TimerParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    tracing::debug!("timer.get called for timer {}", params.timer_id);
    let timer = manager
        .get_timer(&params.timer_id)
        .await
        .map_err(|e| ApiError::Timer(e.to_string()))?;
    tracing::debug!("timer.get returning for timer {}", params.timer_id);

    Ok(serde_json::to_value(&timer)?)
}

pub async fn get_by_profile(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: ProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    tracing::debug!(
        "timer.get_by_profile called for profile {}",
        params.profile_id
    );
    let timers = manager.get_timers_by_profile(&params.profile_id).await;
    tracing::debug!("timer.get_by_profile returning {} timers", timers.len());

    // Return the first timer if exists, null otherwise (backward compatibility)
    if let Some(timer) = timers.first() {
        Ok(serde_json::to_value(timer)?)
    } else {
        Ok(Value::Null)
    }
}

pub async fn list_by_profile(manager: &Arc<TimerManager>, params: Option<Value>) -> Result<Value> {
    let params: ProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let timers = manager.get_timers_by_profile(&params.profile_id).await;
    Ok(serde_json::to_value(&timers)?)
}

pub async fn list(manager: &Arc<TimerManager>, _params: Option<Value>) -> Result<Value> {
    let timers = manager.get_all_timers().await;
    Ok(serde_json::to_value(&timers)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_manager::EventManager;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_start_manual() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(TimerManager::new(event_manager));

        let params = json!({
            "profile_id": "test",
            "task_id": "task1"
        });

        let result = start_manual(&manager, Some(params)).await.unwrap();
        assert!(result.get("timer_id").is_some());
        assert_eq!(result.get("status").unwrap(), "started");
    }

    #[tokio::test]
    async fn test_start_pomodoro_with_partial_config() {
        let event_manager = Arc::new(EventManager::new());
        let timer_manager = Arc::new(TimerManager::new(event_manager.clone()));
        let config_manager = Arc::new(ConfigManager::new().unwrap());

        let params = json!({
            "profile_id": "test",
            "config": {
                "work_duration": 60
            }
        });

        let result = start_pomodoro(&timer_manager, &config_manager, Some(params)).await;

        assert!(
            result.is_ok(),
            "start_pomodoro failed with partial config: {:?}",
            result.err()
        );
    }

    #[tokio::test]
    async fn test_pause_resume() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(TimerManager::new(event_manager));

        let start_params = json!({"profile_id": "test"});
        let result = start_manual(&manager, Some(start_params)).await.unwrap();
        let timer_id = result.get("timer_id").unwrap().as_str().unwrap();

        let pause_params = json!({"timer_id": timer_id});
        let result = pause(&manager, Some(pause_params)).await.unwrap();
        assert_eq!(result.get("status").unwrap(), "paused");

        let resume_params = json!({"timer_id": timer_id});
        let result = resume(&manager, Some(resume_params)).await.unwrap();
        assert_eq!(result.get("status").unwrap(), "resumed");
    }

    #[tokio::test]
    async fn test_list_timers() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(TimerManager::new(event_manager));

        let params1 = json!({"profile_id": "test1"});
        let params2 = json!({"profile_id": "test2"});

        start_manual(&manager, Some(params1)).await.unwrap();
        start_manual(&manager, Some(params2)).await.unwrap();

        let result = list(&manager, None).await.unwrap();
        let timers = result.as_object().unwrap();
        assert_eq!(timers.len(), 2);
    }

    #[tokio::test]
    async fn test_multiple_timers_same_profile() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(TimerManager::new(event_manager));

        let params1 = json!({"profile_id": "test"});
        let params2 = json!({"profile_id": "test"});

        let result1 = start_manual(&manager, Some(params1)).await.unwrap();
        let result2 = start_manual(&manager, Some(params2)).await.unwrap();

        let timer_id1 = result1.get("timer_id").unwrap().as_str().unwrap();
        let timer_id2 = result2.get("timer_id").unwrap().as_str().unwrap();

        assert_ne!(timer_id1, timer_id2);

        let profile_timers = list_by_profile(&manager, Some(json!({"profile_id": "test"})))
            .await
            .unwrap();
        let timers_arr = profile_timers.as_array().unwrap();
        assert_eq!(timers_arr.len(), 2);
    }
}
