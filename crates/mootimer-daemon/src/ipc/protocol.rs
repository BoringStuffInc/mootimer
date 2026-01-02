//! JSON-RPC 2.0 protocol implementation

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: RequestId,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<JsonRpcError>,
    pub id: RequestId,
}

/// JSON-RPC 2.0 Notification (server -> client)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}

/// Request ID (can be string, number, or null)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(untagged)]
pub enum RequestId {
    Number(i64),
    String(String),
    Null,
}

/// JSON-RPC Error
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JsonRpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

impl JsonRpcError {
    /// Parse error (-32700)
    pub fn parse_error(message: impl Into<String>) -> Self {
        Self {
            code: -32700,
            message: message.into(),
            data: None,
        }
    }

    /// Invalid request (-32600)
    pub fn invalid_request(message: impl Into<String>) -> Self {
        Self {
            code: -32600,
            message: message.into(),
            data: None,
        }
    }

    /// Method not found (-32601)
    pub fn method_not_found(method: &str) -> Self {
        Self {
            code: -32601,
            message: format!("Method not found: {}", method),
            data: None,
        }
    }

    /// Invalid params (-32602)
    pub fn invalid_params(message: impl Into<String>) -> Self {
        Self {
            code: -32602,
            message: message.into(),
            data: None,
        }
    }

    /// Internal error (-32603)
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self {
            code: -32603,
            message: message.into(),
            data: None,
        }
    }

    /// Custom application error
    pub fn application_error(code: i32, message: impl Into<String>) -> Self {
        Self {
            code,
            message: message.into(),
            data: None,
        }
    }
}

impl Request {
    /// Create a new request
    pub fn new(method: String, params: Option<Value>, id: RequestId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
            id,
        }
    }

    /// Validate the request
    pub fn validate(&self) -> Result<(), JsonRpcError> {
        if self.jsonrpc != "2.0" {
            return Err(JsonRpcError::invalid_request("Invalid JSON-RPC version"));
        }
        Ok(())
    }
}

impl Response {
    /// Create a success response
    pub fn success(result: Value, id: RequestId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: Some(result),
            error: None,
            id,
        }
    }

    /// Create an error response
    pub fn error(error: JsonRpcError, id: RequestId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            result: None,
            error: Some(error),
            id,
        }
    }
}

impl Notification {
    /// Create a new notification
    pub fn new(method: String, params: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method,
            params,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_request_serialization() {
        let req = Request::new(
            "timer.start".to_string(),
            Some(json!({"profile_id": "test"})),
            RequestId::Number(1),
        );

        let json = serde_json::to_string(&req).unwrap();
        assert!(json.contains("\"method\":\"timer.start\""));
        assert!(json.contains("\"jsonrpc\":\"2.0\""));
    }

    #[test]
    fn test_response_success() {
        let resp = Response::success(json!({"timer_id": "123"}), RequestId::Number(1));

        assert!(resp.result.is_some());
        assert!(resp.error.is_none());
        assert_eq!(resp.jsonrpc, "2.0");
    }

    #[test]
    fn test_response_error() {
        let error = JsonRpcError::method_not_found("test.method");
        let resp = Response::error(error, RequestId::Number(1));

        assert!(resp.result.is_none());
        assert!(resp.error.is_some());
        assert_eq!(resp.error.unwrap().code, -32601);
    }

    #[test]
    fn test_notification_serialization() {
        let notif = Notification::new("timer.tick".to_string(), json!({"elapsed": 100}));

        let json = serde_json::to_string(&notif).unwrap();
        assert!(json.contains("\"method\":\"timer.tick\""));
    }

    #[test]
    fn test_request_id_types() {
        let req1 = Request::new("test".to_string(), None, RequestId::Number(42));
        let req2 = Request::new(
            "test".to_string(),
            None,
            RequestId::String("abc".to_string()),
        );

        let json1 = serde_json::to_string(&req1).unwrap();
        let json2 = serde_json::to_string(&req2).unwrap();

        assert!(json1.contains("\"id\":42"));
        assert!(json2.contains("\"id\":\"abc\""));
    }
}
