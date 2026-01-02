//! MooTimer Client Library
//!
//! Provides a client for communicating with the MooTimer daemon via Unix sockets.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::UnixStream;
use tokio::sync::{mpsc, Mutex, RwLock};

/// JSON-RPC 2.0 Request
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: RequestId,
}

/// JSON-RPC 2.0 Response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
    pub id: RequestId,
}

/// JSON-RPC 2.0 Error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

/// JSON-RPC 2.0 Notification (no id field)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}

/// Request ID (can be string, number, or null)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
    Null,
}

impl Request {
    /// Create a new request
    pub fn new(method: impl Into<String>, params: Option<Value>, id: RequestId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
            id,
        }
    }
}

/// Persistent connection state
struct PersistentConnection {
    writer: Arc<Mutex<BufWriter<tokio::io::WriteHalf<UnixStream>>>>,
    pending_responses: Arc<RwLock<HashMap<i64, mpsc::Sender<Response>>>>,
}

/// MooTimer daemon client
pub struct MooTimerClient {
    socket_path: String,
    request_counter: std::sync::atomic::AtomicI64,
    persistent_conn: Arc<Mutex<Option<PersistentConnection>>>,
}

impl MooTimerClient {
    /// Create a new client
    pub fn new(socket_path: impl Into<String>) -> Self {
        Self {
            socket_path: socket_path.into(),
            request_counter: std::sync::atomic::AtomicI64::new(1),
            persistent_conn: Arc::new(Mutex::new(None)),
        }
    }

    /// Start a persistent connection and subscribe to notifications
    /// Returns a receiver for timer event notifications
    pub async fn subscribe_notifications(&self) -> Result<mpsc::Receiver<Notification>> {
        let mut conn_lock = self.persistent_conn.lock().await;

        // If already connected, return error
        if conn_lock.is_some() {
            anyhow::bail!("Already subscribed to notifications");
        }

        // Connect to daemon
        let stream = self.connect().await?;
        let (read_half, write_half) = tokio::io::split(stream);
        let writer = Arc::new(Mutex::new(BufWriter::new(write_half)));

        // Create channels
        let (notif_tx, notif_rx) = mpsc::channel::<Notification>(100);
        let pending_responses: Arc<RwLock<HashMap<i64, mpsc::Sender<Response>>>> = Arc::new(RwLock::new(HashMap::new()));

        // Spawn background task to read messages
        let pending_clone = pending_responses.clone();
        let notif_tx_clone = notif_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(read_half);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break, // Connection closed
                    Ok(_) => {
                        // Try to parse as notification first (no id field)
                        if let Ok(notification) = serde_json::from_str::<Notification>(&line) {
                            let _ = notif_tx_clone.send(notification).await;
                        }
                        // Otherwise try as response (has id field)
                        else if let Ok(response) = serde_json::from_str::<Response>(&line) {
                            if let RequestId::Number(id) = response.id {
                                let pending = pending_clone.read().await;
                                if let Some(tx) = pending.get(&id) {
                                    let _ = tx.send(response).await;
                                }
                            }
                        }
                    }
                }
            }
        });

        // Store persistent connection
        *conn_lock = Some(PersistentConnection {
            writer,
            pending_responses,
        });

        Ok(notif_rx)
    }

    /// Send a request using the persistent connection (if available)
    async fn call_persistent(&self, method: impl Into<String>, params: Option<Value>) -> Result<Value> {
        let conn_lock = self.persistent_conn.lock().await;

        if let Some(conn) = conn_lock.as_ref() {
            let request = Request::new(method, params, self.next_id());
            let request_id = if let RequestId::Number(id) = request.id.clone() {
                id
            } else {
                anyhow::bail!("Invalid request ID");
            };

            // Create response channel
            let (tx, mut rx) = mpsc::channel::<Response>(1);

            // Register for response
            {
                let mut pending = conn.pending_responses.write().await;
                pending.insert(request_id, tx);
            }

            // Send request
            {
                let mut writer = conn.writer.lock().await;
                let request_json = serde_json::to_string(&request)?;
                writer.write_all(request_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }

            // Wait for response
            let response = rx.recv().await.ok_or_else(|| anyhow::anyhow!("No response received"))?;

            // Cleanup
            {
                let mut pending = conn.pending_responses.write().await;
                pending.remove(&request_id);
            }

            // Check for error
            if let Some(error) = response.error {
                anyhow::bail!("RPC error {}: {}", error.code, error.message);
            }

            return Ok(response.result.unwrap_or(Value::Null));
        }

        // Fallback to one-shot connection
        drop(conn_lock);
        self.call_oneshot(method, params).await
    }

    /// Send a one-shot request (creates new connection)
    async fn call_oneshot(&self, method: impl Into<String>, params: Option<Value>) -> Result<Value> {
        let mut stream = self.connect().await?;
        let request = Request::new(method, params, self.next_id());

        // Send request
        let request_json = serde_json::to_string(&request)?;
        stream.write_all(request_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;

        // Read response
        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: Response = serde_json::from_str(&line)?;

        // Check for error
        if let Some(error) = response.error {
            anyhow::bail!("RPC error {}: {}", error.code, error.message);
        }

        Ok(response.result.unwrap_or(Value::Null))
    }

    /// Connect to the daemon
    async fn connect(&self) -> Result<UnixStream> {
        Ok(UnixStream::connect(&self.socket_path).await?)
    }

    /// Get next request ID
    fn next_id(&self) -> RequestId {
        let id = self
            .request_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        RequestId::Number(id)
    }

    /// Send a request and receive a response
    /// If persistent connection is active, uses it; otherwise creates one-shot connection
    pub async fn call(&self, method: impl Into<String>, params: Option<Value>) -> Result<Value> {
        self.call_persistent(method, params).await
    }

    // Timer methods

    /// Start a manual timer
    pub async fn timer_start_manual(
        &self,
        profile_id: &str,
        task_id: Option<&str>,
    ) -> Result<Value> {
        self.call(
            "timer.start_manual",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "task_id": task_id,
            })),
        )
        .await
    }

    /// Start a pomodoro timer
    pub async fn timer_start_pomodoro(
        &self,
        profile_id: &str,
        task_id: Option<&str>,
        work_duration_minutes: Option<u64>,
    ) -> Result<Value> {
        let mut params = serde_json::json!({
            "profile_id": profile_id,
            "task_id": task_id,
        });

        // If custom duration provided, create config with it
        if let Some(duration) = work_duration_minutes {
            params["config"] = serde_json::json!({
                "work_duration": duration * 60, // Convert minutes to seconds
                "short_break": 300,  // 5 minutes
                "long_break": 900,   // 15 minutes
                "sessions_until_long_break": 4,
            });
        }

        self.call("timer.start_pomodoro", Some(params)).await
    }

    /// Start a countdown timer
    pub async fn timer_start_countdown(
        &self,
        profile_id: &str,
        task_id: Option<&str>,
        duration_minutes: u64,
    ) -> Result<Value> {
        self.call(
            "timer.start_countdown",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "task_id": task_id,
                "duration_minutes": duration_minutes,
            })),
        )
        .await
    }

    /// Pause timer
    pub async fn timer_pause(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.pause",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    /// Resume timer
    pub async fn timer_resume(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.resume",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    /// Stop timer
    pub async fn timer_stop(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.stop",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    /// Cancel timer
    pub async fn timer_cancel(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.cancel",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    /// Get timer status
    pub async fn timer_get(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.get",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    /// List all active timers
    pub async fn timer_list(&self) -> Result<Value> {
        self.call("timer.list", None).await
    }

    // Profile methods

    /// Create a profile
    pub async fn profile_create(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        self.call(
            "profile.create",
            Some(serde_json::json!({
                "id": id,
                "name": name,
                "description": description,
            })),
        )
        .await
    }

    /// Get a profile
    pub async fn profile_get(&self, id: &str) -> Result<Value> {
        self.call(
            "profile.get",
            Some(serde_json::json!({
                "profile_id": id,
            })),
        )
        .await
    }

    /// List all profiles
    pub async fn profile_list(&self) -> Result<Value> {
        self.call("profile.list", None).await
    }

    /// Delete a profile
    pub async fn profile_delete(&self, id: &str) -> Result<Value> {
        self.call(
            "profile.delete",
            Some(serde_json::json!({
                "profile_id": id,
            })),
        )
        .await
    }

    /// Update a profile
    pub async fn profile_update(&self, profile: Value) -> Result<Value> {
        self.call(
            "profile.update",
            Some(serde_json::json!({
                "profile": profile,
            })),
        )
        .await
    }

    // Task methods

    /// Create a task
    pub async fn task_create(
        &self,
        profile_id: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        self.call(
            "task.create",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "title": title,
                "description": description,
            })),
        )
        .await
    }

    /// List tasks for a profile
    pub async fn task_list(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "task.list",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    // Entry methods

    /// Get today's entries
    pub async fn entry_today(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "entry.today",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    /// Get today's stats
    pub async fn entry_stats_today(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "entry.stats_today",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    // Sync methods

    /// Get sync status
    pub async fn sync_status(&self) -> Result<Value> {
        self.call("sync.status", None).await
    }

    /// Perform sync
    pub async fn sync_sync(&self) -> Result<Value> {
        self.call("sync.sync", None).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_creation() {
        let req = Request::new("test.method", None, RequestId::Number(1));
        assert_eq!(req.method, "test.method");
        assert_eq!(req.id, RequestId::Number(1));
    }
}
