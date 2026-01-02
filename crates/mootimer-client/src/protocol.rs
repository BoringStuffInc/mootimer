//! JSON-RPC protocol definitions

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    pub jsonrpc: String,
    pub result: Option<serde_json::Value>,
    pub error: Option<ResponseError>,
    pub id: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    pub data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: serde_json::Value,
}
