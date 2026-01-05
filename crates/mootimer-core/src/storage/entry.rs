
use crate::{
    Result,
    models::{Entry, TimerMode},
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct EntryCsv {
    id: String,
    task_id: String,
    #[serde(default)]
    task_title: String,
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
            task_title: entry.task_title.clone().unwrap_or_default(),
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
            task_title: if csv.task_title.is_empty() {
                None
            } else {
                Some(csv.task_title)
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
        self.migrate(profile_id)?;

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

    fn migrate(&self, profile_id: &str) -> Result<()> {
        let entries_path = self
            .data_dir
            .join("profiles")
            .join(profile_id)
            .join("entries.csv");

        if !entries_path.exists() {
            return Ok(());
        }

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(&entries_path)?;

        if let Some(result) = rdr.records().next() {
            let record = result?;
            if record.iter().any(|f| f == "task_title") {
                return Ok(());
            }
        } else {
            return Ok(());
        }

        let backup_path = entries_path.with_extension("csv.bak");
        std::fs::rename(&entries_path, &backup_path)?;

        let mut rdr = csv::ReaderBuilder::new()
            .has_headers(false)
            .flexible(true)
            .from_path(&backup_path)?;

        let mut wtr = csv::Writer::from_path(&entries_path)?;

        wtr.write_record(&[
            "id",
            "task_id",
            "task_title",
            "start_time",
            "end_time",
            "duration_seconds",
            "mode",
            "description",
            "tags",
        ])?;

        let mut records = rdr.records();
        if let Some(_) = records.next() {
        }

        for result in records {
            let record = result?;
            if record.len() >= 9 {
                wtr.write_record(&record)?;
            } else if record.len() == 8 {
                let mut new_record = Vec::new();
                new_record.push(record[0].to_string());
                new_record.push(record[1].to_string());
                new_record.push("".to_string());
                for i in 2..8 {
                    new_record.push(record[i].to_string());
                }
                wtr.write_record(&new_record)?;
            }
        }

        wtr.flush()?;
        Ok(())
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

        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .from_writer(file);

        let entry_csv = EntryCsv::from(entry);

        if !file_exists {
            writer.write_record([
                "id",
                "task_id",
                "task_title",
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
