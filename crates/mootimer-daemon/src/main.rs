//! MooTimer Daemon
//!
//! The main daemon process that manages timers, profiles, and data storage.

use anyhow::Result;
use clap::Parser;
use mootimer_core::storage::init_data_dir;
use mootimer_daemon::{
    ApiHandler, ConfigManager, EntryManager, IpcServer, ProfileManager, SyncManager, TaskManager,
    TimerManager,
};
use std::fs;
use std::sync::Arc;

#[derive(Parser, Debug)]
#[command(name = "mootimerd")]
#[command(about = "MooTimer daemon - work timer backend", long_about = None)]
struct Args {
    /// Socket path for IPC
    #[arg(short, long, default_value = "/tmp/mootimer.sock")]
    socket: String,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Initialize data directory and log file
    let data_dir = init_data_dir()?;
    let log_file_path = data_dir.join("daemon.log");

    // Create log file with append mode
    let log_file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&log_file_path)?;

    // Initialize logging - write to both file and stdout
    use tracing_subscriber::fmt::writer::MakeWriterExt;
    let stdout_writer = std::io::stdout.with_max_level(tracing::Level::INFO);
    let file_writer = log_file.with_max_level(tracing::Level::DEBUG);

    tracing_subscriber::fmt()
        .with_writer(stdout_writer.and(file_writer))
        .with_env_filter(&args.log_level)
        .with_ansi(false) // No color codes in log file
        .init();

    tracing::info!("MooTimer daemon starting...");
    tracing::info!("Socket path: {}", args.socket);
    tracing::info!("Log file: {}", log_file_path.display());

    // Initialize managers
    let timer_manager = TimerManager::new();
    tracing::info!("Timer manager initialized");

    let profile_manager = ProfileManager::new()?;
    profile_manager.load_all().await?;
    tracing::info!("Profile manager initialized");

    let task_manager = TaskManager::new()?;
    tracing::info!("Task manager initialized");

    let entry_manager = EntryManager::new()?;
    tracing::info!("Entry manager initialized");

    let config_manager = ConfigManager::new()?;
    tracing::info!("Config manager initialized");

    let sync_manager = SyncManager::new()?;
    tracing::info!("Sync manager initialized");

    // Initialize API handler
    let api_handler = Arc::new(ApiHandler::new(
        timer_manager,
        profile_manager,
        task_manager,
        entry_manager,
        config_manager,
        sync_manager,
    ));
    tracing::info!("API handler initialized");

    // Initialize and start IPC server
    let ipc_server = Arc::new(IpcServer::new(args.socket, api_handler));
    tracing::info!("IPC server initialized");

    // Start IPC server in background
    let server_handle = {
        let server = ipc_server.clone();
        tokio::spawn(async move {
            if let Err(e) = server.start().await {
                tracing::error!("IPC server error: {}", e);
            }
        })
    };

    tracing::info!("Daemon ready and listening");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("Shutting down...");

    // Abort server task
    server_handle.abort();

    Ok(())
}
