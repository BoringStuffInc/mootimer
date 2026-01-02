//! JIRA authentication

pub struct JiraAuth {
    username: String,
    api_token: String,
}

impl JiraAuth {
    pub fn new(username: String, api_token: String) -> Self {
        Self {
            username,
            api_token,
        }
    }

    pub fn to_basic_auth(&self) -> String {
        use base64::Engine;
        let credentials = format!("{}:{}", self.username, self.api_token);
        format!(
            "Basic {}",
            base64::engine::general_purpose::STANDARD.encode(credentials)
        )
    }
}
