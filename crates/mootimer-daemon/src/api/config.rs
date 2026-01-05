
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use super::{ApiError, Result};
use crate::config::ConfigManager;

#[derive(Debug, Deserialize)]
struct SetDefaultProfileParams {
    profile_id: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UpdatePomodoroParams {
    work_duration: Option<u64>,
    short_break: Option<u64>,
    long_break: Option<u64>,
    sessions_until_long_break: Option<u32>,
    countdown_default: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct UpdateSyncParams {
    auto_commit: Option<bool>,
    auto_push: Option<bool>,
    remote_url: Option<String>,
}

pub async fn get(manager: &Arc<ConfigManager>, _params: Option<Value>) -> Result<Value> {
    let config = manager.get().await;
    Ok(serde_json::to_value(&config)?)
}

pub async fn set_default_profile(
    manager: &Arc<ConfigManager>,
    params: Option<Value>,
) -> Result<Value> {
    let params: SetDefaultProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let config = manager
        .set_default_profile(params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&config)?)
}

pub async fn update_pomodoro(manager: &Arc<ConfigManager>, params: Option<Value>) -> Result<Value> {
    let params: UpdatePomodoroParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let config = manager
        .update_pomodoro_config(
            params.work_duration,
            params.short_break,
            params.long_break,
            params.sessions_until_long_break,
            params.countdown_default,
        )
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&config)?)
}

pub async fn update_sync(manager: &Arc<ConfigManager>, params: Option<Value>) -> Result<Value> {
    let params: UpdateSyncParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let config = manager
        .update_sync_config(params.auto_commit, params.auto_push, params.remote_url)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&config)?)
}

pub async fn reset(manager: &Arc<ConfigManager>, _params: Option<Value>) -> Result<Value> {
    let config = manager
        .reset_to_default()
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&config)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_get_config() {
        let manager = Arc::new(ConfigManager::new().unwrap());
        let result = get(&manager, None).await.unwrap();
        assert!(result.get("version").is_some());
    }

    #[tokio::test]
    async fn test_update_pomodoro() {
        let manager = Arc::new(ConfigManager::new().unwrap());

        let params = serde_json::json!({
            "work_duration": 1800
        });

        let result = update_pomodoro(&manager, Some(params)).await.unwrap();
        assert_eq!(
            result
                .get("pomodoro")
                .unwrap()
                .get("work_duration")
                .unwrap(),
            1800
        );
    }
}
