//! Profile manager for CRUD operations

use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

use mootimer_core::{
    models::Profile,
    storage::{init_data_dir, ProfileStorage},
    Result as CoreResult,
};

/// Profile manager error
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

/// Manages profiles with caching and persistence
pub struct ProfileManager {
    storage: ProfileStorage,
    cache: Arc<RwLock<HashMap<String, Profile>>>,
}

impl ProfileManager {
    /// Create a new profile manager
    pub fn new() -> CoreResult<Self> {
        let data_dir = init_data_dir()?;
        let storage = ProfileStorage::new(data_dir);

        Ok(Self {
            storage,
            cache: Arc::new(RwLock::new(HashMap::new())),
        })
    }

    /// Load all profiles from storage into cache
    pub async fn load_all(&self) -> Result<()> {
        let profiles = self.storage.list()?;
        let mut cache = self.cache.write().await;

        cache.clear();
        for profile in profiles {
            cache.insert(profile.id.clone(), profile);
        }

        Ok(())
    }

    /// Create a new profile
    pub async fn create(&self, profile: Profile) -> Result<Profile> {
        // Validate profile
        profile
            .validate()
            .map_err(|e| ProfileManagerError::Invalid(e.to_string()))?;

        // Check if already exists
        {
            let cache = self.cache.read().await;
            if cache.contains_key(&profile.id) {
                return Err(ProfileManagerError::AlreadyExists(profile.id.clone()));
            }
        }

        // Save to storage
        self.storage.save(&profile)?;

        // Add to cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(profile.id.clone(), profile.clone());
        }

        Ok(profile)
    }

    /// Get a profile by ID
    pub async fn get(&self, profile_id: &str) -> Result<Profile> {
        // Try cache first
        {
            let cache = self.cache.read().await;
            if let Some(profile) = cache.get(profile_id) {
                return Ok(profile.clone());
            }
        }

        // Load from storage
        let profile = self
            .storage
            .load(profile_id)
            .map_err(|_| ProfileManagerError::NotFound(profile_id.to_string()))?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(profile_id.to_string(), profile.clone());
        }

        Ok(profile)
    }

    /// List all profiles
    pub async fn list(&self) -> Result<Vec<Profile>> {
        let cache = self.cache.read().await;
        Ok(cache.values().cloned().collect())
    }

    /// Update a profile
    pub async fn update(&self, mut profile: Profile) -> Result<Profile> {
        // Validate profile
        profile
            .validate()
            .map_err(|e| ProfileManagerError::Invalid(e.to_string()))?;

        // Check if exists
        {
            let cache = self.cache.read().await;
            if !cache.contains_key(&profile.id) {
                return Err(ProfileManagerError::NotFound(profile.id.clone()));
            }
        }

        // Update timestamp
        profile.touch();

        // Save to storage
        self.storage.save(&profile)?;

        // Update cache
        {
            let mut cache = self.cache.write().await;
            cache.insert(profile.id.clone(), profile.clone());
        }

        Ok(profile)
    }

    /// Delete a profile
    pub async fn delete(&self, profile_id: &str) -> Result<()> {
        // Check if exists
        {
            let cache = self.cache.read().await;
            if !cache.contains_key(profile_id) {
                return Err(ProfileManagerError::NotFound(profile_id.to_string()));
            }
        }

        // Delete from storage
        self.storage.delete(profile_id)?;

        // Remove from cache
        {
            let mut cache = self.cache.write().await;
            cache.remove(profile_id);
        }

        Ok(())
    }

    /// Check if a profile exists
    pub async fn exists(&self, profile_id: &str) -> bool {
        let cache = self.cache.read().await;
        cache.contains_key(profile_id)
    }
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ProfileManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mootimer_core::models::Profile;
    use uuid::Uuid;

    fn unique_id(prefix: &str) -> String {
        format!("{}_{}", prefix, Uuid::new_v4())
    }

    #[tokio::test]
    async fn test_create_profile() {
        let manager = ProfileManager::new().unwrap();
        manager.load_all().await.unwrap();

        let id = unique_id("test_profile");
        let profile = Profile::new(id.clone(), "Test Profile".to_string()).unwrap();
        let created = manager.create(profile).await.unwrap();

        assert_eq!(created.id, id);
        assert_eq!(created.name, "Test Profile");
    }

    #[tokio::test]
    async fn test_get_profile() {
        let manager = ProfileManager::new().unwrap();
        manager.load_all().await.unwrap();

        let id = unique_id("test_get");
        let profile = Profile::new(id.clone(), "Test Get".to_string()).unwrap();
        manager.create(profile).await.unwrap();

        let retrieved = manager.get(&id).await.unwrap();
        assert_eq!(retrieved.id, id);
    }

    #[tokio::test]
    async fn test_list_profiles() {
        let manager = ProfileManager::new().unwrap();
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
    async fn test_update_profile() {
        let manager = ProfileManager::new().unwrap();
        manager.load_all().await.unwrap();

        let id = unique_id("test_update");
        let mut profile = Profile::new(id.clone(), "Old Name".to_string()).unwrap();
        manager.create(profile.clone()).await.unwrap();

        profile.update_name("New Name".to_string()).unwrap();
        let updated = manager.update(profile).await.unwrap();

        assert_eq!(updated.name, "New Name");
    }

    #[tokio::test]
    async fn test_delete_profile() {
        let manager = ProfileManager::new().unwrap();
        manager.load_all().await.unwrap();

        let id = unique_id("test_delete");
        let profile = Profile::new(id.clone(), "Delete Me".to_string()).unwrap();
        manager.create(profile).await.unwrap();

        manager.delete(&id).await.unwrap();

        let result = manager.get(&id).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_duplicate_profile() {
        let manager = ProfileManager::new().unwrap();
        manager.load_all().await.unwrap();

        let id = unique_id("test_dup");
        let profile = Profile::new(id, "Duplicate".to_string()).unwrap();
        manager.create(profile.clone()).await.unwrap();

        let result = manager.create(profile).await;
        assert!(result.is_err());
    }
}
