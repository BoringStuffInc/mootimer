//! Configuration storage operations

use crate::{models::Config, Result};
use std::path::PathBuf;

pub struct ConfigStorage {
    config_dir: PathBuf,
}

impl ConfigStorage {
    pub fn new(config_dir: PathBuf) -> Self {
        Self { config_dir }
    }

    pub fn load(&self) -> Result<Config> {
        let config_path = self.config_dir.join("config.json");

        if !config_path.exists() {
            let config = Config::default();
            self.save(&config)?;
            return Ok(config);
        }

        let content = std::fs::read_to_string(config_path)?;

        // Handle empty file case
        if content.trim().is_empty() {
            let config = Config::default();
            self.save(&config)?;
            return Ok(config);
        }

        let config: Config = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self, config: &Config) -> Result<()> {
        std::fs::create_dir_all(&self.config_dir)?;

        let config_path = self.config_dir.join("config.json");
        let content = serde_json::to_string_pretty(config)?;
        std::fs::write(config_path, content)?;

        Ok(())
    }
}
