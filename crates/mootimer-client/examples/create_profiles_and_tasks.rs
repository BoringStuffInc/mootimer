use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    // 1. Create Personal profile
    println!("Creating 'personal' profile...");
    let _ = client
        .profile_create("personal", "Personal", Some("Personal tasks and habits"))
        .await;

    // 2. Add tasks to Personal
    println!("Adding tasks to 'personal' profile...");
    client
        .task_create("personal", "Morning Exercise", Some("30 minutes of cardio"))
        .await?;
    client
        .task_create(
            "personal",
            "Read 20 pages",
            Some("Current book: Rust in Action"),
        )
        .await?;
    client
        .task_create("personal", "Meditate", Some("10 minutes focus"))
        .await?;

    // 3. Create TestConnection profile
    println!("Creating 'test-connection' profile...");
    let _ = client
        .profile_create(
            "test-connection",
            "Test Connection",
            Some("Verification profile"),
        )
        .await;

    println!("✓ Done!");
    Ok(())
}
