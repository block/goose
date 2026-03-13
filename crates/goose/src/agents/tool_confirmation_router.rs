use std::collections::HashMap;

use tokio::sync::{oneshot, Mutex};
use tracing::warn;

use crate::permission::PermissionConfirmation;

pub struct ToolConfirmationRouter {
    pending: Mutex<HashMap<String, oneshot::Sender<PermissionConfirmation>>>,
}

impl ToolConfirmationRouter {
    pub fn new() -> Self {
        Self {
            pending: Mutex::new(HashMap::new()),
        }
    }

    pub async fn register(&self, request_id: String) -> oneshot::Receiver<PermissionConfirmation> {
        let (tx, rx) = oneshot::channel();
        self.pending.lock().await.insert(request_id, tx);
        rx
    }

    pub async fn deliver(&self, request_id: String, confirmation: PermissionConfirmation) -> bool {
        if let Some(tx) = self.pending.lock().await.remove(&request_id) {
            if tx.send(confirmation).is_err() {
                warn!(
                    request_id = %request_id,
                    "Confirmation receiver was dropped (task cancelled)"
                );
                false
            } else {
                true
            }
        } else {
            warn!(
                request_id = %request_id,
                "No task waiting for confirmation"
            );
            false
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::permission::permission_confirmation::PrincipalType;
    use crate::permission::Permission;

    fn test_confirmation() -> PermissionConfirmation {
        PermissionConfirmation {
            principal_type: PrincipalType::Tool,
            permission: Permission::AllowOnce,
        }
    }

    #[tokio::test]
    async fn test_register_then_deliver() {
        let router = ToolConfirmationRouter::new();
        let rx = router.register("req_1".to_string()).await;
        assert!(
            router
                .deliver("req_1".to_string(), test_confirmation())
                .await
        );
        let confirmation = rx.await.unwrap();
        assert_eq!(confirmation.permission, Permission::AllowOnce);
    }

    #[tokio::test]
    async fn test_deliver_unknown_request() {
        let router = ToolConfirmationRouter::new();
        assert!(
            !router
                .deliver("unknown".to_string(), test_confirmation())
                .await
        );
    }

    #[tokio::test]
    async fn test_cancelled_receiver() {
        let router = ToolConfirmationRouter::new();
        let rx = router.register("req_1".to_string()).await;
        drop(rx); // simulate task cancellation
        assert!(
            !router
                .deliver("req_1".to_string(), test_confirmation())
                .await
        );
    }

    #[tokio::test]
    async fn test_concurrent_requests_out_of_order() {
        use std::sync::Arc;

        let router = Arc::new(ToolConfirmationRouter::new());

        // Register two requests
        let rx1 = router.register("req_1".to_string()).await;
        let rx2 = router.register("req_2".to_string()).await;

        // Deliver in reverse order
        assert!(
            router
                .deliver(
                    "req_2".to_string(),
                    PermissionConfirmation {
                        principal_type: PrincipalType::Tool,
                        permission: Permission::DenyOnce,
                    }
                )
                .await
        );
        assert_eq!(router.pending.lock().await.len(), 1);
        assert!(
            router
                .deliver("req_1".to_string(), test_confirmation())
                .await
        );
        assert_eq!(router.pending.lock().await.len(), 0);

        let c1 = rx1.await.unwrap();
        assert_eq!(c1.permission, Permission::AllowOnce);
        let c2 = rx2.await.unwrap();
        assert_eq!(c2.permission, Permission::DenyOnce);
    }
}
