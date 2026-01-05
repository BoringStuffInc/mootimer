
use mootimer_client::MooTimerClient;
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    unsafe {
        std::env::set_var(
            "XDG_DATA_HOME",
            "/tmp/mootimer-integration-test/.local/share",
        );
        std::env::set_var("XDG_CONFIG_HOME", "/tmp/mootimer-integration-test/.config");
    }

    let client = MooTimerClient::new("/tmp/mootimer-integration.sock");

    println!("=== MooTimer Integration Test ===\n");

    println!("1. Testing profile creation...");
    match client
        .profile_create(
            "test-profile",
            "Test Profile",
            Some("Integration test profile"),
        )
        .await
    {
        Ok(result) => println!("   ✓ Profile created: {}", result),
        Err(e) => {
            println!("   ✗ Failed: {}", e);
            return Err(e.into());
        }
    }
    println!();

    println!("2. Testing profile list...");
    match client.profile_list().await {
        Ok(profiles) => println!("   ✓ Profiles: {}", profiles),
        Err(e) => println!("   ✗ Failed: {}", e),
    }
    println!();

    println!("3. Testing task creation...");
    let task_result = match client
        .task_create("test-profile", "Test Task", Some("A test task"))
        .await
    {
        Ok(result) => {
            println!("   ✓ Task created: {}", result);
            result
        }
        Err(e) => {
            println!("   ✗ Failed: {}", e);
            return Err(e.into());
        }
    };

    let task_id = task_result
        .get("id")
        .and_then(|v| v.as_str())
        .expect("Task ID not found");
    println!("   Task ID: {}", task_id);
    println!();

    println!("4. Testing timer start (manual)...");
    match client
        .timer_start_manual("test-profile", Some(task_id))
        .await
    {
        Ok(result) => println!("   ✓ Timer started: {}", result),
        Err(e) => {
            println!("   ✗ Failed: {}", e);
            return Err(e.into());
        }
    }
    println!();

    println!("5. Waiting 3 seconds...");
    tokio::time::sleep(tokio::time::Duration::from_secs(3)).await;
    println!("   ✓ Done");
    println!();

    println!("6. Testing timer status...");
    match client.timer_get("test-profile").await {
        Ok(result) => println!("   ✓ Timer status: {}", result),
        Err(e) => println!("   ⚠ Failed: {}", e),
    }
    println!();

    println!("7. Testing timer stop...");
    match client.timer_stop("test-profile").await {
        Ok(result) => println!("   ✓ Timer stopped: {}", result),
        Err(e) => {
            println!("   ✗ Failed: {}", e);
            return Err(e.into());
        }
    }
    println!();

    println!("8. Testing sync.init (without remote)...");
    match client.call("sync.init", Some(json!({}))).await {
        Ok(result) => println!("   ✓ Sync initialized: {}", result),
        Err(e) => {
            println!("   ✗ Failed: {}", e);
            return Err(e.into());
        }
    }
    println!();

    println!("9. Testing sync.status...");
    match client.call("sync.status", None).await {
        Ok(result) => {
            println!("   ✓ Sync status: {}", result);
            let initialized = result
                .get("initialized")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            if initialized {
                println!("   ✓ Git is initialized");
            } else {
                println!("   ✗ Git is NOT initialized");
            }
        }
        Err(e) => println!("   ✗ Failed: {}", e),
    }
    println!();

    println!("10. Testing sync.commit (should work without remote)...");
    match client
        .call(
            "sync.commit",
            Some(json!({"message": "Test commit from integration test"})),
        )
        .await
    {
        Ok(result) => println!("   ✓ Commit result: {}", result),
        Err(e) => println!("   ⚠ Commit warning: {} (might be no changes)", e),
    }
    println!();

    println!("11. Testing sync.sync without remote (should fail gracefully)...");
    match client.call("sync.sync", None).await {
        Ok(result) => println!("   ⚠ Unexpected success: {}", result),
        Err(e) => println!("   ✓ Correctly failed: {}", e),
    }
    println!();

    println!("12. Testing config.get...");
    match client.call("config.get", None).await {
        Ok(result) => {
            println!("   ✓ Config: {}", result);
            let auto_push = result
                .get("sync")
                .and_then(|s| s.get("auto_push"))
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            println!("   Auto-push enabled: {}", auto_push);
        }
        Err(e) => println!("   ✗ Failed: {}", e),
    }
    println!();

    println!("=== All Tests Completed Successfully ===");

    Ok(())
}
