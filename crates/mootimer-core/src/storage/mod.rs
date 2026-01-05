
pub mod config;
pub mod entry;
pub mod profile;
pub mod task;

pub use config::ConfigStorage;
pub use entry::EntryStorage;
pub use profile::ProfileStorage;
pub use task::TaskStorage;

use std::path::PathBuf;

pub fn get_data_dir() -> PathBuf {
    dirs::data_dir()
        .expect("Could not find data directory")
        .join("mootimer")
}

pub fn get_config_dir() -> PathBuf {
    dirs::config_dir()
        .expect("Could not find config directory")
        .join("mootimer")
}

pub fn init_data_dir() -> crate::Result<PathBuf> {
    let data_dir = get_data_dir();
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(data_dir.join("profiles"))?;
    Ok(data_dir)
}

pub fn init_config_dir() -> crate::Result<PathBuf> {
    let config_dir = get_config_dir();
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir)
}
