//! API handlers

pub mod config;
pub mod entry;
pub mod profile;
pub mod sync;
pub mod task;
pub mod timer;

use serde_json::{Value, json};
use std::sync::Arc;
use tokio::sync::broadcast;

use crate::config::ConfigManager;
use crate::entry::EntryManager;
use crate::event_manager::EventManager;
use crate::events::DaemonEvent;
use crate::profile::ProfileManager;
use crate::sync::SyncManager;
use crate::task::TaskManager;
use crate::timer::TimerManager;

/// API error
#[derive(Debug, thiserror::Error)]
pub enum ApiError {
    #[error("Method not found: {0}")]
    MethodNotFound(String),

    #[error("Invalid params: {0}")]
    InvalidParams(String),

    #[error("Timer error: {0}")]
    Timer(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

pub type Result<T> = std::result::Result<T, ApiError>;

/// Main API handler that routes requests to appropriate handlers
pub struct ApiHandler {
    event_manager: Arc<EventManager>,
    timer_manager: Arc<TimerManager>,
    profile_manager: Arc<ProfileManager>,
    task_manager: Arc<TaskManager>,
    entry_manager: Arc<EntryManager>,
    config_manager: Arc<ConfigManager>,
    sync_manager: Arc<SyncManager>,
}

impl ApiHandler {
    pub fn new(
        event_manager: Arc<EventManager>,
        timer_manager: Arc<TimerManager>,
        profile_manager: Arc<ProfileManager>,
        task_manager: Arc<TaskManager>,
        entry_manager: Arc<EntryManager>,
        config_manager: Arc<ConfigManager>,
        sync_manager: Arc<SyncManager>,
    ) -> Self {
        // Spawn task to save completed entries from auto-stopped timers
        let tm = timer_manager.clone();
        let em = entry_manager.clone();
        let sm = sync_manager.clone();
        let cm = config_manager.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(std::time::Duration::from_millis(500));
            loop {
                interval.tick().await;
                let entries = tm.take_completed_entries().await;
                for (profile_id, entry) in entries {
                    tracing::info!("Saving auto-completed entry for profile {}", profile_id);
                    if let Err(e) = em.add(&profile_id, entry.clone()).await {
                        tracing::error!("Failed to save auto-completed entry: {}", e);
                        continue;
                    }

                    // Auto-commit if configured
                    let config = cm.get().await;
                    if config.sync.auto_commit {
                        if !sm.is_initialized().await {
                            let _ = sm.init_repo().await;
                        }

                        let task_info = entry
                            .task_id
                            .as_ref()
                            .map(|id| format!("task {}", id))
                            .unwrap_or_else(|| "no task".to_string());
                        let duration_mins = entry.duration_seconds / 60;
                        let commit_msg = format!(
                            "Add entry: {} - {}m ({})",
                            task_info,
                            duration_mins,
                            chrono::Local::now().format("%Y-%m-%d %H:%M")
                        );

                        if let Err(e) = sm.auto_commit(&commit_msg).await {
                            tracing::warn!("Failed to auto-commit: {}", e);
                        }

                        if config.sync.auto_push
                            && config.sync.remote_url.is_some()
                            && let Err(e) = sm.sync(&config.sync).await
                        {
                            tracing::warn!("Failed to auto-sync: {}", e);
                        }
                    }
                }
            }
        });

        Self {
            event_manager,
            timer_manager,
            profile_manager,
            task_manager,
            entry_manager,
            config_manager,
            sync_manager,
        }
    }

    pub async fn handle(&self, method: &str, params: Option<Value>) -> Result<Value> {
        match method {
            // Timer methods
            "timer.start_manual" => self.handle_timer_start_manual(params).await,
            "timer.start_pomodoro" => self.handle_timer_start_pomodoro(params).await,
            "timer.start_countdown" => self.handle_timer_start_countdown(params).await,
            "timer.pause" => self.handle_timer_pause(params).await,
            "timer.resume" => self.handle_timer_resume(params).await,
            "timer.stop" => self.handle_timer_stop(params).await,
            "timer.cancel" => self.handle_timer_cancel(params).await,
            "timer.get" => self.handle_timer_get(params).await,
            "timer.list" => self.handle_timer_list(params).await,

            // Profile methods
            "profile.create" => self.handle_profile_create(params).await,
            "profile.get" => self.handle_profile_get(params).await,
            "profile.list" => self.handle_profile_list(params).await,
            "profile.update" => self.handle_profile_update(params).await,
            "profile.delete" => self.handle_profile_delete(params).await,

            // Task methods
            "task.create" => self.handle_task_create(params).await,
            "task.get" => self.handle_task_get(params).await,
            "task.list" => self.handle_task_list(params).await,
            "task.update" => self.handle_task_update(params).await,
            "task.delete" => self.handle_task_delete(params).await,
            "task.search" => self.handle_task_search(params).await,

            // Entry methods
            "entry.list" => self.handle_entry_list(params).await,
            "entry.filter" => self.handle_entry_filter(params).await,
            "entry.delete" => self.handle_entry_delete(params).await,
            "entry.update" => self.handle_entry_update(params).await,
            "entry.today" => self.handle_entry_today(params).await,
            "entry.week" => self.handle_entry_week(params).await,
            "entry.month" => self.handle_entry_month(params).await,
            "entry.stats_today" => self.handle_entry_stats_today(params).await,
            "entry.stats_week" => self.handle_entry_stats_week(params).await,
            "entry.stats_month" => self.handle_entry_stats_month(params).await,
            "entry.today_all_profiles" => self.handle_entry_today_all_profiles(params).await,
            "entry.week_all_profiles" => self.handle_entry_week_all_profiles(params).await,
            "entry.month_all_profiles" => self.handle_entry_month_all_profiles(params).await,

            // Config methods
            "config.get" => self.handle_config_get(params).await,
            "config.set_default_profile" => self.handle_config_set_default_profile(params).await,
            "config.update_pomodoro" => self.handle_config_update_pomodoro(params).await,
            "config.update_sync" => self.handle_config_update_sync(params).await,
            "config.reset" => self.handle_config_reset(params).await,

            // Sync methods
            "sync.init" => self.handle_sync_init(params).await,
            "sync.status" => self.handle_sync_status(params).await,
            "sync.sync" => self.handle_sync_sync(params).await,
            "sync.commit" => self.handle_sync_commit(params).await,
            "sync.set_remote" => self.handle_sync_set_remote(params).await,

            // Unknown method
            _ => Err(ApiError::MethodNotFound(method.to_string())),
        }
    }

    pub fn subscribe_events(&self) -> broadcast::Receiver<DaemonEvent> {
        self.event_manager.subscribe()
    }

    // Timer API methods
    async fn handle_timer_start_manual(&self, params: Option<Value>) -> Result<Value> {
        timer::start_manual(&self.timer_manager, params).await
    }

    async fn handle_timer_start_pomodoro(&self, params: Option<Value>) -> Result<Value> {
        timer::start_pomodoro(&self.timer_manager, &self.config_manager, params).await
    }

    async fn handle_timer_start_countdown(&self, params: Option<Value>) -> Result<Value> {
        timer::start_countdown(&self.timer_manager, params).await
    }

    async fn handle_timer_pause(&self, params: Option<Value>) -> Result<Value> {
        timer::pause(&self.timer_manager, params).await
    }

    async fn handle_timer_resume(&self, params: Option<Value>) -> Result<Value> {
        timer::resume(&self.timer_manager, params).await
    }

    async fn handle_timer_stop(&self, params: Option<Value>) -> Result<Value> {
        timer::stop(
            &self.timer_manager,
            &self.entry_manager,
            &self.sync_manager,
            &self.config_manager,
            params,
        )
        .await
    }

    async fn handle_timer_cancel(&self, params: Option<Value>) -> Result<Value> {
        timer::cancel(&self.timer_manager, params).await
    }

    async fn handle_timer_get(&self, params: Option<Value>) -> Result<Value> {
        timer::get(&self.timer_manager, params).await
    }

    async fn handle_timer_list(&self, params: Option<Value>) -> Result<Value> {
        timer::list(&self.timer_manager, params).await
    }

    // Profile API methods
    async fn handle_profile_create(&self, params: Option<Value>) -> Result<Value> {
        profile::create(&self.profile_manager, params).await
    }

    async fn handle_profile_get(&self, params: Option<Value>) -> Result<Value> {
        profile::get(&self.profile_manager, params).await
    }

    async fn handle_profile_list(&self, params: Option<Value>) -> Result<Value> {
        profile::list(&self.profile_manager, params).await
    }

    async fn handle_profile_update(&self, params: Option<Value>) -> Result<Value> {
        profile::update(&self.profile_manager, params).await
    }

    async fn handle_profile_delete(&self, params: Option<Value>) -> Result<Value> {
        profile::delete(&self.profile_manager, params).await
    }

    // Task API methods
    async fn handle_task_create(&self, params: Option<Value>) -> Result<Value> {
        task::create(&self.task_manager, params).await
    }

    async fn handle_task_get(&self, params: Option<Value>) -> Result<Value> {
        task::get(&self.task_manager, params).await
    }

    async fn handle_task_list(&self, params: Option<Value>) -> Result<Value> {
        task::list(&self.task_manager, params).await
    }

    async fn handle_task_update(&self, params: Option<Value>) -> Result<Value> {
        task::update(&self.task_manager, params).await
    }

    async fn handle_task_delete(&self, params: Option<Value>) -> Result<Value> {
        task::delete(&self.task_manager, params).await
    }

    async fn handle_task_search(&self, params: Option<Value>) -> Result<Value> {
        task::search(&self.task_manager, params).await
    }

    // Entry API methods
    async fn handle_entry_list(&self, params: Option<Value>) -> Result<Value> {
        entry::list(&self.entry_manager, params).await
    }

    async fn handle_entry_filter(&self, params: Option<Value>) -> Result<Value> {
        entry::filter(&self.entry_manager, params).await
    }

    async fn handle_entry_delete(&self, params: Option<Value>) -> Result<Value> {
        entry::delete(&self.entry_manager, params).await
    }

    async fn handle_entry_update(&self, params: Option<Value>) -> Result<Value> {
        entry::update(&self.entry_manager, params).await
    }

    async fn handle_entry_today(&self, params: Option<Value>) -> Result<Value> {
        entry::get_today(&self.entry_manager, params).await
    }

    async fn handle_entry_week(&self, params: Option<Value>) -> Result<Value> {
        entry::get_week(&self.entry_manager, params).await
    }

    async fn handle_entry_month(&self, params: Option<Value>) -> Result<Value> {
        entry::get_month(&self.entry_manager, params).await
    }

    async fn handle_entry_stats_today(&self, params: Option<Value>) -> Result<Value> {
        entry::stats_today(&self.entry_manager, params).await
    }

    async fn handle_entry_stats_week(&self, params: Option<Value>) -> Result<Value> {
        entry::stats_week(&self.entry_manager, params).await
    }

    async fn handle_entry_stats_month(&self, params: Option<Value>) -> Result<Value> {
        entry::stats_month(&self.entry_manager, params).await
    }

    async fn handle_entry_today_all_profiles(&self, params: Option<Value>) -> Result<Value> {
        entry::get_today_all_profiles(&self.entry_manager, &self.profile_manager, params).await
    }

    async fn handle_entry_week_all_profiles(&self, params: Option<Value>) -> Result<Value> {
        entry::get_week_all_profiles(&self.entry_manager, &self.profile_manager, params).await
    }

    async fn handle_entry_month_all_profiles(&self, params: Option<Value>) -> Result<Value> {
        entry::get_month_all_profiles(&self.entry_manager, &self.profile_manager, params).await
    }

    // Config API methods
    async fn handle_config_get(&self, params: Option<Value>) -> Result<Value> {
        config::get(&self.config_manager, params).await
    }

    async fn handle_config_set_default_profile(&self, params: Option<Value>) -> Result<Value> {
        config::set_default_profile(&self.config_manager, params).await
    }

    async fn handle_config_update_pomodoro(&self, params: Option<Value>) -> Result<Value> {
        config::update_pomodoro(&self.config_manager, params).await
    }

    async fn handle_config_update_sync(&self, params: Option<Value>) -> Result<Value> {
        config::update_sync(&self.config_manager, params).await
    }

    async fn handle_config_reset(&self, params: Option<Value>) -> Result<Value> {
        config::reset(&self.config_manager, params).await
    }

    // Sync API methods
    async fn handle_sync_init(&self, params: Option<Value>) -> Result<Value> {
        sync::init(&self.sync_manager, params).await
    }

    async fn handle_sync_status(&self, params: Option<Value>) -> Result<Value> {
        sync::status(&self.sync_manager, &self.config_manager, params).await
    }

    async fn handle_sync_sync(&self, params: Option<Value>) -> Result<Value> {
        sync::sync(&self.sync_manager, &self.config_manager, params).await
    }

    async fn handle_sync_commit(&self, params: Option<Value>) -> Result<Value> {
        sync::commit(&self.sync_manager, params).await
    }

    async fn handle_sync_set_remote(&self, params: Option<Value>) -> Result<Value> {
        sync::set_remote(&self.sync_manager, params).await?;
        Ok(Value::Null)
    }

    // --- Direct API Access Methods for MCP ---

    pub async fn profile_list(&self) -> Result<Value> {
        profile::list(&self.profile_manager, None).await
    }

    pub async fn task_list(&self, profile_id: &str) -> Result<Value> {
        task::list(
            &self.task_manager,
            Some(json!({ "profile_id": profile_id })),
        )
        .await
    }

    pub async fn task_create(
        &self,
        profile_id: &str,
        title: &str,
        description: Option<&str>,
    ) -> Result<Value> {
        task::create(
            &self.task_manager,
            Some(json!({ "profile_id": profile_id, "title": title, "description": description })),
        )
        .await
    }

    pub async fn task_update(&self, profile_id: &str, task: Value) -> Result<Value> {
        task::update(
            &self.task_manager,
            Some(json!({ "profile_id": profile_id, "task": task })),
        )
        .await
    }

    pub async fn task_delete(&self, profile_id: &str, task_id: &str) -> Result<Value> {
        task::delete(
            &self.task_manager,
            Some(json!({ "profile_id": profile_id, "task_id": task_id })),
        )
        .await
    }

    pub async fn timer_get(&self, profile_id: &str) -> Result<Value> {
        timer::get(
            &self.timer_manager,
            Some(json!({ "profile_id": profile_id })),
        )
        .await
    }

    pub async fn timer_start_manual(
        &self,
        profile_id: &str,
        task_id: Option<&str>,
    ) -> Result<Value> {
        timer::start_manual(
            &self.timer_manager,
            Some(json!({ "profile_id": profile_id, "task_id": task_id })),
        )
        .await
    }

    pub async fn timer_start_pomodoro(
        &self,
        profile_id: &str,
        task_id: Option<&str>,
        config_override: Option<Value>,
    ) -> Result<Value> {
        let mut params = json!({ "profile_id": profile_id, "task_id": task_id });
        if let Some(config) = config_override {
            params["config"] = config;
        }
        timer::start_pomodoro(&self.timer_manager, &self.config_manager, Some(params)).await
    }

    pub async fn timer_start_countdown(
        &self,
        profile_id: &str,
        task_id: Option<&str>,
        duration_minutes: u64,
    ) -> Result<Value> {
        timer::start_countdown(&self.timer_manager, Some(json!({ "profile_id": profile_id, "task_id": task_id, "duration_minutes": duration_minutes }))).await
    }

    pub async fn timer_stop(&self, profile_id: &str) -> Result<Value> {
        timer::stop(
            &self.timer_manager,
            &self.entry_manager,
            &self.sync_manager,
            &self.config_manager,
            Some(json!({ "profile_id": profile_id })),
        )
        .await
    }

    pub async fn task_get(&self, profile_id: &str, task_id: &str) -> Result<Value> {
        task::get(
            &self.task_manager,
            Some(json!({ "profile_id": profile_id, "task_id": task_id })),
        )
        .await
    }
}
