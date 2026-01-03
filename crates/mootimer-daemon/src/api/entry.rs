//! Entry API methods

use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{Value, json};
use std::sync::Arc;

use super::{ApiError, Result};
use crate::entry::{EntryFilter, EntryManager};
use crate::profile::ProfileManager;

#[derive(Debug, Deserialize)]
struct ListEntriesParams {
    profile_id: String,
}

#[derive(Debug, Deserialize)]
struct FilterEntriesParams {
    profile_id: String,
    start_date: Option<DateTime<Utc>>,
    end_date: Option<DateTime<Utc>>,
    task_id: Option<String>,
    tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct StatsParams {
    profile_id: String,
}

#[derive(Debug, Deserialize)]
struct DeleteEntryParams {
    profile_id: String,
    entry_id: String,
}

#[derive(Debug, Deserialize)]
struct UpdateEntryParams {
    profile_id: String,
    entry: mootimer_core::models::Entry,
}

/// Delete an entry
pub async fn delete(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: DeleteEntryParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .delete(&params.profile_id, &params.entry_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({ "status": "deleted", "id": params.entry_id }))
}

/// Update an entry
pub async fn update(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: UpdateEntryParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .update(&params.profile_id, params.entry)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({ "status": "updated" }))
}

/// List all entries for a profile
pub async fn list(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: ListEntriesParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let entries = manager
        .get_all(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&entries)?)
}

/// Filter entries
pub async fn filter(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: FilterEntriesParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let filter = EntryFilter {
        start_date: params.start_date,
        end_date: params.end_date,
        task_id: params.task_id,
        tags: params.tags,
    };

    let entries = manager
        .filter(&params.profile_id, filter)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&entries)?)
}

/// Get today's entries
pub async fn get_today(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: StatsParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let entries = manager
        .get_today(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&entries)?)
}

/// Get this week's entries
pub async fn get_week(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: StatsParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let entries = manager
        .get_week(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&entries)?)
}

/// Get this month's entries
pub async fn get_month(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: StatsParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let entries = manager
        .get_month(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&entries)?)
}

/// Get today's statistics
pub async fn stats_today(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: StatsParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let stats = manager
        .get_today_stats(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({
        "total_entries": stats.total_entries,
        "total_duration_seconds": stats.total_duration_seconds,
        "total_duration_hours": stats.total_duration_hours,
        "pomodoro_count": stats.pomodoro_count,
        "manual_count": stats.manual_count,
        "avg_duration_seconds": stats.avg_duration_seconds,
    }))
}

/// Get this week's statistics
pub async fn stats_week(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: StatsParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let stats = manager
        .get_week_stats(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({
        "total_entries": stats.total_entries,
        "total_duration_seconds": stats.total_duration_seconds,
        "total_duration_hours": stats.total_duration_hours,
        "pomodoro_count": stats.pomodoro_count,
        "manual_count": stats.manual_count,
        "avg_duration_seconds": stats.avg_duration_seconds,
    }))
}

/// Get this month's statistics
pub async fn stats_month(manager: &Arc<EntryManager>, params: Option<Value>) -> Result<Value> {
    let params: StatsParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let stats = manager
        .get_month_stats(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({
        "total_entries": stats.total_entries,
        "total_duration_seconds": stats.total_duration_seconds,
        "total_duration_hours": stats.total_duration_hours,
        "pomodoro_count": stats.pomodoro_count,
        "manual_count": stats.manual_count,
        "avg_duration_seconds": stats.avg_duration_seconds,
    }))
}

/// Get today's entries across all profiles
pub async fn get_today_all_profiles(
    entry_manager: &Arc<EntryManager>,
    profile_manager: &Arc<ProfileManager>,
    _params: Option<Value>,
) -> Result<Value> {
    let profiles = profile_manager
        .list()
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    let mut all_entries = Vec::new();

    for profile in profiles {
        let entries = entry_manager
            .get_today(&profile.id)
            .await
            .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

        // Add profile_id to each entry
        for entry in entries {
            let mut entry_value = serde_json::to_value(&entry)?;
            if let Some(obj) = entry_value.as_object_mut() {
                obj.insert("profile_id".to_string(), json!(profile.id));
            }
            all_entries.push(entry_value);
        }
    }

    Ok(json!(all_entries))
}

/// Get this week's entries across all profiles
pub async fn get_week_all_profiles(
    entry_manager: &Arc<EntryManager>,
    profile_manager: &Arc<ProfileManager>,
    _params: Option<Value>,
) -> Result<Value> {
    let profiles = profile_manager
        .list()
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    let mut all_entries = Vec::new();

    for profile in profiles {
        let entries = entry_manager
            .get_week(&profile.id)
            .await
            .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

        // Add profile_id to each entry
        for entry in entries {
            let mut entry_value = serde_json::to_value(&entry)?;
            if let Some(obj) = entry_value.as_object_mut() {
                obj.insert("profile_id".to_string(), json!(profile.id));
            }
            all_entries.push(entry_value);
        }
    }

    Ok(json!(all_entries))
}

/// Get this month's entries across all profiles
pub async fn get_month_all_profiles(
    entry_manager: &Arc<EntryManager>,
    profile_manager: &Arc<ProfileManager>,
    _params: Option<Value>,
) -> Result<Value> {
    let profiles = profile_manager
        .list()
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    let mut all_entries = Vec::new();

    for profile in profiles {
        let entries = entry_manager
            .get_month(&profile.id)
            .await
            .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

        // Add profile_id to each entry
        for entry in entries {
            let mut entry_value = serde_json::to_value(&entry)?;
            if let Some(obj) = entry_value.as_object_mut() {
                obj.insert("profile_id".to_string(), json!(profile.id));
            }
            all_entries.push(entry_value);
        }
    }

    Ok(json!(all_entries))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_manager::EventManager;
    use std::sync::Arc;

    const TEST_PROFILE: &str = "test_entry_api";

    #[tokio::test]
    async fn test_list_entries() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(EntryManager::new(event_manager).unwrap());

        let params = json!({
            "profile_id": TEST_PROFILE
        });

        let result = list(&manager, Some(params)).await.unwrap();
        assert!(result.is_array());
    }

    #[tokio::test]
    async fn test_stats_today() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(EntryManager::new(event_manager).unwrap());

        let params = json!({
            "profile_id": TEST_PROFILE
        });

        let result = stats_today(&manager, Some(params)).await.unwrap();
        assert!(result.get("total_entries").is_some());
        assert!(result.get("total_duration_hours").is_some());
    }
}
