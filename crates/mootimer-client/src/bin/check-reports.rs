use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        std::env::set_var("XDG_DATA_HOME", "/tmp/mootimer-report-test/.local/share");
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/mootimer-report-test/.config");
    }

    let client = MooTimerClient::new("/tmp/mootimer-report.sock");

    println!("=== Checking Reports Data ===\n");

    // Get tasks
    println!("Tasks:");
    let tasks = client.task_list("test").await?;
    if let Some(tasks_arr) = tasks.as_array() {
        for task in tasks_arr {
            let title = task
                .get("title")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let id = task.get("id").and_then(|v| v.as_str()).unwrap_or("unknown");
            println!("  - {}: {}", title, id);
        }
    }

    println!("\nToday's Stats:");
    let stats = client.entry_stats_today("test").await?;
    println!("{:#?}", stats);

    println!("\nToday's Entries:");
    let entries = client.entry_today("test").await?;
    if let Some(entries_arr) = entries.as_array() {
        for entry in entries_arr {
            let task_id = entry
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            let duration = entry
                .get("duration_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let mode = entry
                .get("mode")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");
            println!(
                "  Task: {}, Duration: {}s, Mode: {}",
                task_id, duration, mode
            );
        }
    }

    println!("\n=== All data looks good! ===");

    Ok(())
}
