use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Maps tool request IDs to security finding IDs (e.g., "toolu_123" -> "SEC-001")
    finding_map: Arc<RwLock<HashMap<String, String>>>,
    /// Counter for generating unique finding IDs
    finding_counter: Arc<RwLock<u32>>,
}

impl SecurityContext {
    /// Create a new SecurityContext
    pub fn new() -> Self {
        Self {
            finding_map: Arc::new(RwLock::new(HashMap::new())),
            finding_counter: Arc::new(RwLock::new(1)),
        }
    }

    /// Generate a new security finding ID in SEC-XXX format
    pub async fn generate_finding_id(&self) -> String {
        let mut counter = self.finding_counter.write().await;
        let id = format!("SEC-{:03}", *counter);
        *counter = counter.saturating_add(1);
        id
    }

    /// Store a mapping between a tool request ID and a security finding ID
    pub async fn store_finding_id(&self, request_id: String, finding_id: String) {
        let mut map = self.finding_map.write().await;
        map.insert(request_id, finding_id);
    }

    /// Retrieve the security finding ID for a given tool request ID
    pub async fn get_finding_id(&self, request_id: &str) -> Option<String> {
        let map = self.finding_map.read().await;
        map.get(request_id).cloned()
    }
}

impl Default for SecurityContext {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_security_context_basic_operations() {
        let context = SecurityContext::new();

        // Test generating finding IDs
        let finding_id1 = context.generate_finding_id().await;
        let finding_id2 = context.generate_finding_id().await;

        assert_eq!(finding_id1, "SEC-001");
        assert_eq!(finding_id2, "SEC-002");

        // Test storing and retrieving mappings
        context
            .store_finding_id("toolu_123".to_string(), finding_id1.clone())
            .await;
        context
            .store_finding_id("toolu_456".to_string(), finding_id2.clone())
            .await;

        assert_eq!(context.get_finding_id("toolu_123").await, Some(finding_id1));
        assert_eq!(context.get_finding_id("toolu_456").await, Some(finding_id2));
        assert_eq!(context.get_finding_id("nonexistent").await, None);
    }

    #[tokio::test]
    async fn test_security_context_concurrent_access() {
        let context = SecurityContext::new();
        let context1 = context.clone();
        let context2 = context.clone();

        // Test concurrent access
        let handle1 = tokio::spawn(async move {
            context1
                .store_finding_id("concurrent_1".to_string(), "SEC-100".to_string())
                .await;
        });

        let handle2 = tokio::spawn(async move {
            context2
                .store_finding_id("concurrent_2".to_string(), "SEC-200".to_string())
                .await;
        });

        handle1.await.unwrap();
        handle2.await.unwrap();

        // Verify both were stored successfully in the same context
        assert_eq!(
            context.get_finding_id("concurrent_1").await,
            Some("SEC-100".to_string())
        );
        assert_eq!(
            context.get_finding_id("concurrent_2").await,
            Some("SEC-200".to_string())
        );
    }
}
