//! JIRA API client

pub struct JiraClient {
    _base_url: String,
    // TODO: Add HTTP client
}

impl JiraClient {
    pub fn new(base_url: String) -> Self {
        Self {
            _base_url: base_url,
        }
    }
}
