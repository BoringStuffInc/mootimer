/// Test countdown completion - verifies that timer.get doesn't hang when countdown completes
use mootimer_client::MooTimerClient;
use std::time::{Duration, Instant};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    println!("=== Countdown Completion Test ===\n");
    
    // Start a very short countdown (2 minutes) so it completes quickly
    println!("Starting 2-minute countdown timer...");
    let start = Instant::now();
    let result = client.timer_start_countdown("test_profile", None, 2).await?;
    println!("✓ Timer started: {:?}\n", result);

    // Poll the timer status every 0.5 seconds until it completes
    let mut prev_state = "running".to_string();
    let timeout = Duration::from_secs(130); // Give it 130 seconds (timeout for 2min + 10sec buffer)

    println!("Polling timer status every 0.5 seconds...");
    println!("(Testing that timer.get doesn't hang when countdown completes)\n");

    loop {
        tokio::time::sleep(Duration::from_millis(500)).await;
        let elapsed = start.elapsed();

        // Test that timer.get doesn't hang
        println!("[{:>3}s] Calling timer.get...", elapsed.as_secs());
        let timer_result = client.timer_get("test_profile").await;

        match timer_result {
            Ok(timer) => {
                let state = timer
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown")
                    .to_string();

                let elapsed_secs = timer
                    .get("elapsed_seconds")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let target = timer
                    .get("target_duration")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let remaining = if target > elapsed_secs {
                    target - elapsed_secs
                } else {
                    0
                };

                print!("        state={}", state);
                print!(", elapsed={}", elapsed_secs);
                print!(", remaining={}s", remaining);

                if state != prev_state {
                    println!(" <- STATE CHANGED");
                    prev_state = state.clone();
                } else {
                    println!();
                }

                // Check if timer is done
                if state == "stopped" || state == "completed" {
                    println!("\n✓ Timer completed successfully!");
                    println!("  Total elapsed real time: {:?}", elapsed);
                    println!("  Timer elapsed: {}s", elapsed_secs);
                    break;
                }

                // Check if timer was auto-removed (countdown feature)
                if timer.is_null() || state == "not_found" {
                    println!("\n✓ Timer auto-removed after completion!");
                    println!("  Total elapsed real time: {:?}", elapsed);
                    break;
                }
            }
            Err(e) => {
                // If timer.get returns an error for "not found", that's OK - it means it was auto-completed
                if e.to_string().contains("not found") || e.to_string().contains("NotFound") {
                    println!("        NotFound (auto-completed) ✓");
                    println!("\n✓ Timer auto-removed after completion!");
                    println!("  Total elapsed real time: {:?}", elapsed);
                    break;
                } else {
                    println!("        ERROR: {}", e);
                    return Err(e);
                }
            }
        }

        if elapsed > timeout {
            println!("\n✗ Test timeout - timer didn't complete within {} seconds", timeout.as_secs());
            return Err(anyhow::anyhow!("Test timeout"));
        }
    }

    println!("\n=== Test Passed ===");
    println!("No hangs detected during countdown completion!");

    Ok(())
}
