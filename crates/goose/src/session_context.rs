//! Session context module for propagating session IDs across async tasks.
//!
//! This module provides task-local storage for the current session ID,
//! allowing it to be propagated to LLM providers and MCP servers without
//! modifying function signatures.

use tokio::task_local;

task_local! {
    /// Task-local storage for the current session ID
    pub static SESSION_ID: Option<String>;
}

/// Execute a future with a session ID set in the task-local context.
///
/// If `session_id` is `Some`, the future will run with that session ID available
/// via `current_session_id()`. If `session_id` is `None`, the future runs normally
/// without a session ID in context.
///
/// # Example
///
/// ```ignore
/// use goose::session_context;
///
/// session_context::with_session_id(Some("session-123".to_string()), async {
///     // Session ID is available here
///     assert_eq!(session_context::current_session_id(), Some("session-123".to_string()));
/// }).await;
/// ```
pub async fn with_session_id<F>(session_id: Option<String>, f: F) -> F::Output
where
    F: std::future::Future,
{
    if let Some(id) = session_id {
        SESSION_ID.scope(Some(id), f).await
    } else {
        f.await
    }
}

/// Get the current session ID from task-local storage.
///
/// Returns `Some(session_id)` if a session ID is set in the current task context,
/// or `None` if no session ID is available.
///
/// # Example
///
/// ```ignore
/// use goose::session_context;
///
/// if let Some(id) = session_context::current_session_id() {
///     println!("Current session: {}", id);
/// }
/// ```
pub fn current_session_id() -> Option<String> {
    SESSION_ID.try_with(|id| id.clone()).ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_session_id_available_when_set() {
        with_session_id(Some("test-session-123".to_string()), async {
            assert_eq!(
                current_session_id(),
                Some("test-session-123".to_string()),
                "Session ID should be available in context"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn test_session_id_none_when_not_set() {
        let id = current_session_id();
        assert_eq!(id, None, "Session ID should be None when not set");
    }

    #[tokio::test]
    async fn test_session_id_none_when_explicitly_none() {
        with_session_id(None, async {
            assert_eq!(
                current_session_id(),
                None,
                "Session ID should be None when set to None"
            );
        })
        .await;
    }

    #[tokio::test]
    async fn test_session_id_scoped_correctly() {
        // Outer scope without session
        assert_eq!(current_session_id(), None);

        with_session_id(Some("outer-session".to_string()), async {
            assert_eq!(current_session_id(), Some("outer-session".to_string()));

            // Inner scope with different session
            with_session_id(Some("inner-session".to_string()), async {
                assert_eq!(
                    current_session_id(),
                    Some("inner-session".to_string()),
                    "Inner session should override outer"
                );
            })
            .await;

            // Back to outer scope
            assert_eq!(
                current_session_id(),
                Some("outer-session".to_string()),
                "Should return to outer session after inner completes"
            );
        })
        .await;

        // Back to no session
        assert_eq!(current_session_id(), None);
    }

    #[tokio::test]
    async fn test_session_id_across_await_points() {
        with_session_id(Some("persistent-session".to_string()), async {
            assert_eq!(current_session_id(), Some("persistent-session".to_string()));

            // Simulate async work
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;

            // Session should still be available after await
            assert_eq!(
                current_session_id(),
                Some("persistent-session".to_string()),
                "Session ID should persist across await points"
            );
        })
        .await;
    }
}