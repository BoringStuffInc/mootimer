//! Storage management for MooTimer data

pub mod config;
pub mod entry;
pub mod profile;
pub mod task;

pub use config::ConfigStorage;
pub use entry::EntryStorage;
pub use profile::ProfileStorage;
pub use task::TaskStorage;

use std::path::PathBuf;

/// Get the XDG data directory for MooTimer
/// Defaults to ~/.local/share/mootimer
pub fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .expect("Could not find data directory")
        .join("mootimer")
}

/// Get the XDG config directory for MooTimer
/// Defaults to ~/.config/mootimer
pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .expect("Could not find config directory")
        .join("mootimer")
}

/// Initialize the data directory structure
pub fn init_data_dir() -> crate::Result<PathBuf> {
    let data_dir = get_data_dir();
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(data_dir.join("profiles"))?;
    Ok(data_dir)
}

/// Initialize the config directory structure
pub fn init_config_dir() -> crate::Result<PathBuf> {
    let config_dir = get_config_dir();
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}
