use crate::{Result, models::Profile};
use std::path::PathBuf;

pub struct ProfileStorage {
    data_dir: PathBuf,
}

impl ProfileStorage {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn load(&self, profile_id: &str) -> Result<Profile> {
        let profile_path = self
            .data_dir
            .join("profiles")
            .join(profile_id)
            .join("profile.json");

        let content = std::fs::read_to_string(profile_path)?;
        let profile: Profile = serde_json::from_str(&content)?;
        Ok(profile)
    }

    pub fn save(&self, profile: &Profile) -> Result<()> {
        let profile_dir = self.data_dir.join("profiles").join(&profile.id);
        std::fs::create_dir_all(&profile_dir)?;

        let profile_path = profile_dir.join("profile.json");
        let content = serde_json::to_string_pretty(profile)?;
        std::fs::write(profile_path, content)?;

        Ok(())
    }

    pub fn list(&self) -> Result<Vec<Profile>> {
        let profiles_dir = self.data_dir.join("profiles");

        if !profiles_dir.exists() {
            return Ok(Vec::new());
        }

        let mut profiles = Vec::new();
        for entry in std::fs::read_dir(profiles_dir)? {
            let entry = entry?;
            if entry.file_type()?.is_dir()
                && let Some(profile_id) = entry.file_name().to_str()
                && let Ok(profile) = self.load(profile_id)
            {
                profiles.push(profile);
            }
        }

        Ok(profiles)
    }

    pub fn delete(&self, profile_id: &str) -> Result<()> {
        let profile_dir = self.data_dir.join("profiles").join(profile_id);
        std::fs::remove_dir_all(profile_dir)?;
        Ok(())
    }
}
