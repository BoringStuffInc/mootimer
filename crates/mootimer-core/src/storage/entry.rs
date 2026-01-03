//! Time entry storage operations (CSV format)

use crate::{
    Result,
    models::{Entry, TimerMode},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// CSV-friendly representation of an Entry
#[derive(Debug, Serialize, Deserialize)]
struct EntryCsv {
    id: String,
    task_id: String,
    start_time: String,
    end_time: String,
    duration_seconds: u64,
    mode: String,
    description: String,
    tags: String,
}

impl From<&Entry> for EntryCsv {
    fn from(entry: &Entry) -> Self {
        Self {
            id: entry.id.clone(),
            task_id: entry.task_id.clone().unwrap_or_default(),
            start_time: entry.start_time.to_rfc3339(),
            end_time: entry.end_time.map(|t| t.to_rfc3339()).unwrap_or_default(),
            duration_seconds: entry.duration_seconds,
            mode: match entry.mode {
                TimerMode::Manual => "manual".to_string(),
                TimerMode::Pomodoro => "pomodoro".to_string(),
                TimerMode::Countdown => "countdown".to_string(),
            },
            description: entry.description.clone().unwrap_or_default(),
            tags: entry.tags.join(","),
        }
    }
}

impl TryFrom<EntryCsv> for Entry {
    type Error = crate::Error;

    fn try_from(csv: EntryCsv) -> Result<Self> {
        Ok(Self {
            id: csv.id,
            task_id: if csv.task_id.is_empty() {
                None
            } else {
                Some(csv.task_id)
            },
            start_time: DateTime::parse_from_rfc3339(&csv.start_time)
                .map_err(|e| crate::Error::InvalidData(format!("Invalid start_time: {}", e)))?
                .with_timezone(&Utc),
            end_time: if csv.end_time.is_empty() {
                None
            } else {
                Some(
                    DateTime::parse_from_rfc3339(&csv.end_time)
                        .map_err(|e| crate::Error::InvalidData(format!("Invalid end_time: {}", e)))?
                        .with_timezone(&Utc),
                )
            },
            duration_seconds: csv.duration_seconds,
            mode: match csv.mode.as_str() {
                "manual" => TimerMode::Manual,
                "pomodoro" => TimerMode::Pomodoro,
                "countdown" => TimerMode::Countdown,
                _ => TimerMode::Manual,
            },
            description: if csv.description.is_empty() {
                None
            } else {
                Some(csv.description)
            },
            tags: if csv.tags.is_empty() {
                Vec::new()
            } else {
                csv.tags.split(',').map(|s| s.trim().to_string()).collect()
            },
        })
    }
}

pub struct EntryStorage {
    data_dir: PathBuf,
}

impl EntryStorage {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn load(&self, profile_id: &str) -> Result<Vec<Entry>> {
        let entries_path = self
            .data_dir
            .join("profiles")
            .join(profile_id)
            .join("entries.csv");

        if !entries_path.exists() {
            return Ok(Vec::new());
        }

        let mut reader = csv::Reader::from_path(entries_path)?;
        let mut entries = Vec::new();

        for result in reader.deserialize() {
            let entry_csv: EntryCsv = result?;
            let entry = Entry::try_from(entry_csv)?;
            entries.push(entry);
        }

        Ok(entries)
    }

    pub fn append(&self, profile_id: &str, entry: &Entry) -> Result<()> {
        let profile_dir = self.data_dir.join("profiles").join(profile_id);
        std::fs::create_dir_all(&profile_dir)?;

        let entries_path = profile_dir.join("entries.csv");
        let file_exists = entries_path.exists();

        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&entries_path)?;

        // When appending, don't write headers
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false) // Don't auto-write headers
            .from_writer(file);

        // Convert to CSV-friendly format
        let entry_csv = EntryCsv::from(entry);

        // Only write header for brand new file
        if !file_exists {
            writer.write_record([
                "id",
                "task_id",
                "start_time",
                "end_time",
                "duration_seconds",
                "mode",
                "description",
                "tags",
            ])?;
        }

        writer.serialize(&entry_csv)?;
        writer.flush()?;

        Ok(())
    }

    pub fn save_all(&self, profile_id: &str, entries: &[Entry]) -> Result<()> {
        let profile_dir = self.data_dir.join("profiles").join(profile_id);
        std::fs::create_dir_all(&profile_dir)?;

        let entries_path = profile_dir.join("entries.csv");
        let mut writer = csv::Writer::from_path(entries_path)?;

        for entry in entries {
            let entry_csv = EntryCsv::from(entry);
            writer.serialize(&entry_csv)?;
        }

        writer.flush()?;
        Ok(())
    }
}
