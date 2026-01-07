use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::UnixStream;
use tokio::sync::{RwLock, mpsc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Request {
    jsonrpc: String,
    pub method: String,
    pub params: Option<Value>,
    pub id: RequestId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Response {
    jsonrpc: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ResponseError>,
    pub id: RequestId,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Notification {
    pub jsonrpc: String,
    pub method: String,
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(untagged)]
pub enum RequestId {
    String(String),
    Number(i64),
    Null,
}

impl Request {
    pub fn new(method: impl Into<String>, params: Option<Value>, id: RequestId) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            method: method.into(),
            params,
            id,
        }
    }
}

struct PersistentConnection {
    writer: mpsc::Sender<Request>,
    pending_responses: Arc<RwLock<HashMap<i64, mpsc::Sender<Response>>>>,
}

pub struct MooTimerClient {
    socket_path: String,
    request_counter: std::sync::atomic::AtomicI64,
    conn: Arc<RwLock<Option<PersistentConnection>>>,
    notif_tx: Arc<RwLock<Option<mpsc::Sender<Notification>>>>,
}

impl MooTimerClient {
    pub fn new(socket_path: impl Into<String>) -> Self {
        Self {
            socket_path: socket_path.into(),
            request_counter: std::sync::atomic::AtomicI64::new(1),
            conn: Arc::new(RwLock::new(None)),
            notif_tx: Arc::new(RwLock::new(None)),
        }
    }

    async fn ensure_connected(&self) -> Result<PersistentConnection> {
        {
            let conn_lock = self.conn.read().await;
            if let Some(conn) = conn_lock.as_ref() {
                return Ok(PersistentConnection {
                    writer: conn.writer.clone(),
                    pending_responses: conn.pending_responses.clone(),
                });
            }
        }

        let mut conn_lock = self.conn.write().await;
        if let Some(conn) = conn_lock.as_ref() {
            return Ok(PersistentConnection {
                writer: conn.writer.clone(),
                pending_responses: conn.pending_responses.clone(),
            });
        }

        let stream = UnixStream::connect(&self.socket_path).await?;
        let (read_half, mut write_half) = tokio::io::split(stream);
        let mut reader = BufReader::new(read_half);

        let pending_responses: Arc<RwLock<HashMap<i64, mpsc::Sender<Response>>>> =
            Arc::new(RwLock::new(HashMap::new()));
        let (req_tx, mut req_rx) = mpsc::channel::<Request>(100);

        let pending_clone = pending_responses.clone();
        let conn_reset = self.conn.clone();
        let notif_tx_lock = self.notif_tx.clone();

        // Writer task
        tokio::spawn(async move {
            while let Some(req) = req_rx.recv().await {
                let json = match serde_json::to_string(&req) {
                    Ok(j) => j,
                    Err(_) => continue,
                };
                if write_half.write_all(json.as_bytes()).await.is_err()
                    || write_half.write_all(b"\n").await.is_err()
                    || write_half.flush().await.is_err()
                {
                    break;
                }
            }
            let mut c = conn_reset.write().await;
            *c = None;
        });

        // Reader task
        let conn_reset_2 = self.conn.clone();
        tokio::spawn(async move {
            let mut line = String::new();
            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if let Ok(response) = serde_json::from_str::<Response>(&line) {
                            if let RequestId::Number(id) = response.id {
                                let pending = pending_clone.read().await;
                                if let Some(tx) = pending.get(&id) {
                                    let _ = tx.send(response).await;
                                }
                            }
                        } else if let Ok(notification) = serde_json::from_str::<Notification>(&line)
                        {
                            let nt = notif_tx_lock.read().await;
                            if let Some(tx) = nt.as_ref() {
                                let _ = tx.send(notification).await;
                            }
                        }
                    }
                }
            }
            let mut c = conn_reset_2.write().await;
            *c = None;
        });

        let conn = PersistentConnection {
            writer: req_tx,
            pending_responses,
        };
        *conn_lock = Some(PersistentConnection {
            writer: conn.writer.clone(),
            pending_responses: conn.pending_responses.clone(),
        });

        Ok(conn)
    }

    pub async fn subscribe_notifications(&self) -> Result<mpsc::Receiver<Notification>> {
        let (tx, rx) = mpsc::channel(100);
        let mut nt = self.notif_tx.write().await;
        *nt = Some(tx);
        let _ = self.ensure_connected().await?;
        Ok(rx)
    }

    pub async fn call(&self, method: impl Into<String>, params: Option<Value>) -> Result<Value> {
        let conn = self.ensure_connected().await?;
        let id = self.next_id();
        let request = Request::new(method, params, id.clone());

        let req_id = match id {
            RequestId::Number(n) => n,
            _ => anyhow::bail!("Unsupported request ID type"),
        };

        let (tx, mut rx) = mpsc::channel(1);
        {
            let mut pending = conn.pending_responses.write().await;
            pending.insert(req_id, tx);
        }

        if conn.writer.send(request).await.is_err() {
            let mut c = self.conn.write().await;
            *c = None;
            anyhow::bail!("Failed to send request");
        }

        let response =
            match tokio::time::timeout(tokio::time::Duration::from_secs(5), rx.recv()).await {
                Ok(Some(r)) => r,
                _ => {
                    let mut pending = conn.pending_responses.write().await;
                    pending.remove(&req_id);
                    anyhow::bail!("Request timed out or connection closed");
                }
            };

        {
            let mut pending = conn.pending_responses.write().await;
            pending.remove(&req_id);
        }

        if let Some(error) = response.error {
            anyhow::bail!("RPC error {}: {}", error.code, error.message);
        }

        Ok(response.result.unwrap_or(Value::Null))
    }

    fn next_id(&self) -> RequestId {
        let id = self
            .request_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        RequestId::Number(id)
    }

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

        if let Some(duration) = work_duration_minutes {
            params["config"] = serde_json::json!({
                "work_duration": duration * 60,
            });
        }

        self.call("timer.start_pomodoro", Some(params)).await
    }

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

    pub async fn timer_pause(&self, timer_id: &str) -> Result<Value> {
        self.call(
            "timer.pause",
            Some(serde_json::json!({
                "timer_id": timer_id,
            })),
        )
        .await
    }

    pub async fn timer_resume(&self, timer_id: &str) -> Result<Value> {
        self.call(
            "timer.resume",
            Some(serde_json::json!({
                "timer_id": timer_id,
            })),
        )
        .await
    }

    pub async fn timer_stop(&self, timer_id: &str) -> Result<Value> {
        self.call(
            "timer.stop",
            Some(serde_json::json!({
                "timer_id": timer_id,
            })),
        )
        .await
    }

    pub async fn timer_cancel(&self, timer_id: &str) -> Result<Value> {
        self.call(
            "timer.cancel",
            Some(serde_json::json!({
                "timer_id": timer_id,
            })),
        )
        .await
    }

    /// Get first timer for a profile (backward compatible convenience method)
    pub async fn timer_get(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.get_by_profile",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    /// Get a specific timer by its ID
    pub async fn timer_get_by_id(&self, timer_id: &str) -> Result<Value> {
        self.call(
            "timer.get",
            Some(serde_json::json!({
                "timer_id": timer_id,
            })),
        )
        .await
    }

    /// List all timers for a specific profile
    pub async fn timer_list_by_profile(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.list_by_profile",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    /// List all active timers across all profiles
    pub async fn timer_list(&self) -> Result<Value> {
        self.call("timer.list", None).await
    }

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

    pub async fn profile_get(&self, id: &str) -> Result<Value> {
        self.call(
            "profile.get",
            Some(serde_json::json!({
                "profile_id": id,
            })),
        )
        .await
    }

    pub async fn profile_list(&self) -> Result<Value> {
        self.call("profile.list", None).await
    }

    pub async fn profile_delete(&self, id: &str) -> Result<Value> {
        self.call(
            "profile.delete",
            Some(serde_json::json!({
                "profile_id": id,
            })),
        )
        .await
    }

    pub async fn profile_update(&self, profile: Value) -> Result<Value> {
        self.call(
            "profile.update",
            Some(serde_json::json!({
                "profile": profile,
            })),
        )
        .await
    }

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

    pub async fn task_list(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "task.list",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    pub async fn task_get(&self, profile_id: &str, task_id: &str) -> Result<Value> {
        self.call(
            "task.get",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "task_id": task_id,
            })),
        )
        .await
    }

    pub async fn task_update(&self, profile_id: &str, task: Value) -> Result<Value> {
        self.call(
            "task.update",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "task": task,
            })),
        )
        .await
    }

    pub async fn task_delete(&self, profile_id: &str, task_id: &str) -> Result<Value> {
        self.call(
            "task.delete",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "task_id": task_id,
            })),
        )
        .await
    }

    pub async fn task_move(
        &self,
        source_profile_id: &str,
        target_profile_id: &str,
        task_id: &str,
        move_entries: Option<bool>,
    ) -> Result<Value> {
        self.call(
            "task.move",
            Some(serde_json::json!({
                "source_profile_id": source_profile_id,
                "target_profile_id": target_profile_id,
                "task_id": task_id,
                "move_entries": move_entries.unwrap_or(true),
            })),
        )
        .await
    }

    pub async fn entry_list(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "entry.list",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    pub async fn entry_filter(
        &self,
        profile_id: &str,
        start_date: Option<String>,
        end_date: Option<String>,
        task_id: Option<&str>,
        tags: Option<Vec<String>>,
    ) -> Result<Value> {
        self.call(
            "entry.filter",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "start_date": start_date,
                "end_date": end_date,
                "task_id": task_id,
                "tags": tags,
            })),
        )
        .await
    }

    pub async fn entry_today(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "entry.today",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    pub async fn entry_stats_today(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "entry.stats_today",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    pub async fn entry_delete(&self, profile_id: &str, entry_id: &str) -> Result<Value> {
        self.call(
            "entry.delete",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "entry_id": entry_id,
            })),
        )
        .await
    }

    pub async fn entry_update(&self, profile_id: &str, entry: Value) -> Result<Value> {
        self.call(
            "entry.update",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "entry": entry,
            })),
        )
        .await
    }

    pub async fn entry_create(
        &self,
        profile_id: &str,
        start_time: &str,
        end_time: &str,
        task_id: Option<&str>,
        description: Option<&str>,
    ) -> Result<Value> {
        self.call(
            "entry.create",
            Some(serde_json::json!({
                "profile_id": profile_id,
                "start_time": start_time,
                "end_time": end_time,
                "task_id": task_id,
                "description": description,
            })),
        )
        .await
    }

    pub async fn sync_status(&self) -> Result<Value> {
        self.call("sync.status", None).await
    }

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
