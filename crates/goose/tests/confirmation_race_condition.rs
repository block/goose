/// Tests for https://github.com/block/goose/issues/5558
///
/// Part 1: Reproduces the bug with the old Mutex<mpsc::Receiver> pattern.
/// Part 2: Verifies the fix using ConfirmationRouter (oneshot + HashMap).
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::{mpsc, Mutex};

// ============================================================
// Part 1: Bug reproduction (old pattern)
// ============================================================

/// Mirrors the OLD buggy pattern from tool_execution.rs:
/// Lock shared receiver, scan for matching ID, discard non-matches.
async fn wait_for_confirmation_buggy(
    rx: &Mutex<mpsc::Receiver<(String, String)>>,
    my_request_id: &str,
) -> Option<String> {
    let mut rx = rx.lock().await;
    while let Some((req_id, value)) = rx.recv().await {
        if req_id == my_request_id {
            return Some(value);
        }
        // Bug: non-matching confirmation is dropped
    }
    None
}

/// Two concurrent tasks share one Mutex<Receiver>. Out-of-order confirmations
/// cause Task 2 to hang forever.
#[tokio::test]
async fn test_old_pattern_out_of_order_causes_hang() {
    let (tx, rx) = mpsc::channel::<(String, String)>(32);
    let rx = Arc::new(Mutex::new(rx));

    let rx1 = rx.clone();
    let task1 = tokio::spawn(async move {
        wait_for_confirmation_buggy(&rx1, "request_1").await
    });

    tokio::time::sleep(Duration::from_millis(50)).await;

    let rx2 = rx.clone();
    let task2 = tokio::spawn(async move {
        wait_for_confirmation_buggy(&rx2, "request_2").await
    });

    // Send in WRONG order: request_2 first, then request_1
    tx.send(("request_2".to_string(), "confirmed_2".to_string())).await.unwrap();
    tx.send(("request_1".to_string(), "confirmed_1".to_string())).await.unwrap();

    let result1 = tokio::time::timeout(Duration::from_secs(2), task1).await;
    assert!(result1.is_ok(), "Task 1 should complete");

    let result2 = tokio::time::timeout(Duration::from_secs(2), task2).await;
    assert!(
        result2.is_err(),
        "BUG: Task 2 hangs because its confirmation was discarded by Task 1"
    );
}

// ============================================================
// Part 2: Fix verification (ConfirmationRouter pattern)
// ============================================================

use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};

/// Two concurrent tasks with out-of-order confirmations — both complete.
#[tokio::test]
async fn test_router_out_of_order_both_complete() {
    let agent = goose::agents::Agent::new();

    // Register both requests (simulates what tool_execution.rs does before yield)
    let rx1 = agent.confirmation_router.register("request_1".to_string()).await;
    let rx2 = agent.confirmation_router.register("request_2".to_string()).await;

    // Deliver in WRONG order: request_2 first, then request_1
    agent
        .handle_confirmation(
            "request_2".to_string(),
            PermissionConfirmation {
                principal_type: PrincipalType::Tool,
                permission: Permission::AllowOnce,
            },
        )
        .await;
    agent
        .handle_confirmation(
            "request_1".to_string(),
            PermissionConfirmation {
                principal_type: PrincipalType::Tool,
                permission: Permission::DenyOnce,
            },
        )
        .await;

    // Both tasks receive their correct confirmation — no hang
    let c1 = tokio::time::timeout(Duration::from_secs(1), rx1).await;
    assert!(c1.is_ok(), "Task 1 should not hang");
    assert_eq!(c1.unwrap().unwrap().permission, Permission::DenyOnce);

    let c2 = tokio::time::timeout(Duration::from_secs(1), rx2).await;
    assert!(c2.is_ok(), "Task 2 should not hang");
    assert_eq!(c2.unwrap().unwrap().permission, Permission::AllowOnce);
}

/// 5 concurrent requests with reverse-order confirmations — all complete.
#[tokio::test]
async fn test_router_five_concurrent_all_complete() {
    let agent = goose::agents::Agent::new();
    let num_tasks = 5;

    // Register all requests
    let mut receivers = Vec::new();
    for i in 0..num_tasks {
        let rx = agent
            .confirmation_router
            .register(format!("request_{}", i))
            .await;
        receivers.push(rx);
    }

    // Deliver in REVERSE order
    for i in (0..num_tasks).rev() {
        agent
            .handle_confirmation(
                format!("request_{}", i),
                PermissionConfirmation {
                    principal_type: PrincipalType::Tool,
                    permission: Permission::AllowOnce,
                },
            )
            .await;
    }

    // All tasks receive their confirmation — no hang
    for (i, rx) in receivers.into_iter().enumerate() {
        let result = tokio::time::timeout(Duration::from_secs(1), rx).await;
        assert!(result.is_ok(), "Task {} should not hang", i);
    }
}
