// Simple test to verify entry loading
use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    println!("Fetching today's entries for 'default' profile...");
    let result = client.entry_today("default").await?;

    if let Some(entries) = result.as_array() {
        println!("✓ Successfully loaded {} entries", entries.len());
        for (i, entry) in entries.iter().enumerate() {
            let id = entry.get("id").and_then(|v| v.as_str()).unwrap_or("?");
            let duration = entry
                .get("duration_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let mode = entry.get("mode").and_then(|v| v.as_str()).unwrap_or("?");
            let task_id = entry.get("task_id").and_then(|v| v.as_str()).unwrap_or("");

            println!(
                "  Entry {}: ID={}, Duration={}s, Mode={}, Task={}",
                i + 1,
                &id[..8], // Show first 8 chars of UUID
                duration,
                mode,
                if task_id.is_empty() {
                    "none"
                } else {
                    &task_id[..8]
                }
            );
        }
    } else {
        println!("✗ Result is not an array: {:?}", result);
    }

    Ok(())
}
