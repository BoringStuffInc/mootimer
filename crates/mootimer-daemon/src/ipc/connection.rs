use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader, BufWriter};
use tokio::net::UnixStream;
use tokio::sync::mpsc;

use super::protocol::{Notification, Request, Response};

#[derive(Debug, thiserror::Error)]
pub enum ConnectionError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("Connection closed")]
    Closed,
}

pub type Result<T> = std::result::Result<T, ConnectionError>;

pub struct Connection {
    reader: BufReader<tokio::io::ReadHalf<UnixStream>>,
    writer: BufWriter<tokio::io::WriteHalf<UnixStream>>,
    notification_rx: mpsc::Receiver<Notification>,
}

impl Connection {
    pub fn new(stream: UnixStream) -> (Self, mpsc::Sender<Notification>) {
        let (notif_tx, notif_rx) = mpsc::channel(100);
        let (read_half, write_half) = tokio::io::split(stream);

        (
            Self {
                reader: BufReader::new(read_half),
                writer: BufWriter::new(write_half),
                notification_rx: notif_rx,
            },
            notif_tx,
        )
    }

    pub async fn read_request(&mut self) -> Result<Request> {
        let mut line = String::new();
        let bytes_read = self.reader.read_line(&mut line).await?;

        if bytes_read == 0 || line.trim().is_empty() {
            return Err(ConnectionError::Closed);
        }

        let request: Request = serde_json::from_str(line.trim())?;
        Ok(request)
    }

    pub async fn write_response(&mut self, response: &Response) -> Result<()> {
        let json = serde_json::to_string(response)?;

        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;

        Ok(())
    }

    pub async fn write_notification(&mut self, notification: &Notification) -> Result<()> {
        let json = serde_json::to_string(notification)?;

        self.writer.write_all(json.as_bytes()).await?;
        self.writer.write_all(b"\n").await?;
        self.writer.flush().await?;

        Ok(())
    }

    pub async fn start_notification_loop(mut self) -> Result<()> {
        while let Some(notification) = self.notification_rx.recv().await {
            if let Err(e) = self.write_notification(&notification).await {
                tracing::warn!("Failed to send notification: {}", e);
                break;
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use tokio::net::{UnixListener, UnixStream};

    #[tokio::test]
    async fn test_connection_creation() {
        let temp_dir = TempDir::new().unwrap();
        let socket_path = temp_dir.path().join("test.sock");

        let listener = UnixListener::bind(&socket_path).unwrap();

        tokio::spawn(async move {
            let _stream = UnixStream::connect(socket_path).await.unwrap();
        });

        let (stream, _) = listener.accept().await.unwrap();
        let (_conn, _tx) = Connection::new(stream);
    }
}
