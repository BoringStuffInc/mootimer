use crate::ui::cow::CowState;
use crate::ui::tomato::TomatoState;
use anyhow::Result;
use mootimer_client::MooTimerClient;
use serde_json::Value;
use std::collections::HashMap;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppView {
    Dashboard,
    Kanban,
    Entries,
    Reports,
    Settings,
    Logs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashboardPane {
    TimerConfig,
    TasksList,
    ProfileList,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    NewTask,
    QuickAddTask,
    EditTask,
    SearchTasks,
    DeleteTaskConfirm,
    FilterEntries,
    ConfigPomodoro,
    ConfigShortBreak,
    ConfigLongBreak,
    ConfigCountdown,
    NewProfile,
    RenameProfile,
    DeleteProfileConfirm,
    EditEntryDuration,
    ConfirmQuit,
    PomodoroBreakFinished,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SettingsItem {
    PomodoroWork,
    PomodoroShortBreak,
    PomodoroLongBreak,
    CountdownDefault,
    AudioAlerts,
    CowModal,
    SyncAutoCommit,
    SyncInitRepo,
    SyncNow,
}

impl SettingsItem {
    pub const ALL: [Self; 9] = [
        Self::PomodoroWork,
        Self::PomodoroShortBreak,
        Self::PomodoroLongBreak,
        Self::CountdownDefault,
        Self::AudioAlerts,
        Self::CowModal,
        Self::SyncAutoCommit,
        Self::SyncInitRepo,
        Self::SyncNow,
    ];
}

pub struct App {
    pub client: MooTimerClient,
    pub profile_id: String,
    pub current_view: AppView,
    pub focused_pane: DashboardPane,
    pub show_help: bool,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub input_buffer_2: String,
    pub focused_input_field: usize,
    pub temp_task_title: Option<String>,

    pub entry_filter: String,
    pub task_search: String,
    pub show_archived: bool,
    pub selected_setting_index: usize,

    pub timer_info: Option<Value>,
    pub stats_today: Option<Value>,
    pub tasks: Vec<Value>,
    pub entries: Vec<Value>,
    pub report_entries: Vec<Value>,
    pub report_stats: Option<Value>,
    pub sync_status: Option<Value>,
    pub config: Option<Value>,
    pub log_lines: Vec<String>,
    pub profiles: Vec<Value>,

    cross_profile_cache: HashMap<String, (Vec<Value>, Instant)>,

    pub selected_task_index: usize,
    pub selected_entry_index: usize,
    pub selected_log_index: usize,
    pub selected_profile_index: usize,
    pub selected_column_index: usize,
    pub selected_kanban_card_index: usize,
    pub report_period: String,
    pub report_profile: String,
    pub selected_timer_type: TimerType,
    pub pomodoro_minutes: u64,
    pub countdown_minutes: u64,
    pub should_quit: bool,
    pub status_message: String,
    pub five_min_warning_shown: bool,
    pub audio_alerts_enabled: bool,
    pub cow_modal_enabled: bool,
    pub show_cow_modal: bool,
    pub show_task_description: bool,
    pub tomato_state: TomatoState,
    pub cow_state: CowState,
    pub selected_timer_button: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TimerType {
    Manual,
    Pomodoro,
    Countdown,
}

impl App {
    pub fn new(client: MooTimerClient, profile_id: String) -> Self {
        let report_profile = profile_id.clone();
        Self {
            client,
            profile_id,
            current_view: AppView::Dashboard,
            focused_pane: DashboardPane::TimerConfig,
            show_help: false,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            input_buffer_2: String::new(),
            focused_input_field: 0,
            temp_task_title: None,
            entry_filter: String::new(),
            task_search: String::new(),
            show_archived: false,
            selected_setting_index: 0,

            timer_info: None,
            stats_today: None,
            tasks: Vec::new(),
            entries: Vec::new(),
            report_entries: Vec::new(),
            report_stats: None,
            sync_status: None,
            config: None,
            log_lines: Vec::new(),
            profiles: Vec::new(),
            cross_profile_cache: HashMap::new(),

            selected_task_index: 0,
            selected_entry_index: 0,
            selected_log_index: 0,
            selected_profile_index: 0,
            selected_column_index: 0,
            selected_kanban_card_index: 0,
            report_period: "day".to_string(),
            report_profile,
            selected_timer_type: TimerType::Manual,
            pomodoro_minutes: 25,
            countdown_minutes: 30,
            should_quit: false,
            status_message: String::new(),
            five_min_warning_shown: false,
            audio_alerts_enabled: true,
            cow_modal_enabled: true,
            show_cow_modal: false,
            show_task_description: false,
            tomato_state: TomatoState::new(),
            cow_state: CowState::new(),
            selected_timer_button: 0,
        }
    }

    pub fn handle_input_char(&mut self, c: char) {
        if self.focused_input_field == 0 {
            self.input_buffer.push(c);
        } else {
            self.input_buffer_2.push(c);
        }
    }

    pub fn handle_input_backspace(&mut self) {
        if self.focused_input_field == 0 {
            self.input_buffer.pop();
        } else {
            self.input_buffer_2.pop();
        }
    }

    pub fn get_profile_name(&self) -> &str {
        self.get_profile_name_by_id(&self.profile_id)
    }

    pub fn get_profile_name_by_id<'a>(&'a self, profile_id: &'a str) -> &'a str {
        self.profiles
            .iter()
            .find(|p| p.get("id").and_then(|v| v.as_str()) == Some(profile_id))
            .and_then(|p| p.get("name"))
            .and_then(|v| v.as_str())
            .unwrap_or(profile_id)
    }

    pub fn get_filtered_tasks(&self) -> Vec<&Value> {
        let search = self.task_search.to_lowercase();
        self.tasks
            .iter()
            .filter(|task| {
                let status = task
                    .get("status")
                    .and_then(|v| v.as_str())
                    .unwrap_or("todo");
                let is_archived = status == "archived";

                if self.show_archived {
                    if !is_archived {
                        return false;
                    }
                } else if is_archived {
                    return false;
                }

                if !self.task_search.is_empty()
                    && let Some(title) = task.get("title").and_then(|v| v.as_str())
                    && !title.to_lowercase().contains(&search)
                {
                    return false;
                }
                true
            })
            .collect()
    }

    pub fn get_filtered_entries(&self) -> Vec<&Value> {
        if self.entry_filter.is_empty() {
            self.entries.iter().collect()
        } else {
            let filter = self.entry_filter.to_lowercase();
            self.entries
                .iter()
                .filter(|entry| {
                    if let Some(desc) = entry.get("description").and_then(|v| v.as_str())
                        && desc.to_lowercase().contains(&filter)
                    {
                        return true;
                    }

                    if let Some(tid) = entry.get("task_id").and_then(|v| v.as_str())
                        && let Some(task) = self
                            .tasks
                            .iter()
                            .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(tid))
                        && let Some(title) = task.get("title").and_then(|v| v.as_str())
                        && title.to_lowercase().contains(&filter)
                    {
                        return true;
                    }

                    if let Some(id) = entry.get("id").and_then(|v| v.as_str())
                        && id.to_lowercase().contains(&filter)
                    {
                        return true;
                    }

                    false
                })
                .collect()
        }
    }

    pub fn get_selected_kanban_task_id(&self) -> Option<String> {
        self.get_kanban_tasks(self.selected_column_index)
            .get(self.selected_kanban_card_index)
            .and_then(|t| t.get("id").and_then(|v| v.as_str()))
            .map(|s| s.to_string())
    }

    pub fn sync_kanban_to_task_index(&mut self, task_id: &str) {
        if let Some(idx) = self
            .tasks
            .iter()
            .position(|t| t.get("id").and_then(|v| v.as_str()) == Some(task_id))
        {
            self.selected_task_index = idx;
        }
    }

    pub fn get_kanban_tasks(&self, column_index: usize) -> Vec<&Value> {
        let search = self.task_search.to_lowercase();

        self.tasks
            .iter()
            .filter(|t| {
                let status = t.get("status").and_then(|v| v.as_str()).unwrap_or("todo");

                let matches_status = if self.show_archived {
                    column_index == 0 && status == "archived"
                } else {
                    if status == "archived" {
                        return false;
                    }
                    match column_index {
                        0 => status == "todo",
                        1 => status == "in_progress",
                        2 => status == "done" || status == "completed",
                        _ => false,
                    }
                };

                if !matches_status {
                    return false;
                }

                if !search.is_empty() {
                    if let Some(title) = t.get("title").and_then(|v| v.as_str()) {
                        if !title.to_lowercase().contains(&search) {
                            return false;
                        }
                    } else {
                        return false;
                    }
                }

                true
            })
            .collect()
    }

    pub async fn move_kanban_card(&mut self, direction: i32) -> Result<()> {
        let current_col = self.selected_column_index;
        let new_col = if direction > 0 {
            (current_col + 1).min(2)
        } else {
            current_col.saturating_sub(1)
        };

        if current_col == new_col {
            return Ok(());
        }

        let tasks = self.get_kanban_tasks(current_col);
        if let Some(task) = tasks.get(self.selected_kanban_card_index) {
            let mut task_clone = (*task).clone();

            let new_status = match new_col {
                0 => "todo",
                1 => "in_progress",
                2 => "done",
                _ => "todo",
            };

            if let Some(obj) = task_clone.as_object_mut() {
                obj.insert(
                    "status".to_string(),
                    serde_json::Value::String(new_status.to_string()),
                );

                self.client
                    .task_update(&self.profile_id, task_clone)
                    .await?;
                self.refresh_tasks().await?;

                self.selected_column_index = new_col;
                let new_len = self.get_kanban_tasks(new_col).len();
                self.selected_kanban_card_index = new_len.saturating_sub(1);
            }
        }
        Ok(())
    }

    pub fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub fn list_next(&mut self) {
        match self.current_view {
            AppView::Dashboard => {
                let len = self.get_filtered_tasks().len();
                if len > 0 && self.selected_task_index < len.saturating_sub(1) {
                    self.selected_task_index += 1;
                }
            }
            AppView::Entries => {
                let len = self.get_filtered_entries().len();
                if len > 0 && self.selected_entry_index < len.saturating_sub(1) {
                    self.selected_entry_index += 1;
                }
            }
            AppView::Kanban => {
                let len = self.get_kanban_tasks(self.selected_column_index).len();
                if len > 0 && self.selected_kanban_card_index < len.saturating_sub(1) {
                    self.selected_kanban_card_index += 1;
                }
            }
            _ => {}
        }
    }

    pub fn list_previous(&mut self) {
        match self.current_view {
            AppView::Dashboard => {
                if !self.get_filtered_tasks().is_empty() {
                    self.selected_task_index = self.selected_task_index.saturating_sub(1);
                }
            }
            AppView::Entries => {
                self.selected_entry_index = self.selected_entry_index.saturating_sub(1);
            }
            AppView::Kanban => {
                self.selected_kanban_card_index = self.selected_kanban_card_index.saturating_sub(1);
            }
            _ => {}
        }
    }

    pub fn list_page_up(&mut self) {
        match self.current_view {
            AppView::Dashboard => {
                self.selected_task_index = self.selected_task_index.saturating_sub(10);
            }
            AppView::Entries => {
                self.selected_entry_index = self.selected_entry_index.saturating_sub(10);
            }
            _ => {}
        }
    }

    pub fn list_page_down(&mut self) {
        match self.current_view {
            AppView::Dashboard => {
                let len = self.get_filtered_tasks().len();
                self.selected_task_index =
                    (self.selected_task_index + 10).min(len.saturating_sub(1));
            }
            AppView::Entries => {
                let len = self.get_filtered_entries().len();
                self.selected_entry_index =
                    (self.selected_entry_index + 10).min(len.saturating_sub(1));
            }
            _ => {}
        }
    }

    pub async fn refresh_all(&mut self) -> Result<()> {
        self.refresh_timer().await?;
        self.refresh_stats().await?;
        self.refresh_tasks().await?;
        self.refresh_entries().await?;
        self.refresh_sync().await?;
        self.refresh_config().await?;
        self.refresh_profiles().await?;
        Ok(())
    }

    pub async fn refresh_timer(&mut self) -> Result<()> {
        self.timer_info = self.client.timer_get(&self.profile_id).await.ok();
        Ok(())
    }

    pub async fn refresh_stats(&mut self) -> Result<()> {
        self.stats_today = self.client.entry_stats_today(&self.profile_id).await.ok();
        Ok(())
    }

    pub async fn refresh_tasks(&mut self) -> Result<()> {
        if let Ok(tasks) = self.client.task_list(&self.profile_id).await {
            self.tasks = tasks.as_array().cloned().unwrap_or_default();
            self.status_message = format!("Loaded {} tasks", self.tasks.len());
        }
        Ok(())
    }

    pub async fn refresh_entries(&mut self) -> Result<()> {
        if let Ok(entries) = self.client.entry_today(&self.profile_id).await {
            self.entries = entries.as_array().cloned().unwrap_or_default();
        }
        Ok(())
    }

    pub async fn refresh_sync(&mut self) -> Result<()> {
        self.sync_status = self.client.sync_status().await.ok();
        Ok(())
    }

    pub async fn refresh_config(&mut self) -> Result<()> {
        self.config = self.client.call("config.get", None).await.ok();

        if let Some(config) = &self.config
            && let Some(pomodoro) = config.get("pomodoro")
        {
            if let Some(countdown_seconds) =
                pomodoro.get("countdown_default").and_then(|v| v.as_u64())
            {
                self.countdown_minutes = countdown_seconds / 60;
            }

            if let Some(work_seconds) = pomodoro.get("work_duration").and_then(|v| v.as_u64()) {
                self.pomodoro_minutes = work_seconds / 60;
            }
        }

        Ok(())
    }

    pub async fn refresh_profiles(&mut self) -> Result<()> {
        if let Ok(profiles) = self.client.profile_list().await {
            self.profiles = profiles.as_array().cloned().unwrap_or_default();
            self.status_message = format!("Loaded {} profiles", self.profiles.len());
        }
        Ok(())
    }

    pub async fn refresh_reports(&mut self) -> Result<()> {
        if self.report_profile == "all" {
            self.refresh_all_profile_reports().await?;
        } else {
            let params = Some(serde_json::json!({"profile_id": self.report_profile}));

            let stats_method = match self.report_period.as_str() {
                "week" => "entry.stats_week",
                "month" => "entry.stats_month",
                _ => "entry.stats_today",
            };

            self.report_stats = self.client.call(stats_method, params.clone()).await.ok();

            let entries_method = match self.report_period.as_str() {
                "week" => "entry.week",
                "month" => "entry.month",
                _ => "entry.today",
            };

            self.report_entries = self
                .client
                .call(entries_method, params)
                .await
                .ok()
                .and_then(|v| v.as_array().cloned())
                .unwrap_or_default();
        }

        let profile_label = if self.report_profile == "all" {
            "all profiles"
        } else {
            &self.report_profile
        };
        self.status_message = format!(
            "Refreshed {} report for {}",
            self.report_period, profile_label
        );
        Ok(())
    }

    async fn refresh_all_profile_reports(&mut self) -> Result<()> {
        let cache_key = format!("all_{}", self.report_period);
        if let Some((cached_entries, timestamp)) = self.cross_profile_cache.get(&cache_key)
            && timestamp.elapsed() < Duration::from_secs(30)
        {
            self.report_entries = cached_entries.clone();
            self.status_message = "Loaded from cache".to_string();
            self.update_cross_profile_stats();
            return Ok(());
        }

        self.status_message = format!("Loading {} for all profiles...", self.report_period);

        let method = match self.report_period.as_str() {
            "week" => "entry.week_all_profiles",
            "month" => "entry.month_all_profiles",
            _ => "entry.today_all_profiles",
        };

        match self.client.call(method, None).await {
            Ok(value) => {
                let all_entries = value.as_array().cloned().unwrap_or_default();
                self.cross_profile_cache
                    .insert(cache_key, (all_entries.clone(), Instant::now()));

                self.report_entries = all_entries;
                self.update_cross_profile_stats();

                self.status_message = format!(
                    "Loaded {} entries from all profiles",
                    self.report_entries.len()
                );
            }
            Err(e) => {
                self.status_message = format!("Failed to fetch all profiles: {}", e);
            }
        }

        Ok(())
    }

    fn update_cross_profile_stats(&mut self) {
        let total_seconds: u64 = self
            .report_entries
            .iter()
            .filter_map(|e| e.get("duration_seconds").and_then(|v| v.as_u64()))
            .sum();

        self.report_stats = Some(serde_json::json!({
            "total_duration_seconds": total_seconds,
            "total_entries": self.report_entries.len(),
            "pomodoro_count": 0,
            "manual_count": 0,
            "avg_duration_seconds": if !self.report_entries.is_empty() {
                total_seconds / self.report_entries.len() as u64
            } else {
                0
            }
        }));
    }

    pub async fn refresh_logs(&mut self) -> Result<()> {
        use mootimer_core::storage::init_data_dir;
        use std::fs;

        let data_dir = init_data_dir()?;
        let log_file_path = data_dir.join("daemon.log");

        if log_file_path.exists() {
            let content = fs::read_to_string(&log_file_path)?;
            self.log_lines = content
                .lines()
                .rev()
                .take(1000)
                .map(|s| s.to_string())
                .collect();
        } else {
            self.log_lines = vec!["Log file not found".to_string()];
        }

        self.status_message = "Logs refreshed".to_string();
        Ok(())
    }

    pub async fn start_timer(
        &mut self,
        pomodoro: bool,
        duration_minutes: Option<u64>,
    ) -> Result<()> {
        if let Some(timer) = &self.timer_info
            && let Some(state) = timer.get("state").and_then(|v| v.as_str())
            && (state == "running" || state == "paused")
        {
            self.status_message = "Timer already active! Stop it first with [x]".to_string();
            return Ok(());
        }

        let filtered_tasks = self.get_filtered_tasks();
        let task_id = filtered_tasks
            .get(self.selected_task_index)
            .and_then(|t| t.get("id"))
            .and_then(|v| v.as_str());

        let result = if pomodoro {
            self.client
                .timer_start_pomodoro(&self.profile_id, task_id, duration_minutes)
                .await
        } else {
            self.client
                .timer_start_manual(&self.profile_id, task_id)
                .await
        };

        match result {
            Ok(_) => {
                let task_name = task_id.and_then(|id| {
                    self.tasks
                        .iter()
                        .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(id))
                        .and_then(|t| t.get("title"))
                        .and_then(|v| v.as_str())
                });

                self.status_message = if let Some(name) = task_name {
                    format!(
                        "Started {} timer for: {}",
                        if pomodoro { "pomodoro" } else { "manual" },
                        name
                    )
                } else {
                    format!(
                        "Started {} timer (no task)",
                        if pomodoro { "pomodoro" } else { "manual" }
                    )
                };
                self.refresh_timer().await?;
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }

        Ok(())
    }

    pub async fn start_countdown_timer(&mut self) -> Result<()> {
        if let Some(timer) = &self.timer_info
            && let Some(state) = timer.get("state").and_then(|v| v.as_str())
            && (state == "running" || state == "paused")
        {
            self.status_message = "Timer already active! Stop it first with [x]".to_string();
            return Ok(());
        }

        let filtered_tasks = self.get_filtered_tasks();
        let task_id = filtered_tasks
            .get(self.selected_task_index)
            .and_then(|t| t.get("id"))
            .and_then(|v| v.as_str());

        let result = self
            .client
            .timer_start_countdown(&self.profile_id, task_id, self.countdown_minutes)
            .await;

        match result {
            Ok(_) => {
                let task_name = task_id.and_then(|id| {
                    self.tasks
                        .iter()
                        .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(id))
                        .and_then(|t| t.get("title"))
                        .and_then(|v| v.as_str())
                });

                self.status_message = if let Some(name) = task_name {
                    format!(
                        "Started {}m countdown for: {}",
                        self.countdown_minutes, name
                    )
                } else {
                    format!("Started {}m countdown (no task)", self.countdown_minutes)
                };
                self.refresh_timer().await?;
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }

        Ok(())
    }

    pub fn cycle_timer_type(&mut self) {
        self.selected_timer_type = match self.selected_timer_type {
            TimerType::Manual => TimerType::Pomodoro,
            TimerType::Pomodoro => TimerType::Countdown,
            TimerType::Countdown => TimerType::Manual,
        };
        self.status_message = format!("Timer type: {:?}", self.selected_timer_type);
    }

    pub fn cycle_timer_type_reverse(&mut self) {
        self.selected_timer_type = match self.selected_timer_type {
            TimerType::Manual => TimerType::Countdown,
            TimerType::Countdown => TimerType::Pomodoro,
            TimerType::Pomodoro => TimerType::Manual,
        };
        self.status_message = format!("Timer type: {:?}", self.selected_timer_type);
    }

    pub fn adjust_timer_duration_up(&mut self) {
        match self.selected_timer_type {
            TimerType::Manual => {
                self.status_message = "Manual timer has no duration".to_string();
            }
            TimerType::Pomodoro => {
                if self.pomodoro_minutes < 60 {
                    self.pomodoro_minutes += 1;
                }
                self.status_message = format!("Pomodoro: {}m", self.pomodoro_minutes);
            }
            TimerType::Countdown => {
                if self.countdown_minutes < 180 {
                    self.countdown_minutes += 1;
                }
                self.status_message = format!("Countdown: {}m", self.countdown_minutes);
            }
        }
    }

    pub fn adjust_timer_duration_down(&mut self) {
        match self.selected_timer_type {
            TimerType::Manual => {
                self.status_message = "Manual timer has no duration".to_string();
            }
            TimerType::Pomodoro => {
                if self.pomodoro_minutes > 1 {
                    self.pomodoro_minutes -= 1;
                }
                self.status_message = format!("Pomodoro: {}m", self.pomodoro_minutes);
            }
            TimerType::Countdown => {
                if self.countdown_minutes > 1 {
                    self.countdown_minutes -= 1;
                }
                self.status_message = format!("Countdown: {}m", self.countdown_minutes);
            }
        }
    }

    pub async fn start_selected_timer(&mut self) -> Result<()> {
        match self.selected_timer_type {
            TimerType::Manual => self.start_timer(false, None).await,
            TimerType::Pomodoro => self.start_timer(true, Some(self.pomodoro_minutes)).await,
            TimerType::Countdown => self.start_countdown_timer().await,
        }
    }

    pub async fn toggle_pause(&mut self) -> Result<()> {
        if let Some(timer) = &self.timer_info {
            let state = timer.get("state").and_then(|v| v.as_str()).unwrap_or("");

            let result = if state == "paused" {
                self.client.timer_resume(&self.profile_id).await
            } else if state == "running" {
                self.client.timer_pause(&self.profile_id).await
            } else {
                return Ok(());
            };

            match result {
                Ok(_) => {
                    self.status_message = if state == "paused" {
                        "Resumed"
                    } else {
                        "Paused"
                    }
                    .to_string();
                    self.refresh_timer().await?;
                }
                Err(e) => {
                    self.status_message = format!("Error: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn resume(&mut self) -> Result<()> {
        match self.client.timer_resume(&self.profile_id).await {
            Ok(_) => {
                self.status_message = "Resumed".to_string();
                self.refresh_timer().await?;
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
        Ok(())
    }

    pub async fn stop_timer(&mut self) -> Result<()> {
        match self.client.timer_stop(&self.profile_id).await {
            Ok(_) => {
                self.status_message = "Timer stopped, entry saved!".to_string();
                self.selected_timer_button = 0;
                self.refresh_timer().await?;
                self.refresh_stats().await?;
                self.refresh_entries().await?;
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
        Ok(())
    }

    pub async fn quick_task_create(&mut self, title: &str) -> Result<()> {
        if title.trim().is_empty() {
            return Ok(());
        }

        match self.client.task_create(&self.profile_id, title, None).await {
            Ok(_) => {
                self.status_message = format!("Quick add: {}", title);
                self.refresh_tasks().await?;
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
        Ok(())
    }

    pub async fn archive_task(&mut self, task_id: &str) -> Result<()> {
        if let Some(task) = self
            .tasks
            .iter()
            .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(task_id))
        {
            let mut task_clone = task.clone();

            let current_status = task_clone
                .get("status")
                .and_then(|v| v.as_str())
                .unwrap_or("todo");
            let new_status = if current_status == "archived" {
                "todo"
            } else {
                "archived"
            };

            if let Some(obj) = task_clone.as_object_mut() {
                obj.insert(
                    "status".to_string(),
                    serde_json::Value::String(new_status.to_string()),
                );
                match self.client.task_update(&self.profile_id, task_clone).await {
                    Ok(_) => {
                        self.status_message = if new_status == "archived" {
                            "Task archived".to_string()
                        } else {
                            "Task restored to To Do".to_string()
                        };
                        self.refresh_tasks().await?;
                    }
                    Err(e) => {
                        self.status_message = format!("Error updating task: {}", e);
                    }
                }
            }
        }
        Ok(())
    }

    pub async fn submit_input(&mut self) -> Result<()> {
        match self.input_mode {
            InputMode::NewTask => {
                if !self.input_buffer.is_empty() {
                    let title = self.input_buffer.clone();
                    let description = if self.input_buffer_2.trim().is_empty() {
                        None
                    } else {
                        Some(self.input_buffer_2.clone())
                    };

                    match self
                        .client
                        .task_create(&self.profile_id, &title, description.as_deref())
                        .await
                    {
                        Ok(_) => {
                            self.status_message = format!("Created task: {}", title);
                            self.refresh_tasks().await?;
                        }
                        Err(e) => {
                            self.status_message = format!("Error creating task: {}", e);
                        }
                    }
                }
            }
            InputMode::QuickAddTask => {
                if !self.input_buffer.is_empty() {
                    let title = self.input_buffer.clone();
                    self.quick_task_create(&title).await?;
                }
            }
            InputMode::EditTask => {
                if !self.input_buffer.is_empty() {
                    let new_title = self.input_buffer.clone();
                    let new_desc = self.input_buffer_2.clone();

                    let task_opt = {
                        let filtered_tasks = self.get_filtered_tasks();
                        filtered_tasks
                            .get(self.selected_task_index)
                            .map(|t| (*t).clone())
                    };

                    if let Some(mut task) = task_opt
                        && let Some(obj) = task.as_object_mut()
                    {
                        obj.insert(
                            "title".to_string(),
                            serde_json::Value::String(new_title.clone()),
                        );

                        if new_desc.trim().is_empty() {
                            obj.insert(
                                "description".to_string(),
                                serde_json::Value::String("".to_string()),
                            );
                        } else {
                            obj.insert(
                                "description".to_string(),
                                serde_json::Value::String(new_desc),
                            );
                        }

                        match self.client.task_update(&self.profile_id, task).await {
                            Ok(_) => {
                                self.status_message = format!("Updated task: {}", new_title);
                                self.refresh_tasks().await?;
                            }
                            Err(e) => {
                                self.status_message = format!("Error updating task: {}", e);
                            }
                        }
                    }
                }
            }
            InputMode::SearchTasks => {
                self.task_search = self.input_buffer.clone();
                self.status_message = if self.task_search.is_empty() {
                    "Search cleared".to_string()
                } else {
                    format!("Searching tasks: {}", self.task_search)
                };
                self.selected_task_index = 0;
            }
            InputMode::FilterEntries => {
                self.entry_filter = self.input_buffer.clone();
                self.status_message = if self.entry_filter.is_empty() {
                    "Filter cleared".to_string()
                } else {
                    format!("Filtered by: {}", self.entry_filter)
                };
                self.selected_entry_index = 0;
            }
            InputMode::ConfigPomodoro => {
                self.update_config_duration("work_duration", "Work duration")
                    .await?;
            }
            InputMode::ConfigShortBreak => {
                self.update_config_duration("short_break", "Short break")
                    .await?;
            }
            InputMode::ConfigLongBreak => {
                self.update_config_duration("long_break", "Long break")
                    .await?;
            }
            InputMode::ConfigCountdown => {
                self.update_config_duration("countdown_default", "Countdown default")
                    .await?;
            }
            InputMode::NewProfile => {
                if !self.input_buffer.is_empty() {
                    let name = self.input_buffer.clone();
                    self.create_profile(&name).await?;
                }
            }
            InputMode::RenameProfile => {
                if !self.input_buffer.is_empty() {
                    let name = self.input_buffer.clone();
                    self.rename_selected_profile(&name).await?;
                }
            }
            InputMode::EditEntryDuration => {
                if let Ok(minutes) = self.input_buffer.parse::<u64>() {
                    let filtered_entries = self.get_filtered_entries();
                    if let Some(entry_ref) = filtered_entries.get(self.selected_entry_index) {
                        let mut entry = (*entry_ref).clone();
                        if let Some(obj) = entry.as_object_mut() {
                            let seconds = minutes * 60;
                            obj.insert(
                                "duration_seconds".to_string(),
                                serde_json::Value::Number(seconds.into()),
                            );

                            if let Some(start_str) = obj.get("start_time").and_then(|v| v.as_str())
                                && let Ok(start_time) =
                                    chrono::DateTime::parse_from_rfc3339(start_str)
                            {
                                let end_time =
                                    start_time + chrono::Duration::seconds(seconds as i64);
                                obj.insert(
                                    "end_time".to_string(),
                                    serde_json::Value::String(end_time.to_rfc3339()),
                                );
                            }

                            match self.client.entry_update(&self.profile_id, entry).await {
                                Ok(_) => {
                                    self.status_message =
                                        format!("Updated entry duration to {}m", minutes);
                                    self.refresh_entries().await?;
                                }
                                Err(e) => {
                                    self.status_message = format!("Error updating entry: {}", e);
                                }
                            }
                        }
                    }
                }
            }
            _ => {}
        }

        self.input_mode = InputMode::Normal;
        self.input_buffer.clear();
        Ok(())
    }

    pub async fn delete_selected_entry(&mut self) -> Result<()> {
        let filtered_entries = self.get_filtered_entries();
        if let Some(entry) = filtered_entries.get(self.selected_entry_index)
            && let Some(id) = entry.get("id").and_then(|v| v.as_str())
        {
            match self.client.entry_delete(&self.profile_id, id).await {
                Ok(_) => {
                    self.status_message = "Entry deleted".to_string();
                    self.refresh_entries().await?;
                    let new_len = self.get_filtered_entries().len();
                    if self.selected_entry_index >= new_len {
                        self.selected_entry_index = new_len.saturating_sub(1);
                    }
                }
                Err(e) => {
                    self.status_message = format!("Error: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn edit_selected_entry(&mut self) -> Result<()> {
        let filtered_entries = self.get_filtered_entries();
        if let Some(entry) = filtered_entries.get(self.selected_entry_index) {
            let duration_secs = entry
                .get("duration_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let minutes = duration_secs / 60;

            self.input_mode = InputMode::EditEntryDuration;
            self.input_buffer = minutes.to_string();
            self.status_message = "Edit duration (minutes):".to_string();
        }
        Ok(())
    }

    pub async fn delete_selected_task(&mut self) -> Result<()> {
        let filtered_tasks = self.get_filtered_tasks();
        if let Some(task) = filtered_tasks.get(self.selected_task_index)
            && let Some(id) = task.get("id").and_then(|v| v.as_str())
        {
            match self.client.task_delete(&self.profile_id, id).await {
                Ok(_) => {
                    self.status_message = "Task deleted".to_string();
                    self.refresh_tasks().await?;
                    let new_len = self.get_filtered_tasks().len();
                    if self.selected_task_index >= new_len {
                        self.selected_task_index = new_len.saturating_sub(1);
                    }
                }
                Err(e) => {
                    self.status_message = format!("Error: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn edit_selected_task(&mut self) -> Result<()> {
        let task_data = {
            let filtered_tasks = self.get_filtered_tasks();
            if let Some(task) = filtered_tasks.get(self.selected_task_index) {
                let title = task
                    .get("title")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                let desc = task
                    .get("description")
                    .and_then(|v| v.as_str())
                    .unwrap_or("")
                    .to_string();
                Some((title, desc))
            } else {
                None
            }
        };

        if let Some((title, desc)) = task_data {
            self.input_mode = InputMode::EditTask;
            self.input_buffer = title;
            self.input_buffer_2 = desc;
            self.focused_input_field = 0;
            self.status_message = "Edit Task".to_string();
        }
        Ok(())
    }

    pub async fn show_entries_for_day(&mut self) -> Result<()> {
        self.refresh_entries().await?;
        self.status_message = "Showing today's entries".to_string();
        Ok(())
    }

    pub async fn show_entries_for_week(&mut self) -> Result<()> {
        if let Ok(entries) = self
            .client
            .call(
                "entry.week",
                Some(serde_json::json!({"profile_id": self.profile_id})),
            )
            .await
        {
            self.entries = entries.as_array().cloned().unwrap_or_default();
            self.status_message = "Showing this week's entries".to_string();
        }
        Ok(())
    }

    pub async fn show_entries_for_month(&mut self) -> Result<()> {
        if let Ok(entries) = self
            .client
            .call(
                "entry.month",
                Some(serde_json::json!({"profile_id": self.profile_id})),
            )
            .await
        {
            self.entries = entries.as_array().cloned().unwrap_or_default();
            self.status_message = "Showing this month's entries".to_string();
        }
        Ok(())
    }

    pub async fn adjust_pomodoro_setting(&mut self, item: SettingsItem, delta: i32) -> Result<()> {
        if let Some(config) = &self.config {
            let (key, current_val) = match item {
                SettingsItem::PomodoroWork => (
                    "work_duration",
                    config
                        .get("pomodoro")
                        .and_then(|p| p.get("work_duration"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(1500),
                ),
                SettingsItem::PomodoroShortBreak => (
                    "short_break",
                    config
                        .get("pomodoro")
                        .and_then(|p| p.get("short_break"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(300),
                ),
                SettingsItem::PomodoroLongBreak => (
                    "long_break",
                    config
                        .get("pomodoro")
                        .and_then(|p| p.get("long_break"))
                        .and_then(|v| v.as_u64())
                        .unwrap_or(900),
                ),
                _ => return Ok(()),
            };

            let current_minutes = current_val / 60;
            let new_minutes = (current_minutes as i32 + delta).max(1) as u64;
            let new_seconds = new_minutes * 60;

            match self
                .client
                .call(
                    "config.update_pomodoro",
                    Some(serde_json::json!({key: new_seconds})),
                )
                .await
            {
                Ok(_) => {
                    self.status_message =
                        format!("Set {} to {} minutes", key.replace('_', " "), new_minutes);
                    self.refresh_config().await?;
                }
                Err(e) => {
                    self.status_message = format!("Error: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn toggle_git_sync(&mut self) -> Result<()> {
        if let Some(config) = &self.config {
            let auto_commit = config
                .get("sync")
                .and_then(|s| s.get("auto_commit"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);

            match self
                .client
                .call(
                    "config.update_sync",
                    Some(serde_json::json!({"auto_commit": !auto_commit})),
                )
                .await
            {
                Ok(_) => {
                    self.status_message = format!(
                        "Auto-commit {}",
                        if !auto_commit { "enabled" } else { "disabled" }
                    );
                    self.refresh_config().await?;
                }
                Err(e) => {
                    self.status_message = format!("Error: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn init_git_sync(&mut self) -> Result<()> {
        match self.client.call("sync.init", None).await {
            Ok(_) => {
                self.status_message = "Git sync initialized!".to_string();
                self.refresh_sync().await?;
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
        Ok(())
    }

    pub async fn sync_now(&mut self) -> Result<()> {
        self.status_message = "Syncing...".to_string();
        match self.client.call("sync.sync", None).await {
            Ok(result) => {
                let pulled = result
                    .get("pulled")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                let pushed = result
                    .get("pushed")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);

                self.status_message = match (pulled, pushed) {
                    (true, true) => "Sync complete (Pulled & Pushed)".to_string(),
                    (true, false) => "Sync complete (Pulled)".to_string(),
                    (false, true) => "Sync complete (Pushed)".to_string(),
                    (false, false) => "Sync complete (Already up to date)".to_string(),
                };
                self.refresh_all().await?;
            }
            Err(e) => {
                self.status_message = format!("Sync failed: {}", e);
            }
        }
        Ok(())
    }

    async fn update_config_duration(&mut self, config_key: &str, display_name: &str) -> Result<()> {
        let Ok(minutes) = self.input_buffer.parse::<u64>() else {
            return Ok(());
        };

        if minutes == 0 {
            self.status_message = "Duration cannot be zero.".to_string();
            return Ok(());
        }

        let seconds = minutes * 60;
        match self
            .client
            .call(
                "config.update_pomodoro",
                Some(serde_json::json!({ config_key: seconds })),
            )
            .await
        {
            Ok(_) => {
                self.status_message = format!("{} set to {} minutes", display_name, minutes);
                if config_key == "countdown_default" {
                    self.countdown_minutes = minutes;
                }
                self.refresh_config().await?;
            }
            Err(e) => {
                self.status_message = format!("Error: {}", e);
            }
        }
        Ok(())
    }

    pub fn toggle_audio_alerts(&mut self) {
        self.audio_alerts_enabled = !self.audio_alerts_enabled;
        self.status_message = if self.audio_alerts_enabled {
            "Audio Alerts: Enabled 🔔".to_string()
        } else {
            "Audio Alerts: Disabled 🔇".to_string()
        };
    }

    pub fn toggle_cow_modal(&mut self) {
        self.cow_modal_enabled = !self.cow_modal_enabled;
        self.status_message = if self.cow_modal_enabled {
            "Cow Modal: Enabled 🐮".to_string()
        } else {
            "Cow Modal: Disabled".to_string()
        };
    }

    pub async fn create_profile(&mut self, name: &str) -> Result<()> {
        let id = name.to_lowercase().replace(' ', "_");
        match self.client.profile_create(&id, name, None).await {
            Ok(_) => {
                self.status_message = format!("Created profile: {}", name);
                self.refresh_profiles().await?;
            }
            Err(e) => {
                self.status_message = format!("Error creating profile: {}", e);
            }
        }
        Ok(())
    }

    pub async fn delete_selected_profile(&mut self) -> Result<()> {
        if let Some(profile) = self.profiles.get(self.selected_profile_index) {
            let id = profile.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let name = profile.get("name").and_then(|v| v.as_str()).unwrap_or("");

            if id == self.profile_id {
                self.status_message = "Cannot delete active profile!".to_string();
                return Ok(());
            }

            match self.client.profile_delete(id).await {
                Ok(_) => {
                    self.status_message = format!("Deleted profile: {}", name);
                    self.refresh_profiles().await?;
                }
                Err(e) => {
                    self.status_message = format!("Error deleting profile: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn switch_to_selected_profile(&mut self) -> Result<()> {
        if let Some(profile) = self.profiles.get(self.selected_profile_index) {
            let id = profile.get("id").and_then(|v| v.as_str()).unwrap_or("");
            let name = profile.get("name").and_then(|v| v.as_str()).unwrap_or("");

            self.profile_id = id.to_string();
            self.status_message = format!("Switched to profile: {}", name);

            self.refresh_all().await?;
        }
        Ok(())
    }

    pub async fn rename_selected_profile(&mut self, new_name: &str) -> Result<()> {
        if let Some(mut profile) = self.profiles.get(self.selected_profile_index).cloned()
            && let Some(obj) = profile.as_object_mut()
        {
            obj.insert(
                "name".to_string(),
                serde_json::Value::String(new_name.to_string()),
            );

            match self.client.profile_update(profile).await {
                Ok(_) => {
                    self.status_message = format!("Renamed profile to: {}", new_name);
                    self.refresh_profiles().await?;
                }
                Err(e) => {
                    self.status_message = format!("Error renaming profile: {}", e);
                }
            }
        }
        Ok(())
    }

    pub async fn toggle_report_profile(&mut self) -> Result<()> {
        if self.report_profile == "all" {
            self.report_profile = self.profile_id.clone();
            self.status_message = format!("Reports: Current Profile ({})", self.profile_id);
        } else {
            self.report_profile = "all".to_string();
            self.status_message = "Reports: All Profiles".to_string();
        }

        self.refresh_reports().await?;
        Ok(())
    }
}
