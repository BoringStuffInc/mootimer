use std::path::Path;
use std::sync::Arc;
use tokio::net::{UnixListener, UnixStream};
use tokio::sync::mpsc;

use super::connection::ConnectionError;
use super::protocol::{JsonRpcError, Notification, Request, Response};
use crate::api::ApiHandler;

#[derive(Debug, thiserror::Error)]
pub enum IpcServerError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Connection error: {0}")]
    Connection(#[from] super::connection::ConnectionError),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Server already running")]
    AlreadyRunning,
}

pub type Result<T> = std::result::Result<T, IpcServerError>;

pub struct IpcServer {
    socket_path: String,
    api_handler: Arc<ApiHandler>,
}

impl IpcServer {
    pub fn new(socket_path: String, api_handler: Arc<ApiHandler>) -> Self {
        Self {
            socket_path,
            api_handler,
        }
    }

    pub async fn start(self: Arc<Self>) -> Result<()> {
        let path = Path::new(&self.socket_path);
        if path.exists() {
            std::fs::remove_file(path)?;
        }

        let listener = UnixListener::bind(&self.socket_path)?;
        tracing::info!("IPC server listening on {}", self.socket_path);

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let server = self.clone();
                    tokio::spawn(async move {
                        if let Err(e) = server.handle_connection(stream).await {
                            tracing::error!("Connection error: {}", e);
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Failed to accept connection: {}", e);
                }
            }
        }
    }

    async fn handle_connection(&self, stream: UnixStream) -> Result<()> {
        tracing::debug!("New client connected");

        let (read_half, write_half) = tokio::io::split(stream);
        let mut reader = tokio::io::BufReader::new(read_half);
        let mut writer = tokio::io::BufWriter::new(write_half);

        let mut event_rx = self.api_handler.subscribe_events();

        let (notif_tx, mut notif_rx) = mpsc::channel::<Notification>(100);
        tokio::spawn(async move {
            tracing::info!("IPC: Event forwarder task started");
            while let Ok(event) = event_rx.recv().await {
                use crate::events::DaemonEvent;
                let (method, params) = match &event {
                    DaemonEvent::Timer(e) => {
                        tracing::debug!("IPC: Forwarding timer event");
                        ("timer.event", serde_json::to_value(e))
                    }
                    DaemonEvent::Task(e) => {
                        tracing::info!("IPC: Forwarding task event to client");
                        ("task.event", serde_json::to_value(e))
                    }
                    DaemonEvent::Entry(e) => {
                        tracing::debug!("IPC: Forwarding entry event");
                        ("entry.event", serde_json::to_value(e))
                    }
                    DaemonEvent::Profile(e) => {
                        tracing::debug!("IPC: Forwarding profile event");
                        ("profile.event", serde_json::to_value(e))
                    }
                };

                let notification = Notification {
                    jsonrpc: "2.0".to_string(),
                    method: method.to_string(),
                    params: params.unwrap_or(serde_json::Value::Null),
                };

                if notif_tx.send(notification).await.is_err() {
                    tracing::info!("IPC: Event forwarder stopping - client disconnected");
                    break;
                }
            }
            tracing::info!("IPC: Event forwarder task stopped");
        });

        loop {
            tokio::select! {
                result = Self::read_request_from(&mut reader) => {
                    match result {
                        Ok(request) => {
                            tracing::info!("handler: received request: {}", request.method);
                            let response = self.handle_request(request).await;
                            if let Err(e) = Self::write_response_to(&mut writer, &response).await {
                                tracing::error!("Failed to write response: {}", e);
                                break;
                            }
                        }
                        Err(IpcServerError::Connection(ConnectionError::Closed)) => {
                            tracing::debug!("Client disconnected");
                            break;
                        }
                        Err(e) => {
                            tracing::error!("Failed to read request: {}", e);
                            break;
                        }
                    }
                }
                Some(notification) = notif_rx.recv() => {
                    tracing::info!("IPC: Sending notification to client: {}", notification.method);
                    if let Err(e) = Self::write_notification_to(&mut writer, &notification).await {
                        tracing::warn!("Failed to send notification: {}", e);
                        break;
                    }
                    tracing::info!("IPC: Notification sent successfully");
                }
            }
        }

        Ok(())
    }

    async fn read_request_from(
        reader: &mut tokio::io::BufReader<tokio::io::ReadHalf<UnixStream>>,
    ) -> Result<Request> {
        use tokio::io::AsyncBufReadExt;

        let mut line = String::new();
        let bytes_read = reader.read_line(&mut line).await?;

        if bytes_read == 0 || line.trim().is_empty() {
            return Err(IpcServerError::Connection(ConnectionError::Closed));
        }

        let request: Request = serde_json::from_str(line.trim())?;
        Ok(request)
    }

    async fn write_response_to(
        writer: &mut tokio::io::BufWriter<tokio::io::WriteHalf<UnixStream>>,
        response: &Response,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let json = serde_json::to_string(response)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }

    async fn write_notification_to(
        writer: &mut tokio::io::BufWriter<tokio::io::WriteHalf<UnixStream>>,
        notification: &Notification,
    ) -> Result<()> {
        use tokio::io::AsyncWriteExt;

        let json = serde_json::to_string(notification)?;
        writer.write_all(json.as_bytes()).await?;
        writer.write_all(b"\n").await?;
        writer.flush().await?;
        Ok(())
    }

    async fn handle_request(&self, request: Request) -> Response {
        if let Err(error) = request.validate() {
            return Response::error(error, request.id);
        }

        match self
            .api_handler
            .handle(&request.method, request.params)
            .await
        {
            Ok(result) => Response::success(result, request.id),
            Err(error) => {
                let json_rpc_error = JsonRpcError::application_error(-32000, error.to_string());
                Response::error(json_rpc_error, request.id)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_manager::EventManager;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_server_creation() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir
            .path()
            .join("test.sock")
            .to_str()
            .unwrap()
            .to_string();

        let event_manager = Arc::new(EventManager::new());
        let timer_manager = Arc::new(crate::timer::TimerManager::new(event_manager.clone()));
        let profile_manager =
            Arc::new(crate::profile::ProfileManager::new(event_manager.clone()).unwrap());
        let task_manager = Arc::new(crate::task::TaskManager::new(event_manager.clone()).unwrap());
        let entry_manager =
            Arc::new(crate::entry::EntryManager::new(event_manager.clone()).unwrap());
        let config_manager = Arc::new(crate::config::ConfigManager::new().unwrap());
        let sync_manager = Arc::new(crate::sync::SyncManager::new().unwrap());

        let api_handler = Arc::new(ApiHandler::new(
            event_manager,
            timer_manager,
            profile_manager,
            task_manager,
            entry_manager,
            config_manager,
            sync_manager,
        ));

        let _server = IpcServer::new(socket_path, api_handler);
    }
}
