use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::event_manager::EventManager;
use crate::events::ProfileEvent;
use mootimer_core::{
    Result as CoreResult,
    models::Profile,
    storage::{ProfileStorage, init_data_dir},
};

#[derive(Debug, thiserror::Error)]
pub enum ProfileManagerError {
    #[error("Profile not found: {0}")]
    NotFound(String),

    #[error("Profile already exists: {0}")]
    AlreadyExists(String),

    #[error("Storage error: {0}")]
    Storage(#[from] mootimer_core::Error),

    #[error("Invalid profile: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, ProfileManagerError>;

pub struct ProfileManager {
    storage: ProfileStorage,
    cache: Arc<RwLock<HashMap<String, Profile>>>,
    event_manager: Arc<EventManager>,
}

impl ProfileManager {
    pub fn new(event_manager: Arc<EventManager>) -> CoreResult<Self> {
        let data_dir = init_data_dir()?;
        let storage = ProfileStorage::new(data_dir);

        Ok(Self {
            storage,
            cache: Arc::new(RwLock::new(HashMap::new())),
            event_manager,
        })
    }

    pub async fn load_all(&self) -> Result<()> {
        let profiles = self.storage.list()?;
        let mut cache = self.cache.write().await;

        cache.clear();
        for profile in profiles {
            cache.insert(profile.id.clone(), profile);
        }

        Ok(())
    }

    pub async fn create(&self, profile: Profile) -> Result<Profile> {
        profile
            .validate()
            .map_err(|e| ProfileManagerError::Invalid(e.to_string()))?;

        {
            let cache = self.cache.read().await;
            if cache.contains_key(&profile.id) {
                return Err(ProfileManagerError::AlreadyExists(profile.id.clone()));
            }
        }

        self.storage.save(&profile)?;

        {
            let mut cache = self.cache.write().await;
            cache.insert(profile.id.clone(), profile.clone());
        }

        let event = ProfileEvent::created(profile.clone());
        self.event_manager.emit_profile(event);

        Ok(profile)
    }

    pub async fn get(&self, profile_id: &str) -> Result<Profile> {
        {
            let cache = self.cache.read().await;
            if let Some(profile) = cache.get(profile_id) {
                return Ok(profile.clone());
            }
        }

        let profile = self
            .storage
            .load(profile_id)
            .map_err(|_| ProfileManagerError::NotFound(profile_id.to_string()))?;

        {
            let mut cache = self.cache.write().await;
            cache.insert(profile_id.to_string(), profile.clone());
        }

        Ok(profile)
    }

    pub async fn list(&self) -> Result<Vec<Profile>> {
        let cache = self.cache.read().await;
        Ok(cache.values().cloned().collect())
    }

    pub async fn update(&self, mut profile: Profile) -> Result<Profile> {
        profile
            .validate()
            .map_err(|e| ProfileManagerError::Invalid(e.to_string()))?;

        {
            let cache = self.cache.read().await;
            if !cache.contains_key(&profile.id) {
                return Err(ProfileManagerError::NotFound(profile.id.clone()));
            }
        }

        profile.touch();

        self.storage.save(&profile)?;

        {
            let mut cache = self.cache.write().await;
            cache.insert(profile.id.clone(), profile.clone());
        }

        let event = ProfileEvent::updated(profile.clone());
        self.event_manager.emit_profile(event);

        Ok(profile)
    }

    pub async fn delete(&self, profile_id: &str) -> Result<()> {
        {
            let cache = self.cache.read().await;
            if !cache.contains_key(profile_id) {
                return Err(ProfileManagerError::NotFound(profile_id.to_string()));
            }
        }

        self.storage.delete(profile_id)?;

        {
            let mut cache = self.cache.write().await;
            cache.remove(profile_id);
        }

        let event = ProfileEvent::deleted(profile_id.to_string());
        self.event_manager.emit_profile(event);

        Ok(())
    }

    pub async fn exists(&self, profile_id: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_key(profile_id)
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new(Arc::new(crate::event_manager::EventManager::new()))
            .expect("Failed to create ProfileManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_manager::EventManager;
    use mootimer_core::models::Profile;
    use serial_test::serial;
    use std::sync::Arc;
    use tempfile::TempDir;
    use uuid::Uuid;

    fn unique_id(prefix: &str) -> String {
        format!("{}_{}", prefix, Uuid::new_v4())
    }

    fn create_manager(_temp_dir: &TempDir) -> ProfileManager {
        let event_manager = Arc::new(EventManager::new());
        unsafe {
            std::env::set_var("HOME", _temp_dir.path());
            std::env::set_var("XDG_DATA_HOME", _temp_dir.path().join("data"));
            std::env::set_var("XDG_CONFIG_HOME", _temp_dir.path().join("config"));
        }
        ProfileManager::new(event_manager).unwrap()
    }

    #[tokio::test]
    #[serial]
    async fn test_create_profile() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        manager.load_all().await.unwrap();

        let id = unique_id("test_profile");
        let profile = Profile::new(id.clone(), "Test Profile".to_string()).unwrap();
        let created = manager.create(profile).await.unwrap();

        assert_eq!(created.id, id);
        assert_eq!(created.name, "Test Profile");
    }

    #[tokio::test]
    #[serial]
    async fn test_get_profile() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        manager.load_all().await.unwrap();

        let id = unique_id("test_get");
        let profile = Profile::new(id.clone(), "Test Get".to_string()).unwrap();
        manager.create(profile).await.unwrap();

        let retrieved = manager.get(&id).await.unwrap();
        assert_eq!(retrieved.id, id);
    }

    #[tokio::test]
    #[serial]
    async fn test_list_profiles() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        manager.load_all().await.unwrap();

        let id1 = unique_id("test_list1");
        let id2 = unique_id("test_list2");
        let profile1 = Profile::new(id1, "Test 1".to_string()).unwrap();
        let profile2 = Profile::new(id2, "Test 2".to_string()).unwrap();

        manager.create(profile1).await.unwrap();
        manager.create(profile2).await.unwrap();

        let profiles = manager.list().await.unwrap();
        assert!(profiles.len() >= 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_profile() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        manager.load_all().await.unwrap();

        let id = unique_id("test_update");
        let mut profile = Profile::new(id.clone(), "Old Name".to_string()).unwrap();
        manager.create(profile.clone()).await.unwrap();

        profile.update_name("New Name".to_string()).unwrap();
        let updated = manager.update(profile).await.unwrap();

        assert_eq!(updated.name, "New Name");
    }

    #[tokio::test]
    #[serial]
    async fn test_delete_profile() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        manager.load_all().await.unwrap();

        let id = unique_id("test_delete");
        let profile = Profile::new(id.clone(), "Delete Me".to_string()).unwrap();
        manager.create(profile).await.unwrap();

        manager.delete(&id).await.unwrap();

        let result = manager.get(&id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    #[serial]
    async fn test_duplicate_profile() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        manager.load_all().await.unwrap();

        let id = unique_id("test_dup");
        let profile = Profile::new(id, "Duplicate".to_string()).unwrap();
        manager.create(profile.clone()).await.unwrap();

        let result = manager.create(profile).await;
        assert!(result.is_err());
    }
}
