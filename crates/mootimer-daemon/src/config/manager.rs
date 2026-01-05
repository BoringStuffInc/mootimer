//! Configuration manager

use std::sync::Arc;
use tokio::sync::RwLock;

use mootimer_core::{
    Result as CoreResult,
    models::Config,
    storage::{ConfigStorage, init_config_dir},
};

/// Config manager error
#[derive(Debug, thiserror::Error)]
pub enum ConfigManagerError {
    #[error("Storage error: {0}")]
    Storage(#[from] mootimer_core::Error),

    #[error("Invalid config: {0}")]
    Invalid(String),
}

pub type Result<T> = std::result::Result<T, ConfigManagerError>;

/// Manages application configuration
pub struct ConfigManager {
    storage: ConfigStorage,
    config: Arc<RwLock<Config>>,
}

impl ConfigManager {
    pub fn new() -> CoreResult<Self> {
        let config_dir = init_config_dir()?;
        let storage = ConfigStorage::new(config_dir);

        // Load or create default config
        let config = storage.load()?;

        Ok(Self {
            storage,
            config: Arc::new(RwLock::new(config)),
        })
    }

    pub async fn get(&self) -> Config {
        self.config.read().await.clone()
    }

    pub async fn update(&self, config: Config) -> Result<Config> {
        // Validate config
        config
            .validate()
            .map_err(|e| ConfigManagerError::Invalid(e.to_string()))?;

        // Save to storage
        self.storage.save(&config)?;

        // Update in-memory config
        {
            let mut current = self.config.write().await;
            *current = config.clone();
        }

        Ok(config)
    }

    pub async fn set_default_profile(&self, profile_id: Option<String>) -> Result<Config> {
        let mut config = self.get().await;
        config.default_profile = profile_id;
        self.update(config).await
    }

    pub async fn update_daemon_config(
        &self,
        socket_path: Option<String>,
        log_level: Option<String>,
    ) -> Result<Config> {
        let mut config = self.get().await;

        if let Some(path) = socket_path {
            config.daemon.socket_path = path;
        }

        if let Some(level) = log_level {
            config.daemon.log_level = level;
        }

        self.update(config).await
    }

    pub async fn update_pomodoro_config(
        &self,
        work_duration: Option<u64>,
        short_break: Option<u64>,
        long_break: Option<u64>,
        sessions_until_long_break: Option<u32>,
        countdown_default: Option<u64>,
    ) -> Result<Config> {
        let mut config = self.get().await;

        if let Some(duration) = work_duration {
            config.pomodoro.work_duration = duration;
        }

        if let Some(duration) = short_break {
            config.pomodoro.short_break = duration;
        }

        if let Some(duration) = long_break {
            config.pomodoro.long_break = duration;
        }

        if let Some(sessions) = sessions_until_long_break {
            config.pomodoro.sessions_until_long_break = sessions;
        }

        if let Some(duration) = countdown_default {
            config.pomodoro.countdown_default = duration;
        }

        self.update(config).await
    }

    pub async fn update_sync_config(
        &self,
        auto_commit: Option<bool>,
        auto_push: Option<bool>,
        remote_url: Option<String>,
    ) -> Result<Config> {
        let mut config = self.get().await;

        if let Some(enabled) = auto_commit {
            config.sync.auto_commit = enabled;
        }

        if let Some(enabled) = auto_push {
            config.sync.auto_push = enabled;
        }

        if let Some(url) = remote_url {
            config.sync.remote_url = Some(url);
        }

        self.update(config).await
    }

    pub async fn reset_to_default(&self) -> Result<Config> {
        let config = Config::default();
        self.update(config).await
    }
}

impl Default for ConfigManager {
    fn default() -> Self {
        Self::new().expect("Failed to create ConfigManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn create_manager(_temp_dir: &TempDir) -> ConfigManager {
        unsafe {
            std::env::set_var("HOME", _temp_dir.path());
            std::env::set_var("XDG_CONFIG_HOME", _temp_dir.path().join("config"));
        }
        ConfigManager::new().unwrap()
    }

    #[tokio::test]
    #[serial]
    async fn test_get_config() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let config = manager.get().await;
        assert_eq!(config.version, "1.0.0");
    }

    #[tokio::test]
    #[serial]
    async fn test_set_default_profile() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);

        let updated = manager
            .set_default_profile(Some("work".to_string()))
            .await
            .unwrap();
        assert_eq!(updated.default_profile, Some("work".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn test_update_pomodoro_config() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);

        let updated = manager
            .update_pomodoro_config(
                Some(1800), // 30 minutes
                None,
                None,
                None,
                None,
            )
            .await
            .unwrap();

        assert_eq!(updated.pomodoro.work_duration, 1800);
    }

    #[tokio::test]
    #[serial]
    async fn test_update_sync_config() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);

        let updated = manager
            .update_sync_config(
                Some(true),
                Some(false),
                Some("git@github.com:user/repo.git".to_string()),
            )
            .await
            .unwrap();

        assert!(updated.sync.auto_commit);
        assert_eq!(
            updated.sync.remote_url,
            Some("git@github.com:user/repo.git".to_string())
        );
    }
}
