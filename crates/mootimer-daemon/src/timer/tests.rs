#[cfg(test)]
mod countdown_completion_tests {
    use super::super::{TimerEngine, TimerManager};
    use mootimer_core::models::TimerMode;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_countdown_completion_removes_timer() {
        let manager = TimerManager::new();

        let timer_id = manager
            .start_countdown("profile1".to_string(), None, 1)
            .await
            .unwrap();

        sleep(Duration::from_millis(100)).await;

        assert!(manager.has_active_timer("profile1").await);


    }

    #[tokio::test]
    async fn test_countdown_tick_loop_doesnt_deadlock() {
        use tokio::sync::broadcast;

        let (tx, _) = broadcast::channel(100);
        let engine = super::super::engine::TimerEngine::new_countdown(
            "test_profile".to_string(),
            None,
            1,
            tx,
        );

        let engine_arc = std::sync::Arc::new(engine);
        let engine_clone = engine_arc.clone();

        let tick_task = tokio::spawn(async move {
            engine_clone.start_tick_loop().await;
        });

        sleep(Duration::from_millis(500)).await;

        let timer_result = tokio::time::timeout(
            Duration::from_secs(5),
            engine_arc.get_timer(),
        ).await;

        assert!(timer_result.is_ok(), "get_timer should not timeout/deadlock");

        let _  = tokio::time::timeout(
            Duration::from_secs(5),
            tick_task,
        ).await;
    }
}
