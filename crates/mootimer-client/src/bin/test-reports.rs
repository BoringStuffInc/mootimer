use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    std::env::set_var("XDG_DATA_HOME", "/tmp/mootimer-report-test/.local/share");
    std::env::set_var("XDG_CONFIG_HOME", "/tmp/mootimer-report-test/.config");

    let client = MooTimerClient::new("/tmp/mootimer-report.sock");

    println!("=== Creating Test Data for Reports ===\n");

    // Create profile
    println!("1. Creating profile...");
    client.profile_create("test", "Test Profile", None).await?;
    println!("   ✓ Profile created\n");

    // Create multiple tasks
    println!("2. Creating tasks...");
    let task1 = client
        .task_create("test", "Frontend Development", Some("React components"))
        .await?;
    let task1_id = task1.get("id").and_then(|v| v.as_str()).unwrap();
    println!("   ✓ Task 1: Frontend Development ({})", task1_id);

    let task2 = client
        .task_create("test", "Backend API", Some("REST endpoints"))
        .await?;
    let task2_id = task2.get("id").and_then(|v| v.as_str()).unwrap();
    println!("   ✓ Task 2: Backend API ({})", task2_id);

    let task3 = client
        .task_create("test", "Documentation", Some("Update README"))
        .await?;
    let task3_id = task3.get("id").and_then(|v| v.as_str()).unwrap();
    println!("   ✓ Task 3: Documentation ({})\n", task3_id);

    // Create multiple timer sessions
    println!("3. Creating timer sessions...\n");

    // Session 1: Frontend - 30 min
    println!("   Session 1: Frontend Development (30 min)...");
    client.timer_start_manual("test", Some(task1_id)).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    client.timer_stop("test").await?;
    println!("   ✓ Completed\n");

    // Session 2: Backend - 45 min (pomodoro)
    println!("   Session 2: Backend API (pomodoro)...");
    client
        .timer_start_pomodoro("test", Some(task2_id), None)
        .await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    client.timer_stop("test").await?;
    println!("   ✓ Completed\n");

    // Session 3: Frontend again - 20 min
    println!("   Session 3: Frontend Development (20 min)...");
    client.timer_start_manual("test", Some(task1_id)).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    client.timer_stop("test").await?;
    println!("   ✓ Completed\n");

    // Session 4: Documentation - 15 min
    println!("   Session 4: Documentation (15 min)...");
    client.timer_start_manual("test", Some(task3_id)).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    client.timer_stop("test").await?;
    println!("   ✓ Completed\n");

    // Session 5: Backend again - 30 min
    println!("   Session 5: Backend API (30 min)...");
    client.timer_start_manual("test", Some(task2_id)).await?;
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;
    client.timer_stop("test").await?;
    println!("   ✓ Completed\n");

    // Get today's stats
    println!("4. Checking today's stats...");
    let stats = client.entry_stats_today("test").await?;
    println!("   ✓ Stats: {}\n", stats);

    // Get today's entries
    println!("5. Checking today's entries...");
    let entries = client.entry_today("test").await?;
    if let Some(entries_arr) = entries.as_array() {
        println!("   ✓ Found {} entries", entries_arr.len());
        for (i, entry) in entries_arr.iter().enumerate() {
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
                "     Entry {}: task={}, duration={}s, mode={}",
                i + 1,
                task_id,
                duration,
                mode
            );
        }
    }

    println!("\n=== Test Data Created Successfully ===");
    println!("\nNow you can:");
    println!("  1. Run the TUI: ./target/release/mootimer --socket /tmp/mootimer-report.sock --profile test");
    println!("  2. Navigate to Reports view (press '4')");
    println!("  3. See the task breakdown!\n");

    Ok(())
}
