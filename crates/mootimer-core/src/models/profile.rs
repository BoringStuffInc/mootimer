//! Profile data model

use crate::{Error, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub color: Option<String>,
    pub jira: Option<JiraConfig>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct JiraConfig {
    pub url: String,
    pub username: String,
    pub api_token_keychain: String,
    pub default_jql: Option<String>,
}

impl Profile {
    /// Create a new profile with the given ID and name
    pub fn new(id: String, name: String) -> Result<Self> {
        let now = Utc::now();
        let profile = Self {
            id,
            name,
            description: None,
            color: None,
            jira: None,
            created_at: now,
            updated_at: now,
        };
        profile.validate()?;
        Ok(profile)
    }

    /// Validate the profile data
    pub fn validate(&self) -> Result<()> {
        if self.id.trim().is_empty() {
            return Err(Error::Validation("Profile ID cannot be empty".to_string()));
        }

        if self.name.trim().is_empty() {
            return Err(Error::Validation(
                "Profile name cannot be empty".to_string(),
            ));
        }

        // Validate ID format (alphanumeric, underscore, hyphen only)
        if !self
            .id
            .chars()
            .all(|c| c.is_alphanumeric() || c == '_' || c == '-')
        {
            return Err(Error::Validation(
                "Profile ID must contain only alphanumeric characters, underscores, and hyphens"
                    .to_string(),
            ));
        }

        // Validate color format if present (hex color)
        if let Some(ref color) = self.color {
            if !color.starts_with('#') || !(color.len() == 7 || color.len() == 4) {
                return Err(Error::Validation(
                    "Color must be a valid hex color (e.g., #FF5733 or #F73)".to_string(),
                ));
            }
        }

        // Validate JIRA config if present
        if let Some(ref jira) = self.jira {
            jira.validate()?;
        }

        Ok(())
    }

    /// Update the profile's name
    pub fn update_name(&mut self, name: String) -> Result<()> {
        if name.trim().is_empty() {
            return Err(Error::Validation(
                "Profile name cannot be empty".to_string(),
            ));
        }
        self.name = name;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update the profile's description
    pub fn update_description(&mut self, description: Option<String>) {
        self.description = description;
        self.updated_at = Utc::now();
    }

    /// Update the profile's color
    pub fn update_color(&mut self, color: Option<String>) -> Result<()> {
        if let Some(ref c) = color {
            if !c.starts_with('#') || !(c.len() == 7 || c.len() == 4) {
                return Err(Error::Validation(
                    "Color must be a valid hex color (e.g., #FF5733 or #F73)".to_string(),
                ));
            }
        }
        self.color = color;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Update the profile's JIRA configuration
    pub fn update_jira_config(&mut self, jira: Option<JiraConfig>) -> Result<()> {
        if let Some(ref j) = jira {
            j.validate()?;
        }
        self.jira = jira;
        self.updated_at = Utc::now();
        Ok(())
    }

    /// Mark the profile as updated (updates the timestamp)
    pub fn touch(&mut self) {
        self.updated_at = Utc::now();
    }
}

impl JiraConfig {
    /// Create a new JIRA configuration
    pub fn new(url: String, username: String, api_token_keychain: String) -> Result<Self> {
        let config = Self {
            url,
            username,
            api_token_keychain,
            default_jql: None,
        };
        config.validate()?;
        Ok(config)
    }

    /// Validate the JIRA configuration
    pub fn validate(&self) -> Result<()> {
        if self.url.trim().is_empty() {
            return Err(Error::Validation("JIRA URL cannot be empty".to_string()));
        }

        if !self.url.starts_with("http://") && !self.url.starts_with("https://") {
            return Err(Error::Validation(
                "JIRA URL must start with http:// or https://".to_string(),
            ));
        }

        if self.username.trim().is_empty() {
            return Err(Error::Validation(
                "JIRA username cannot be empty".to_string(),
            ));
        }

        if self.api_token_keychain.trim().is_empty() {
            return Err(Error::Validation(
                "JIRA API token keychain ID cannot be empty".to_string(),
            ));
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_new_profile() {
        let profile = Profile::new("test_profile".to_string(), "Test Profile".to_string()).unwrap();
        assert_eq!(profile.id, "test_profile");
        assert_eq!(profile.name, "Test Profile");
        assert!(profile.description.is_none());
        assert!(profile.color.is_none());
        assert!(profile.jira.is_none());
    }

    #[test]
    fn test_profile_validation_empty_id() {
        let result = Profile::new("".to_string(), "Test".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_validation_empty_name() {
        let result = Profile::new("test".to_string(), "".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_validation_invalid_id() {
        let result = Profile::new("test profile!".to_string(), "Test".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_profile_update_name() {
        let mut profile = Profile::new("test".to_string(), "Test".to_string()).unwrap();
        let old_updated_at = profile.updated_at;

        std::thread::sleep(std::time::Duration::from_millis(10));
        profile.update_name("New Name".to_string()).unwrap();

        assert_eq!(profile.name, "New Name");
        assert!(profile.updated_at > old_updated_at);
    }

    #[test]
    fn test_profile_update_color_valid() {
        let mut profile = Profile::new("test".to_string(), "Test".to_string()).unwrap();
        profile.update_color(Some("#FF5733".to_string())).unwrap();
        assert_eq!(profile.color, Some("#FF5733".to_string()));
    }

    #[test]
    fn test_profile_update_color_invalid() {
        let mut profile = Profile::new("test".to_string(), "Test".to_string()).unwrap();
        let result = profile.update_color(Some("FF5733".to_string()));
        assert!(result.is_err());
    }

    #[test]
    fn test_jira_config_new() {
        let config = JiraConfig::new(
            "https://example.atlassian.net".to_string(),
            "user@example.com".to_string(),
            "jira_token".to_string(),
        )
        .unwrap();

        assert_eq!(config.url, "https://example.atlassian.net");
        assert_eq!(config.username, "user@example.com");
    }

    #[test]
    fn test_jira_config_validation_invalid_url() {
        let result = JiraConfig::new(
            "not-a-url".to_string(),
            "user@example.com".to_string(),
            "jira_token".to_string(),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_jira_config_validation_empty_username() {
        let result = JiraConfig::new(
            "https://example.atlassian.net".to_string(),
            "".to_string(),
            "jira_token".to_string(),
        );
        assert!(result.is_err());
    }
}
