// Test to show entries with task names resolved
use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    println!("Fetching tasks...");
    let tasks = client.task_list("default").await?;

    println!("Fetching entries...");
    let entries = client.entry_today("default").await?;

    if let (Some(task_arr), Some(entry_arr)) = (tasks.as_array(), entries.as_array()) {
        println!("\n═══════════════════════════════════════════════════════");
        println!("  ENTRIES WITH TASK NAMES");
        println!("═══════════════════════════════════════════════════════\n");

        for (i, entry) in entry_arr.iter().enumerate() {
            let entry_id = entry
                .get("id")
                .and_then(|v| v.as_str())
                .map(|id| &id[..8])
                .unwrap_or("????????");

            let duration = entry
                .get("duration_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let mode = entry.get("mode").and_then(|v| v.as_str()).unwrap_or("?");
            let task_id = entry.get("task_id").and_then(|v| v.as_str());

            let task_display = if let Some(tid) = task_id {
                let task_name = task_arr
                    .iter()
                    .find(|t| t.get("id").and_then(|v| v.as_str()) == Some(tid))
                    .and_then(|t| t.get("title"))
                    .and_then(|v| v.as_str())
                    .unwrap_or("Unknown task");
                format!("{} [{}]", task_name, &tid[..8])
            } else {
                "No task".to_string()
            };

            let mode_icon = if mode == "pomodoro" { "🍅" } else { "⏱ " };
            let minutes = duration / 60;
            let seconds = duration % 60;

            println!(
                "  {} {} │ {:2}m {:2}s │ {:8} │ {}",
                mode_icon, entry_id, minutes, seconds, mode, task_display
            );
        }

        println!("\n═══════════════════════════════════════════════════════");
        println!("  Total: {} entries", entry_arr.len());
        println!("═══════════════════════════════════════════════════════\n");
    }

    Ok(())
}
