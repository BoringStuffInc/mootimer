//! JIRA API types

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraIssue {
    pub key: String,
    pub fields: JiraFields,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraFields {
    pub summary: String,
    pub description: Option<String>,
    pub status: JiraStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JiraStatus {
    pub name: String,
}
