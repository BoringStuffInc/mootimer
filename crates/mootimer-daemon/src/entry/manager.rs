use chrono::{DateTime, Datelike, Utc};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::event_manager::EventManager;
use crate::events::EntryEvent;
use mootimer_core::{
    Result as CoreResult,
    models::Entry,
    storage::{EntryStorage, init_data_dir},
};

#[derive(Debug, thiserror::Error)]
pub enum EntryManagerError {
    #[error("Entry not found: {0}")]
    NotFound(String),

    #[error("Storage error: {0}")]
    Storage(#[from] mootimer_core::Error),

    #[error("Invalid entry: {0}")]
    Invalid(String),

    #[error("Task join error: {0}")]
    JoinError(String),
}

pub type Result<T> = std::result::Result<T, EntryManagerError>;

#[derive(Debug, Clone)]
pub struct EntryFilter {
    pub start_date: Option<DateTime<Utc>>,
    pub end_date: Option<DateTime<Utc>>,
    pub task_id: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Clone)]
pub struct EntryStats {
    pub total_entries: usize,
    pub total_duration_seconds: u64,
    pub total_duration_hours: f64,
    pub pomodoro_count: usize,
    pub manual_count: usize,
    pub avg_duration_seconds: u64,
}

pub struct EntryManager {
    data_dir: PathBuf,
    cache: Arc<RwLock<HashMap<String, Vec<Entry>>>>,
    event_manager: Arc<EventManager>,
}

impl EntryManager {
    pub fn new(event_manager: Arc<EventManager>) -> CoreResult<Self> {
        let data_dir = init_data_dir()?;

        Ok(Self {
            data_dir,
            cache: Arc::new(RwLock::new(HashMap::new())),
            event_manager,
        })
    }

    pub async fn load_profile(&self, profile_id: &str) -> Result<()> {
        let data_dir = self.data_dir.clone();
        let profile_id_owned = profile_id.to_string();

        let entries = tokio::task::spawn_blocking(move || {
            let storage = EntryStorage::new(data_dir);
            storage.load(&profile_id_owned)
        })
        .await
        .map_err(|e| EntryManagerError::JoinError(e.to_string()))??;

        tracing::info!(
            "Loaded {} entries for profile '{}'",
            entries.len(),
            profile_id
        );
        let mut cache = self.cache.write().await;
        cache.insert(profile_id.to_string(), entries);
        Ok(())
    }

    pub async fn add(&self, profile_id: &str, entry: Entry) -> Result<Entry> {
        entry
            .validate()
            .map_err(|e| EntryManagerError::Invalid(e.to_string()))?;

        let data_dir = self.data_dir.clone();
        let profile_id_owned = profile_id.to_string();
        let entry_clone = entry.clone();

        tokio::task::spawn_blocking(move || {
            let storage = EntryStorage::new(data_dir);
            storage.append(&profile_id_owned, &entry_clone)
        })
        .await
        .map_err(|e| EntryManagerError::JoinError(e.to_string()))??;

        {
            let mut cache = self.cache.write().await;
            cache
                .entry(profile_id.to_string())
                .or_insert_with(Vec::new)
                .push(entry.clone());
        }

        let event = EntryEvent::added(profile_id.to_string(), entry.clone());
        self.event_manager.emit_entry(event);

        Ok(entry)
    }

    pub async fn get_all(&self, profile_id: &str) -> Result<Vec<Entry>> {
        {
            let cache = self.cache.read().await;
            if let Some(entries) = cache.get(profile_id) {
                return Ok(entries.clone());
            }
        }

        self.load_profile(profile_id).await?;

        let cache = self.cache.read().await;
        Ok(cache.get(profile_id).cloned().unwrap_or_default())
    }

    pub async fn filter(&self, profile_id: &str, filter: EntryFilter) -> Result<Vec<Entry>> {
        let entries = self.get_all(profile_id).await?;

        Ok(entries
            .into_iter()
            .filter(|entry| {
                if let Some(start) = filter.start_date
                    && entry.start_time < start
                {
                    return false;
                }
                if let Some(end) = filter.end_date
                    && entry.start_time > end
                {
                    return false;
                }

                if let Some(ref task_id) = filter.task_id
                    && entry.task_id.as_ref() != Some(task_id)
                {
                    return false;
                }

                if let Some(ref tags) = filter.tags
                    && !tags.iter().any(|tag| entry.has_tag(tag))
                {
                    return false;
                }

                true
            })
            .collect())
    }

    pub async fn get_today(&self, profile_id: &str) -> Result<Vec<Entry>> {
        let now = Utc::now();
        let start_of_day = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc())
            .unwrap_or(now);

        self.filter(
            profile_id,
            EntryFilter {
                start_date: Some(start_of_day),
                end_date: None,
                task_id: None,
                tags: None,
            },
        )
        .await
    }

    pub async fn get_week(&self, profile_id: &str) -> Result<Vec<Entry>> {
        let now = Utc::now();
        let days_from_monday = now.weekday().num_days_from_monday();
        let start_of_week = now
            .date_naive()
            .and_hms_opt(0, 0, 0)
            .map(|dt| dt.and_utc())
            .map(|dt| dt - chrono::Duration::days(days_from_monday as i64))
            .unwrap_or(now);

        self.filter(
            profile_id,
            EntryFilter {
                start_date: Some(start_of_week),
                end_date: None,
                task_id: None,
                tags: None,
            },
        )
        .await
    }

    pub async fn get_month(&self, profile_id: &str) -> Result<Vec<Entry>> {
        let now = Utc::now();
        let start_of_month = now
            .date_naive()
            .with_day(1)
            .and_then(|d| d.and_hms_opt(0, 0, 0))
            .map(|dt| dt.and_utc())
            .unwrap_or(now);

        self.filter(
            profile_id,
            EntryFilter {
                start_date: Some(start_of_month),
                end_date: None,
                task_id: None,
                tags: None,
            },
        )
        .await
    }

    pub fn calculate_stats(entries: &[Entry]) -> EntryStats {
        let total_duration: u64 = entries.iter().map(|e| e.duration_seconds).sum();
        let pomodoro_count = entries
            .iter()
            .filter(|e| e.mode == mootimer_core::models::TimerMode::Pomodoro)
            .count();
        let manual_count = entries.len() - pomodoro_count;

        EntryStats {
            total_entries: entries.len(),
            total_duration_seconds: total_duration,
            total_duration_hours: total_duration as f64 / 3600.0,
            pomodoro_count,
            manual_count,
            avg_duration_seconds: if entries.is_empty() {
                0
            } else {
                total_duration / entries.len() as u64
            },
        }
    }

    pub async fn get_today_stats(&self, profile_id: &str) -> Result<EntryStats> {
        let entries = self.get_today(profile_id).await?;
        Ok(Self::calculate_stats(&entries))
    }

    pub async fn get_week_stats(&self, profile_id: &str) -> Result<EntryStats> {
        let entries = self.get_week(profile_id).await?;
        Ok(Self::calculate_stats(&entries))
    }

    pub async fn get_month_stats(&self, profile_id: &str) -> Result<EntryStats> {
        let entries = self.get_month(profile_id).await?;
        Ok(Self::calculate_stats(&entries))
    }

    pub async fn delete(&self, profile_id: &str, entry_id: &str) -> Result<()> {
        let mut entries = self.get_all(profile_id).await?;

        let initial_len = entries.len();
        entries.retain(|e| e.id != entry_id);

        if entries.len() == initial_len {
            return Err(EntryManagerError::NotFound(entry_id.to_string()));
        }

        let data_dir = self.data_dir.clone();
        let profile_id_owned = profile_id.to_string();
        let entries_clone = entries.clone();

        tokio::task::spawn_blocking(move || {
            let storage = EntryStorage::new(data_dir);
            storage.save_all(&profile_id_owned, &entries_clone)
        })
        .await
        .map_err(|e| EntryManagerError::JoinError(e.to_string()))??;

        {
            let mut cache = self.cache.write().await;
            cache.insert(profile_id.to_string(), entries);
        }

        let event = EntryEvent::deleted(profile_id.to_string(), entry_id.to_string());
        self.event_manager.emit_entry(event);

        Ok(())
    }

    pub async fn update(&self, profile_id: &str, entry: Entry) -> Result<()> {
        let mut entries = self.get_all(profile_id).await?;

        let mut found = false;
        for e in entries.iter_mut() {
            if e.id == entry.id {
                *e = entry.clone();
                found = true;
                break;
            }
        }

        if !found {
            return Err(EntryManagerError::NotFound(entry.id));
        }

        let data_dir = self.data_dir.clone();
        let profile_id_owned = profile_id.to_string();
        let entries_clone = entries.clone();

        tokio::task::spawn_blocking(move || {
            let storage = EntryStorage::new(data_dir);
            storage.save_all(&profile_id_owned, &entries_clone)
        })
        .await
        .map_err(|e| EntryManagerError::JoinError(e.to_string()))??;

        {
            let mut cache = self.cache.write().await;
            cache.insert(profile_id.to_string(), entries);
        }

        let event = EntryEvent::updated(profile_id.to_string(), entry.clone());
        self.event_manager.emit_entry(event);

        Ok(())
    }

    pub async fn move_entries_for_task(
        &self,
        source_profile_id: &str,
        target_profile_id: &str,
        task_id: &str,
    ) -> Result<usize> {
        let source_entries = self.get_all(source_profile_id).await?;

        let (entries_to_move, entries_to_keep): (Vec<Entry>, Vec<Entry>) = source_entries
            .into_iter()
            .partition(|e| e.task_id.as_deref() == Some(task_id));

        if entries_to_move.is_empty() {
            return Ok(0);
        }

        let moved_count = entries_to_move.len();

        let data_dir = self.data_dir.clone();
        let source_id = source_profile_id.to_string();
        let entries_keep = entries_to_keep.clone();

        tokio::task::spawn_blocking(move || {
            let storage = EntryStorage::new(data_dir);
            storage.save_all(&source_id, &entries_keep)
        })
        .await
        .map_err(|e| EntryManagerError::JoinError(e.to_string()))??;

        {
            let mut cache = self.cache.write().await;
            cache.insert(source_profile_id.to_string(), entries_to_keep);
        }

        let mut target_entries = self.get_all(target_profile_id).await.unwrap_or_default();
        target_entries.extend(entries_to_move.clone());

        let data_dir = self.data_dir.clone();
        let target_id = target_profile_id.to_string();
        let entries_target = target_entries.clone();

        tokio::task::spawn_blocking(move || {
            let storage = EntryStorage::new(data_dir);
            storage.save_all(&target_id, &entries_target)
        })
        .await
        .map_err(|e| EntryManagerError::JoinError(e.to_string()))??;

        {
            let mut cache = self.cache.write().await;
            cache.insert(target_profile_id.to_string(), target_entries);
        }

        for entry in &entries_to_move {
            let event = EntryEvent::deleted(source_profile_id.to_string(), entry.id.clone());
            self.event_manager.emit_entry(event);

            let event = EntryEvent::added(target_profile_id.to_string(), entry.clone());
            self.event_manager.emit_entry(event);
        }

        Ok(moved_count)
    }
}

impl Default for EntryManager {
    fn default() -> Self {
        Self::new(Arc::new(crate::event_manager::EventManager::new()))
            .expect("Failed to create EntryManager")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event_manager::EventManager;
    use chrono::Duration;
    use mootimer_core::models::{Entry, TimerMode};
    use serial_test::serial;
    use std::sync::Arc;
    use tempfile::TempDir;

    fn create_manager(_temp_dir: &TempDir) -> EntryManager {
        let event_manager = Arc::new(EventManager::new());
        unsafe {
            std::env::set_var("HOME", _temp_dir.path());
            std::env::set_var("XDG_DATA_HOME", _temp_dir.path().join("data"));
            std::env::set_var("XDG_CONFIG_HOME", _temp_dir.path().join("config"));
        }
        EntryManager::new(event_manager).unwrap()
    }

    #[tokio::test]
    #[serial]
    async fn test_add_entry() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let profile_id = "test_entry";

        let entry = Entry::new(
            Some("task1".to_string()),
            Some("Task".to_string()),
            TimerMode::Manual,
        );
        let added = manager.add(profile_id, entry).await.unwrap();

        assert!(added.is_active());
    }

    #[tokio::test]
    #[serial]
    async fn test_get_all_entries() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let profile_id = "test_get_all";

        let entry1 = Entry::new(None, None, TimerMode::Manual);
        let entry2 = Entry::new(None, None, TimerMode::Pomodoro);

        manager.add(profile_id, entry1).await.unwrap();
        manager.add(profile_id, entry2).await.unwrap();

        let entries = manager.get_all(profile_id).await.unwrap();
        assert!(entries.len() >= 2);
    }

    #[tokio::test]
    #[serial]
    async fn test_filter_by_task() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let profile_id = "test_filter";

        let entry1 = Entry::new(Some("task1".to_string()), None, TimerMode::Manual);
        let entry2 = Entry::new(Some("task2".to_string()), None, TimerMode::Manual);

        manager.add(profile_id, entry1).await.unwrap();
        manager.add(profile_id, entry2).await.unwrap();

        let filtered = manager
            .filter(
                profile_id,
                EntryFilter {
                    start_date: None,
                    end_date: None,
                    task_id: Some("task1".to_string()),
                    tags: None,
                },
            )
            .await
            .unwrap();

        assert_eq!(filtered.len(), 1);
        assert_eq!(filtered[0].task_id, Some("task1".to_string()));
    }

    #[tokio::test]
    #[serial]
    async fn test_calculate_stats() {
        let start = Utc::now();
        let end1 = start + Duration::hours(1);
        let end2 = start + Duration::hours(2);

        let entry1 = Entry::create_completed(None, None, start, end1, TimerMode::Manual).unwrap();

        let entry2 = Entry::create_completed(None, None, start, end2, TimerMode::Pomodoro).unwrap();

        let stats = EntryManager::calculate_stats(&[entry1, entry2]);

        assert_eq!(stats.total_entries, 2);
        assert_eq!(stats.pomodoro_count, 1);
        assert_eq!(stats.manual_count, 1);
        assert_eq!(stats.total_duration_seconds, 3600 + 7200);
    }

    #[tokio::test]
    #[serial]
    async fn test_move_entries_for_task() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);
        let source_profile = "source_profile";
        let target_profile = "target_profile";
        let task_id = "task_to_move";

        let entry1 = Entry::new(
            Some(task_id.to_string()),
            Some("Task".to_string()),
            TimerMode::Manual,
        );
        let entry2 = Entry::new(
            Some(task_id.to_string()),
            Some("Task".to_string()),
            TimerMode::Manual,
        );
        let entry3 = Entry::new(
            Some("other_task".to_string()),
            Some("Other".to_string()),
            TimerMode::Manual,
        );

        manager.add(source_profile, entry1).await.unwrap();
        manager.add(source_profile, entry2).await.unwrap();
        manager.add(source_profile, entry3).await.unwrap();

        let moved_count = manager
            .move_entries_for_task(source_profile, target_profile, task_id)
            .await
            .unwrap();

        assert_eq!(moved_count, 2);

        let source_entries = manager.get_all(source_profile).await.unwrap();
        assert_eq!(source_entries.len(), 1);
        assert_eq!(source_entries[0].task_id, Some("other_task".to_string()));

        let target_entries = manager.get_all(target_profile).await.unwrap();
        assert_eq!(target_entries.len(), 2);
        assert!(
            target_entries
                .iter()
                .all(|e| e.task_id.as_deref() == Some(task_id))
        );
    }

    #[tokio::test]
    #[serial]
    async fn test_move_entries_for_task_no_entries() {
        let temp_dir = TempDir::new().unwrap();
        let manager = create_manager(&temp_dir);

        let moved_count = manager
            .move_entries_for_task("source", "target", "nonexistent_task")
            .await
            .unwrap();

        assert_eq!(moved_count, 0);
    }
}
