use crate::{Result, models::Task};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Serialize, Deserialize)]
struct TasksFile {
    tasks: Vec<Task>,
}

pub struct TaskStorage {
    data_dir: PathBuf,
}

impl TaskStorage {
    pub fn new(data_dir: PathBuf) -> Self {
        Self { data_dir }
    }

    pub fn load(&self, profile_id: &str) -> Result<Vec<Task>> {
        let tasks_path = self
            .data_dir
            .join("profiles")
            .join(profile_id)
            .join("tasks.json");

        if !tasks_path.exists() {
            return Ok(Vec::new());
        }

        let content = std::fs::read_to_string(tasks_path)?;
        let tasks_file: TasksFile = serde_json::from_str(&content)?;
        Ok(tasks_file.tasks)
    }

    pub fn save(&self, profile_id: &str, tasks: &[Task]) -> Result<()> {
        let profile_dir = self.data_dir.join("profiles").join(profile_id);
        std::fs::create_dir_all(&profile_dir)?;

        let tasks_path = profile_dir.join("tasks.json");
        let tasks_file = TasksFile {
            tasks: tasks.to_vec(),
        };
        let content = serde_json::to_string_pretty(&tasks_file)?;
        std::fs::write(tasks_path, content)?;

        Ok(())
    }
}
