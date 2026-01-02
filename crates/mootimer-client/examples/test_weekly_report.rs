// Test weekly report data loading
use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    println!("Fetching weekly stats...");
    let stats = client
        .call(
            "entry.stats_week",
            Some(serde_json::json!({"profile_id": "default"})),
        )
        .await?;

    println!("Weekly Stats: {:#?}", stats);

    println!("\nFetching weekly entries...");
    let entries = client
        .call(
            "entry.week",
            Some(serde_json::json!({"profile_id": "default"})),
        )
        .await?;

    if let Some(entry_arr) = entries.as_array() {
        println!("✓ Found {} entries for this week", entry_arr.len());

        // Show first 3 entries
        for (i, entry) in entry_arr.iter().take(3).enumerate() {
            let duration = entry
                .get("duration_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let task_id = entry
                .get("task_id")
                .and_then(|v| v.as_str())
                .unwrap_or("none");
            println!(
                "  Entry {}: {}s, task={}",
                i + 1,
                duration,
                if task_id == "none" {
                    "No task".to_string()
                } else {
                    task_id[..8].to_string()
                }
            );
        }
    } else {
        println!("✗ Entries is not an array: {:?}", entries);
    }

    Ok(())
}
