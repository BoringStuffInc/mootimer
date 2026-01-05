use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::UnixStream;
use tokio::sync::{Mutex, RwLock, mpsc};

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
    writer: Arc<Mutex<BufWriter<tokio::io::WriteHalf<UnixStream>>>>,
    pending_responses: Arc<RwLock<HashMap<i64, mpsc::Sender<Response>>>>,
}

pub struct MooTimerClient {
    socket_path: String,
    request_counter: std::sync::atomic::AtomicI64,
    persistent_conn: Arc<Mutex<Option<PersistentConnection>>>,
}

impl MooTimerClient {
    pub fn new(socket_path: impl Into<String>) -> Self {
        Self {
            socket_path: socket_path.into(),
            request_counter: std::sync::atomic::AtomicI64::new(1),
            persistent_conn: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn subscribe_notifications(&self) -> Result<mpsc::Receiver<Notification>> {
        let mut conn_lock = self.persistent_conn.lock().await;

        if conn_lock.is_some() {
            anyhow::bail!("Already subscribed to notifications");
        }

        let stream = self.connect().await?;
        let (read_half, write_half) = tokio::io::split(stream);
        let writer = Arc::new(Mutex::new(BufWriter::new(write_half)));

        let (notif_tx, notif_rx) = mpsc::channel::<Notification>(100);
        let pending_responses: Arc<RwLock<HashMap<i64, mpsc::Sender<Response>>>> =
            Arc::new(RwLock::new(HashMap::new()));

        let pending_clone = pending_responses.clone();
        let notif_tx_clone = notif_tx.clone();
        tokio::spawn(async move {
            let mut reader = BufReader::new(read_half);
            let mut line = String::new();

            loop {
                line.clear();
                match reader.read_line(&mut line).await {
                    Ok(0) | Err(_) => break,
                    Ok(_) => {
                        if let Ok(notification) = serde_json::from_str::<Notification>(&line) {
                            let _ = notif_tx_clone.send(notification).await;
                        } else if let Ok(response) = serde_json::from_str::<Response>(&line)
                            && let RequestId::Number(id) = response.id
                        {
                            let pending = pending_clone.read().await;
                            if let Some(tx) = pending.get(&id) {
                                let _ = tx.send(response).await;
                            }
                        }
                    }
                }
            }
        });

        *conn_lock = Some(PersistentConnection {
            writer,
            pending_responses,
        });

        Ok(notif_rx)
    }

    async fn call_persistent(
        &self,
        method: impl Into<String>,
        params: Option<Value>,
    ) -> Result<Value> {
        let method_str = method.into();
        let conn_lock = self.persistent_conn.lock().await;

        if let Some(conn) = conn_lock.as_ref() {
            let _ = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/mootimer-client.log")
                .and_then(|mut f| {
                    use std::io::Write;
                    writeln!(f, "✅ PERSISTENT: {}", method_str)
                });
            let method = method_str;
            let request = Request::new(method, params, self.next_id());
            let request_id = if let RequestId::Number(id) = request.id.clone() {
                id
            } else {
                anyhow::bail!("Invalid request ID");
            };

            let (tx, mut rx) = mpsc::channel::<Response>(1);

            {
                let mut pending = conn.pending_responses.write().await;
                pending.insert(request_id, tx);
            }

            {
                let mut writer = conn.writer.lock().await;
                let request_json = serde_json::to_string(&request)?;
                writer.write_all(request_json.as_bytes()).await?;
                writer.write_all(b"\n").await?;
                writer.flush().await?;
            }

            let response = rx
                .recv()
                .await
                .ok_or_else(|| anyhow::anyhow!("No response received"))?;

            {
                let mut pending = conn.pending_responses.write().await;
                pending.remove(&request_id);
            }

            if let Some(error) = response.error {
                anyhow::bail!("RPC error {}: {}", error.code, error.message);
            }

            return Ok(response.result.unwrap_or(Value::Null));
        }

        let _ = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open("/tmp/mootimer-client.log")
            .and_then(|mut f| {
                use std::io::Write;
                writeln!(f, "❌ ONE-SHOT: {}", method_str)
            });
        drop(conn_lock);
        self.call_oneshot(method_str, params).await
    }

    async fn call_oneshot(
        &self,
        method: impl Into<String>,
        params: Option<Value>,
    ) -> Result<Value> {
        let mut stream = self.connect().await?;
        let request = Request::new(method, params, self.next_id());

        let request_json = serde_json::to_string(&request)?;
        stream.write_all(request_json.as_bytes()).await?;
        stream.write_all(b"\n").await?;
        stream.flush().await?;

        let mut reader = BufReader::new(stream);
        let mut line = String::new();
        reader.read_line(&mut line).await?;

        let response: Response = serde_json::from_str(&line)?;

        if let Some(error) = response.error {
            anyhow::bail!("RPC error {}: {}", error.code, error.message);
        }

        Ok(response.result.unwrap_or(Value::Null))
    }

    async fn connect(&self) -> Result<UnixStream> {
        Ok(UnixStream::connect(&self.socket_path).await?)
    }

    fn next_id(&self) -> RequestId {
        let id = self
            .request_counter
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        RequestId::Number(id)
    }

    pub async fn call(&self, method: impl Into<String>, params: Option<Value>) -> Result<Value> {
        self.call_persistent(method, params).await
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

    pub async fn timer_pause(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.pause",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    pub async fn timer_resume(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.resume",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    pub async fn timer_stop(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.stop",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    pub async fn timer_cancel(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.cancel",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

    pub async fn timer_get(&self, profile_id: &str) -> Result<Value> {
        self.call(
            "timer.get",
            Some(serde_json::json!({
                "profile_id": profile_id,
            })),
        )
        .await
    }

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
