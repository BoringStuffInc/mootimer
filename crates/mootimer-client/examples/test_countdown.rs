// Test countdown timer
use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    println!("Starting 1-minute countdown timer...");
    let result = client.timer_start_countdown("default", None, 1).await?;
    println!("✓ Started: {:?}", result);

    // Wait a bit and check status
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    println!("\nChecking timer status after 2 seconds...");
    let timer = client.timer_get("default").await?;
    println!("Timer state: {:#?}", timer);

    if let Some(mode) = timer.get("mode").and_then(|v| v.as_str()) {
        if mode == "countdown" {
            println!("✓ Mode is countdown!");

            let elapsed = timer
                .get("elapsed_seconds")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let target = timer
                .get("target_duration")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            let remaining = if target > elapsed {
                target - elapsed
            } else {
                0
            };

            println!("  Elapsed: {}s", elapsed);
            println!("  Target: {}s ({}m)", target, target / 60);
            println!("  Remaining: {}s", remaining);
        }
    }

    println!("\nLet the timer run to completion (58s)...");
    println!("(The daemon will auto-stop when it reaches 60 seconds)");

    Ok(())
}
