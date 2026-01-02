//! IPC module for daemon communication

pub mod connection;
pub mod protocol;
pub mod server;

pub use connection::Connection;
pub use protocol::{JsonRpcError, Notification, Request, Response};
pub use server::IpcServer;
