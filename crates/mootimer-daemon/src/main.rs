//! MooTimer Daemon
//!
//! The main daemon process that manages timers, profiles, and data storage.

use anyhow::Result;
use clap::Parser;
use mootimer_core::storage::init_data_dir;
use mootimer_daemon::{
    ApiHandler, ConfigManager, EntryManager, EventManager, IpcServer, ProfileManager, SyncManager,
    TaskManager, TimerManager,
};
use std::fs;
use std::sync::Arc;

mod mcp;

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

    /// Run in Model Context Protocol (MCP) server mode
    #[arg(long)]
    mcp: bool,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    if args.mcp {
        // Run as MCP server (connects to existing daemon)
        mcp::run_mcp_server(args.socket).await
    } else {
        // Run as standard daemon
        // Initialize event manager
        let event_manager = Arc::new(EventManager::new());

        // Initialize managers
        let timer_manager = Arc::new(TimerManager::new(event_manager.clone()));
        let profile_manager = Arc::new(ProfileManager::new(event_manager.clone())?);
        profile_manager.load_all().await?; // load_all needs access to the manager so it needs to be Arc'd first
        let task_manager = Arc::new(TaskManager::new(event_manager.clone())?);
        let entry_manager = Arc::new(EntryManager::new(event_manager.clone())?);
        let config_manager = Arc::new(ConfigManager::new()?);
        let sync_manager = Arc::new(SyncManager::new()?);

        // Initialize API handler
        let api_handler = Arc::new(ApiHandler::new(
            event_manager,
            timer_manager,
            profile_manager,
            task_manager,
            entry_manager,
            config_manager,
            sync_manager,
        ));

        run_daemon(args, api_handler).await
    }
}

async fn run_daemon(args: Args, api_handler: Arc<ApiHandler>) -> Result<()> {
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

    tracing::info!("Timer manager initialized");
    tracing::info!("Profile manager initialized");
    tracing::info!("Task manager initialized");
    tracing::info!("Entry manager initialized");
    tracing::info!("Config manager initialized");
    tracing::info!("Sync manager initialized");
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
