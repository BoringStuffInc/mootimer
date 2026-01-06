use mootimer_client::MooTimerClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = MooTimerClient::new("/tmp/mootimer.sock");

    println!("Testing persistent connection...");

    // First call will establish connection
    let profiles = client.profile_list().await?;
    println!(
        "✓ Call 1 (connect): Found {} profiles",
        profiles.as_array().map(|a| a.len()).unwrap_or(0)
    );

    // Second call should reuse connection
    let profiles_2 = client.profile_list().await?;
    println!(
        "✓ Call 2 (reuse): Found {} profiles",
        profiles_2.as_array().map(|a| a.len()).unwrap_or(0)
    );

    println!("Test successful!");
    Ok(())
}
