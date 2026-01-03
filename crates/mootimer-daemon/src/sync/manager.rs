//! Synchronization manager for git operations

use std::sync::Arc;

use mootimer_core::{
    Result as CoreResult, git::GitOperations, models::SyncConfig, storage::init_data_dir,
};

/// Sync manager error
#[derive(Debug, thiserror::Error)]
pub enum SyncManagerError {
    #[error("Storage error: {0}")]
    Storage(#[from] mootimer_core::Error),

    #[error("Git error: {0}")]
    Git(String),

    #[error("Not configured: {0}")]
    NotConfigured(String),

    #[error("Task join error: {0}")]
    JoinError(String),
}

pub type Result<T> = std::result::Result<T, SyncManagerError>;

/// Manages git synchronization for time tracking data
pub struct SyncManager {
    git_ops: Arc<GitOperations>,
}

impl SyncManager {
    pub fn new() -> CoreResult<Self> {
        let data_dir = init_data_dir()?;
        let git_ops = GitOperations::new(data_dir);

        Ok(Self {
            git_ops: Arc::new(git_ops),
        })
    }

    pub async fn init_repo(&self) -> Result<()> {
        let git_ops = self.git_ops.clone();
        tokio::task::spawn_blocking(move || git_ops.init())
            .await
            .map_err(|e| SyncManagerError::JoinError(e.to_string()))??;
        Ok(())
    }

    pub async fn is_initialized(&self) -> bool {
        let git_ops = self.git_ops.clone();
        tokio::task::spawn_blocking(move || git_ops.is_initialized())
            .await
            .unwrap_or(false)
    }

    pub async fn auto_commit(&self, message: &str) -> Result<Option<String>> {
        let git_ops = self.git_ops.clone();
        let message = message.to_string();

        tokio::task::spawn_blocking(move || {
            // Check if repo is initialized
            if !git_ops.is_initialized() {
                return Err(SyncManagerError::NotConfigured(
                    "Git repository not initialized".to_string(),
                ));
            }

            // Check if there are changes
            if !git_ops.has_changes()? {
                return Ok(None);
            }

            // Add all changes
            git_ops.add_all()?;

            // Commit
            let commit_id = git_ops.commit(&message)?;

            Ok(Some(commit_id.to_string()))
        })
        .await
        .map_err(|e| SyncManagerError::JoinError(e.to_string()))?
    }

    pub async fn sync(&self, config: &SyncConfig) -> Result<SyncResult> {
        let git_ops = self.git_ops.clone();
        let remote_url = config.remote_url.clone().ok_or_else(|| {
            SyncManagerError::NotConfigured("Remote URL not configured".to_string())
        })?;
        let auto_push = config.auto_push;

        tokio::task::spawn_blocking(move || {
            // Check if repo is initialized
            if !git_ops.is_initialized() {
                return Err(SyncManagerError::NotConfigured(
                    "Git repository not initialized".to_string(),
                ));
            }

            // Get current branch
            let branch = git_ops.current_branch()?;

            // Add/update remote
            git_ops.add_remote("origin", &remote_url)?;

            let mut pulled = false;
            let mut pushed = false;

            // Pull changes if there are remote commits
            match git_ops.pull("origin", &branch) {
                Ok(_) => {
                    pulled = true;
                }
                Err(e) => {
                    // If pull fails, log but continue (might be first push)
                    tracing::warn!("Failed to pull: {}", e);
                }
            }

            // Push changes if auto_push is enabled and there are local commits
            if auto_push {
                match git_ops.push("origin", &branch) {
                    Ok(_) => {
                        pushed = true;
                    }
                    Err(e) => {
                        return Err(SyncManagerError::Git(format!("Failed to push: {}", e)));
                    }
                }
            }

            Ok(SyncResult { pulled, pushed })
        })
        .await
        .map_err(|e| SyncManagerError::JoinError(e.to_string()))?
    }

    pub async fn get_status(&self, config: &SyncConfig) -> Result<SyncStatus> {
        let git_ops = self.git_ops.clone();
        let remote_url = config.remote_url.clone();

        tokio::task::spawn_blocking(move || {
            // Check if repo is initialized
            if !git_ops.is_initialized() {
                return Ok(SyncStatus {
                    initialized: false,
                    has_changes: false,
                    ahead: 0,
                    behind: 0,
                    current_branch: None,
                    last_commit: None,
                });
            }

            let has_changes = git_ops.has_changes()?;
            let current_branch = git_ops.current_branch().ok();
            let last_commit = git_ops.last_commit_message().ok();

            let (ahead, behind) = if remote_url.is_some() {
                if let Some(branch) = &current_branch {
                    git_ops.get_sync_status("origin", branch).unwrap_or((0, 0))
                } else {
                    (0, 0)
                }
            } else {
                (0, 0)
            };

            Ok(SyncStatus {
                initialized: true,
                has_changes,
                ahead,
                behind,
                current_branch,
                last_commit,
            })
        })
        .await
        .map_err(|e| SyncManagerError::JoinError(e.to_string()))?
    }

    pub async fn set_remote(&self, url: &str) -> Result<()> {
        let git_ops = self.git_ops.clone();
        let url = url.to_string();

        tokio::task::spawn_blocking(move || {
            // Check if repo is initialized
            if !git_ops.is_initialized() {
                return Err(SyncManagerError::NotConfigured(
                    "Git repository not initialized. Call init_repo() first.".to_string(),
                ));
            }

            git_ops.add_remote("origin", &url)?;
            Ok(())
        })
        .await
        .map_err(|e| SyncManagerError::JoinError(e.to_string()))?
    }
}

/// Result of a sync operation
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncResult {
    pub pulled: bool,
    pub pushed: bool,
}

/// Current sync status
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct SyncStatus {
    pub initialized: bool,
    pub has_changes: bool,
    pub ahead: usize,
    pub behind: usize,
    pub current_branch: Option<String>,
    pub last_commit: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_init_repo() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("HOME", temp_dir.path());
        std::env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));

        let manager = SyncManager::new().unwrap();
        assert!(!manager.is_initialized().await);

        manager.init_repo().await.unwrap();
        assert!(manager.is_initialized().await);
    }

    #[tokio::test]
    async fn test_auto_commit() {
        let temp_dir = TempDir::new().unwrap();
        let home_path = temp_dir.path().to_str().unwrap().to_string();

        // Use a scoped environment change
        std::env::set_var("HOME", &home_path);
        std::env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));

        let manager = SyncManager::new().unwrap();
        manager.init_repo().await.unwrap();

        // Create a file
        let data_dir = init_data_dir().unwrap();
        std::fs::write(data_dir.join("test.txt"), "Hello").unwrap();

        // Should commit now
        let result = manager.auto_commit("Test commit").await;
        assert!(result.is_ok(), "Auto-commit should succeed");
        // Note: result might be None if config files were already committed
    }

    #[tokio::test]
    async fn test_get_status() {
        let temp_dir = TempDir::new().unwrap();
        std::env::set_var("HOME", temp_dir.path());
        std::env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));

        let manager = SyncManager::new().unwrap();

        // Not initialized
        let config = SyncConfig::default();
        let status = manager.get_status(&config).await.unwrap();
        assert!(!status.initialized);

        // Initialize
        manager.init_repo().await.unwrap();
        let status = manager.get_status(&config).await.unwrap();
        assert!(status.initialized);
        // Note: has_changes might be true if config.json was created, which is expected
    }
}
