//! Error types for JIRA integration

use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Authentication error: {0}")]
    Auth(String),

    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("JIRA API error: {0}")]
    Api(String),

    #[error("Not found: {0}")]
    NotFound(String),
}

pub type Result<T> = std::result::Result<T, Error>;
