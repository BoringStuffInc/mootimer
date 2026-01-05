use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    println!("Starting a 5-second manual timer...");
    client.timer_start_manual("default", None).await?;

    println!("Waiting 5 seconds...");
    tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

    println!("Stopping timer...");
    client.timer_stop("default").await?;

    println!("Timer stopped! Waiting 1 second for entry to be saved...");
    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

    println!("\nFetching all entries...");
    let result = client.entry_today("default").await?;

    if let Some(entries) = result.as_array() {
        println!("✓ Now have {} entries (was 11)", entries.len());
        if let Some(last_entry) = entries.last() {
            let duration = last_entry
                .get("duration_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            println!("✓ Latest entry duration: {}s", duration);
        }
    }

    println!("\n✓ Test complete! Check ~/.local/share/mootimer/profiles/default/entries.csv");

    Ok(())
}
