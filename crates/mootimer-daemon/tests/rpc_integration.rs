use anyhow::Result;
use mootimer_client::MooTimerClient;
use mootimer_daemon::{
    ApiHandler, ConfigManager, EntryManager, EventManager, IpcServer, ProfileManager, SyncManager,
    TaskManager, TimerManager,
};
use std::sync::Arc;
use tempfile::TempDir;

#[tokio::test]
async fn test_rpc_crud_operations() -> Result<()> {
    let temp_dir = TempDir::new()?;
    unsafe {
        std::env::set_var("HOME", temp_dir.path());
        std::env::set_var("XDG_DATA_HOME", temp_dir.path().join("data"));
        std::env::set_var("XDG_CONFIG_HOME", temp_dir.path().join("config"));
    }

    let socket_path = temp_dir.path().join("mootimer_test.sock");
    let socket_str = socket_path.to_string_lossy().to_string();

    let event_manager = Arc::new(EventManager::new());
    let timer_manager = Arc::new(TimerManager::new(event_manager.clone()));
    let profile_manager = Arc::new(ProfileManager::new(event_manager.clone())?);
    let task_manager = Arc::new(TaskManager::new(event_manager.clone())?);
    let entry_manager = Arc::new(EntryManager::new(event_manager.clone())?);
    let config_manager = Arc::new(ConfigManager::new()?);
    let sync_manager = Arc::new(SyncManager::new()?);

    let api_handler = Arc::new(ApiHandler::new(
        event_manager,
        timer_manager,
        profile_manager,
        task_manager,
        entry_manager,
        config_manager,
        sync_manager,
    ));

    let ipc_server = Arc::new(IpcServer::new(socket_str.clone(), api_handler));

    let server_handle = tokio::spawn(async move {
        ipc_server.start().await.unwrap();
    });

    let mut retries = 0;
    while !socket_path.exists() && retries < 50 {
        tokio::time::sleep(std::time::Duration::from_millis(50)).await;
        retries += 1;
    }
    if !socket_path.exists() {
        panic!("Socket was not created");
    }

    let client = MooTimerClient::new(&socket_str);

    println!("Testing Profile...");
    let p_res = client
        .profile_create("test-profile", "Test Profile", None)
        .await?;
    assert_eq!(p_res["id"], "test-profile");
    assert_eq!(p_res["name"], "Test Profile");

    let profiles = client.profile_list().await?;
    let profiles_arr = profiles.as_array().expect("Profiles should be array");
    assert!(profiles_arr.iter().any(|p| p["id"] == "test-profile"));

    println!("Testing Task...");
    let t_res = client
        .task_create("test-profile", "Delete Me Task", None)
        .await?;
    let task_id = t_res["id"]
        .as_str()
        .expect("Task ID should be string")
        .to_string();

    let task = client.task_get("test-profile", &task_id).await?;
    assert_eq!(task["title"], "Delete Me Task");

    client.task_delete("test-profile", &task_id).await?;

    let get_res = client.task_get("test-profile", &task_id).await;
    assert!(get_res.is_err(), "Task should be deleted");

    println!("Testing Entry...");
    let start_res = client.timer_start_manual("test-profile", None).await?;
    let timer_id = start_res["timer_id"]
        .as_str()
        .expect("Timer ID should be string")
        .to_string();
    tokio::time::sleep(std::time::Duration::from_millis(100)).await;
    let stop_res = client.timer_stop(&timer_id).await?;
    let entry_id = stop_res["id"].as_str().expect("Entry ID").to_string();

    let entries = client.entry_list("test-profile").await?;
    assert!(
        entries
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == entry_id)
    );

    client.entry_delete("test-profile", &entry_id).await?;

    let entries_after = client.entry_list("test-profile").await?;
    assert!(
        !entries_after
            .as_array()
            .unwrap()
            .iter()
            .any(|e| e["id"] == entry_id)
    );

    println!("Testing Profile Delete...");
    client.profile_delete("test-profile").await?;

    let profiles_after = client.profile_list().await?;
    assert!(
        !profiles_after
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["id"] == "test-profile")
    );

    println!("All integration tests passed!");

    server_handle.abort();

    Ok(())
}
