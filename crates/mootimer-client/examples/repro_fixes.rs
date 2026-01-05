use mootimer_client::MooTimerClient;
use serde_json::json;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    println!("Testing Pomodoro Start...");
    let result = client.timer_start_pomodoro("default", None, Some(25)).await;
    match result {
        Ok(res) => println!("✅ Pomodoro started successfully: {:?}", res),
        Err(e) => println!("❌ Pomodoro start failed: {}", e),
    }

    let _ = client.timer_stop("default").await;

    println!("\nTesting Entry Update...");
    let entries = client.entry_today("default").await?;
    if let Some(entry_array) = entries.as_array() {
        if let Some(entry) = entry_array.first() {
            let mut entry_clone = entry.clone();
            if let Some(obj) = entry_clone.as_object_mut() {
                obj.insert("description".to_string(), json!("Updated via script"));

                let update_res = client.entry_update("default", entry_clone).await;
                match update_res {
                    Ok(_) => println!("✅ Entry update successful"),
                    Err(e) => println!("❌ Entry update failed: {}", e),
                }
            }
        } else {
            println!("⚠️ No entries found to update. Please create an entry first.");
        }
    }

    println!("\nIf you see ✅ checks above, the daemon is up to date.");
    Ok(())
}
