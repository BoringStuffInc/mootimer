use serde::Deserialize;
use serde_json::{Value, json};
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

pub async fn list(manager: &Arc<ProfileManager>, _params: Option<Value>) -> Result<Value> {
    let profiles = manager
        .list()
        .await
        .map_err(|e| ApiError::InvalidParams(e.to_string()))?;

    Ok(serde_json::to_value(&profiles)?)
}

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
    use crate::event_manager::EventManager;
    use std::sync::Arc;

    #[tokio::test]
    async fn test_create_and_get_profile() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(ProfileManager::new(event_manager).unwrap());

        let params = json!({
            "id": "test_profile_api",
            "name": "Test Profile API"
        });

        let result = create(&manager, Some(params)).await.unwrap();
        assert!(result.get("id").is_some());

        let get_params = json!({ "profile_id": "test_profile_api" });
        let get_result = get(&manager, Some(get_params)).await.unwrap();
        assert_eq!(
            get_result.get("name").unwrap().as_str().unwrap(),
            "Test Profile API"
        );
    }

    #[tokio::test]
    async fn test_list_profiles() {
        let event_manager = Arc::new(EventManager::new());
        let manager = Arc::new(ProfileManager::new(event_manager).unwrap());
        let result = list(&manager, None).await.unwrap();
        assert!(result.is_array());
    }
}
