use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    println!("Cleaning up test profiles...");

    match client.profile_delete("personal").await {
        Ok(_) => println!("✓ Deleted 'personal' profile"),
        Err(e) => println!("! Failed to delete 'personal': {}", e),
    }

    match client.profile_delete("test-connection").await {
        Ok(_) => println!("✓ Deleted 'test-connection' profile"),
        Err(e) => println!("! Failed to delete 'test-connection': {}", e),
    }

    println!("Cleanup finished.");
    Ok(())
}
