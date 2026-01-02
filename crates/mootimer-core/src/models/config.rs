//! Application configuration

use crate::{Error, Result};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    pub version: String,
    pub default_profile: Option<String>,
    pub daemon: DaemonConfig,
    pub pomodoro: PomodoroConfig,
    pub sync: SyncConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DaemonConfig {
    pub socket_path: String,
    pub log_level: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct PomodoroConfig {
    pub work_duration: u64,
    pub short_break: u64,
    pub long_break: u64,
    pub sessions_until_long_break: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SyncConfig {
    pub auto_commit: bool,
    pub auto_push: bool,
    pub remote_url: Option<String>,
}

impl Config {
    /// Validate the configuration
    pub fn validate(&self) -> Result<()> {
        self.daemon.validate()?;
        self.pomodoro.validate()?;
        self.sync.validate()?;
        Ok(())
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: "1.0.0".to_string(),
            default_profile: None,
            daemon: DaemonConfig::default(),
            pomodoro: PomodoroConfig::default(),
            sync: SyncConfig::default(),
        }
    }
}

impl DaemonConfig {
    /// Validate daemon configuration
    pub fn validate(&self) -> Result<()> {
        if self.socket_path.trim().is_empty() {
            return Err(Error::Validation("Socket path cannot be empty".to_string()));
        }

        let valid_log_levels = ["error", "warn", "info", "debug", "trace"];
        if !valid_log_levels.contains(&self.log_level.as_str()) {
            return Err(Error::Validation(format!(
                "Invalid log level '{}'. Must be one of: {}",
                self.log_level,
                valid_log_levels.join(", ")
            )));
        }

        Ok(())
    }
}

impl Default for DaemonConfig {
    fn default() -> Self {
        Self {
            socket_path: "/tmp/mootimer.sock".to_string(),
            log_level: "info".to_string(),
        }
    }
}

impl PomodoroConfig {
    /// Validate pomodoro configuration
    pub fn validate(&self) -> Result<()> {
        if self.work_duration == 0 {
            return Err(Error::Validation(
                "Work duration must be greater than 0".to_string(),
            ));
        }

        if self.short_break == 0 {
            return Err(Error::Validation(
                "Short break duration must be greater than 0".to_string(),
            ));
        }

        if self.long_break == 0 {
            return Err(Error::Validation(
                "Long break duration must be greater than 0".to_string(),
            ));
        }

        if self.sessions_until_long_break == 0 {
            return Err(Error::Validation(
                "Sessions until long break must be greater than 0".to_string(),
            ));
        }

        // Reasonable upper limits
        const MAX_DURATION: u64 = 7200; // 2 hours
        if self.work_duration > MAX_DURATION {
            return Err(Error::Validation(format!(
                "Work duration too long (max {} seconds)",
                MAX_DURATION
            )));
        }

        if self.short_break > MAX_DURATION {
            return Err(Error::Validation(format!(
                "Short break too long (max {} seconds)",
                MAX_DURATION
            )));
        }

        if self.long_break > MAX_DURATION {
            return Err(Error::Validation(format!(
                "Long break too long (max {} seconds)",
                MAX_DURATION
            )));
        }

        Ok(())
    }

    /// Get work duration in minutes
    pub fn work_minutes(&self) -> u64 {
        self.work_duration / 60
    }

    /// Get short break in minutes
    pub fn short_break_minutes(&self) -> u64 {
        self.short_break / 60
    }

    /// Get long break in minutes
    pub fn long_break_minutes(&self) -> u64 {
        self.long_break / 60
    }
}

impl Default for PomodoroConfig {
    fn default() -> Self {
        Self {
            work_duration: 1500, // 25 minutes
            short_break: 300,    // 5 minutes
            long_break: 900,     // 15 minutes
            sessions_until_long_break: 4,
        }
    }
}

impl SyncConfig {
    /// Validate sync configuration
    pub fn validate(&self) -> Result<()> {
        if self.auto_push && self.remote_url.is_none() {
            return Err(Error::Validation(
                "Remote URL must be set when auto-push is enabled".to_string(),
            ));
        }

        if let Some(ref url) = self.remote_url {
            if url.trim().is_empty() {
                return Err(Error::Validation("Remote URL cannot be empty".to_string()));
            }
        }

        Ok(())
    }
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            auto_commit: true,
            auto_push: false,
            remote_url: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.version, "1.0.0");
        assert!(config.default_profile.is_none());
        assert!(config.validate().is_ok());
    }

    #[test]
    fn test_daemon_config_validation() {
        let mut config = DaemonConfig::default();
        assert!(config.validate().is_ok());

        config.socket_path = "".to_string();
        assert!(config.validate().is_err());

        config.socket_path = "/tmp/test.sock".to_string();
        config.log_level = "invalid".to_string();
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_pomodoro_config_validation() {
        let config = PomodoroConfig::default();
        assert!(config.validate().is_ok());
        assert_eq!(config.work_minutes(), 25);
        assert_eq!(config.short_break_minutes(), 5);
        assert_eq!(config.long_break_minutes(), 15);
    }

    #[test]
    fn test_pomodoro_config_invalid() {
        let mut config = PomodoroConfig {
            work_duration: 0,
            ..PomodoroConfig::default()
        };
        assert!(config.validate().is_err());

        config.work_duration = 10000; // Too long
        assert!(config.validate().is_err());
    }

    #[test]
    fn test_sync_config_validation() {
        let config = SyncConfig::default();
        assert!(config.validate().is_ok());

        let mut config_with_push = SyncConfig {
            auto_push: true,
            auto_commit: true,
            remote_url: None,
        };
        assert!(config_with_push.validate().is_err());

        config_with_push.remote_url = Some("git@github.com:user/repo.git".to_string());
        assert!(config_with_push.validate().is_ok());
    }
}
