//! Profile API methods

use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use super::{ApiError, Result};
use crate::profile::ProfileManager;
use mootimer_core::models::Profile;

#[derive(Debug, Deserialize)]
struct CreateProfileParams {
    id: String,
    name: String,
    description: Option<String>,
    color: Option<String>,
}

#[derive(Debug, Deserialize)]
struct ProfileIdParams {
    profile_id: String,
}

#[derive(Debug, Deserialize)]
struct UpdateProfileParams {
    profile: Profile,
}

/// Create a new profile
pub async fn create(manager: &Arc<ProfileManager>, params: Option<Value>) -> Result<Value> {
    let params: CreateProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let mut profile =
        Profile::new(params.id, params.name).map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    if let Some(desc) = params.description {
        profile.update_description(Some(desc));
    }

    if let Some(color) = params.color {
        profile
            .update_color(Some(color))
            .map_err(|e| ApiError::InvalidParams(e.to_string()))?;
    }

    let created = manager
        .create(profile)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&created)?)
}

/// Get a profile
pub async fn get(manager: &Arc<ProfileManager>, params: Option<Value>) -> Result<Value> {
    let params: ProfileIdParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let profile = manager
        .get(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&profile)?)
}

/// List all profiles
pub async fn list(manager: &Arc<ProfileManager>, _params: Option<Value>) -> Result<Value> {
    let profiles = manager
        .list()
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&profiles)?)
}

/// Update a profile
pub async fn update(manager: &Arc<ProfileManager>, params: Option<Value>) -> Result<Value> {
    let params: UpdateProfileParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    let updated = manager
        .update(params.profile)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&updated)?)
}

/// Delete a profile
pub async fn delete(manager: &Arc<ProfileManager>, params: Option<Value>) -> Result<Value> {
    let params: ProfileIdParams = serde_json::from_value(
        params.ok_or_else(|| ApiError::InvalidParams("Missing params".to_string()))?,
    )?;

    manager
        .delete(&params.profile_id)
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(json!({
        "status": "deleted"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use uuid::Uuid;

    #[tokio::test]
    async fn test_create_profile() {
        let manager = Arc::new(ProfileManager::new().unwrap());
        manager.load_all().await.unwrap();

        let params = json!({
            "id": format!("test_api_{}", Uuid::new_v4()),
            "name": "Test API Profile"
        });

        let result = create(&manager, Some(params)).await.unwrap();
        assert!(result.get("id").is_some());
    }

    #[tokio::test]
    async fn test_list_profiles() {
        let manager = Arc::new(ProfileManager::new().unwrap());
        manager.load_all().await.unwrap();

        let result = list(&manager, None).await.unwrap();
        assert!(result.is_array());
    }
}
