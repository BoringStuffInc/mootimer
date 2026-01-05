use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use super::{ApiError, Result};
use crate::config::ConfigManager;
use crate::sync::SyncManager;

#[derive(Debug, Deserialize)]
struct SetRemoteParams {
    url: String,
}

#[derive(Debug, Deserialize)]
struct CommitParams {
    message: String,
}

pub async fn init(sync_manager: &Arc<SyncManager>, _params: Option<Value>) -> Result<Value> {
    sync_manager
        .init_repo()
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({
        "status": "initialized"
    }))
}

pub async fn status(
    sync_manager: &Arc<SyncManager>,
    config_manager: &Arc<ConfigManager>,
    _params: Option<Value>,
) -> Result<Value> {
    let config = config_manager.get().await;

    let status = sync_manager
        .get_status(&config.sync)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&status)?)
}

pub async fn sync(
    sync_manager: &Arc<SyncManager>,
    config_manager: &Arc<ConfigManager>,
    _params: Option<Value>,
) -> Result<Value> {
    let config = config_manager.get().await;

    let result = sync_manager
        .sync(&config.sync)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&result)?)
}

pub async fn commit(sync_manager: &Arc<SyncManager>, params: Option<Value>) -> Result<Value> {
    let params: CommitParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let commit_id = sync_manager
        .auto_commit(&params.message)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({
        "commit_id": commit_id,
        "status": if commit_id.is_some() { "committed" } else { "no_changes" }
    }))
}

pub async fn set_remote(sync_manager: &Arc<SyncManager>, params: Option<Value>) -> Result<Value> {
    let params: SetRemoteParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    sync_manager
        .set_remote(&params.url)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({
        "status": "remote_set",
        "url": params.url
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[tokio::test]
    #[serial]
    async fn test_init() {
        use tempfile::TempDir;
        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
            std::env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));
        }

        let manager = Arc::new(SyncManager::new().unwrap());
        let result = init(&manager, None).await.unwrap();

        assert_eq!(result.get("status").unwrap(), "initialized");
    }

    #[tokio::test]
    #[serial]
    async fn test_commit() {
        use mootimer_core::storage::init_data_dir;
        use tempfile::TempDir;

        let temp_dir = TempDir::new().unwrap();
        unsafe {
            std::env::set_var("HOME", temp_dir.path());
            std::env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));
            std::env::set_var("XDG_CONFIG_HOME", temp_dir.path().join("config"));
        }

        let manager = Arc::new(SyncManager::new().unwrap());
        manager.init_repo().await.unwrap();

        let data_dir = init_data_dir().unwrap();
        std::fs::write(data_dir.join("test.txt"), "Hello").unwrap();

        let params = json!({ "message": "Test commit" });
        let result = commit(&manager, Some(params)).await.unwrap();

        let status = result.get("status").unwrap().as_str().unwrap();
        assert!(status == "committed" || status == "no_changes");
        assert!(result.get("commit_id").is_some());
    }
}
