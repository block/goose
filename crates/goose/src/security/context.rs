use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone)]
pub struct SecurityContext {
    /// Maps tool request IDs to security finding IDs (e.g., "toolu_123" -> "SEC-001")
    finding_map: Arc<RwLock<HashMap<String, String>>>,
}

impl SecurityContext {
    /// Create a new SecurityContext
    pub fn new() -> Self {
        Self {
            finding_map: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Store a mapping between a tool request ID and a security finding ID
    pub async fn store_finding_id(&self, request_id: &str, finding_id: &str) {
        let mut map = self.finding_map.write().await;
        map.insert(request_id.to_string(), finding_id.to_string());
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

        let finding_id1 = "SEC-001".to_string();
        let finding_id2 = "SEC-002".to_string();

        // Test storing and retrieving mappings
        context.store_finding_id("toolu_123", &finding_id1).await;
        context.store_finding_id("toolu_456", &finding_id2).await;

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
            context1.store_finding_id("concurrent_1", "SEC-100").await;
        });

        let handle2 = tokio::spawn(async move {
            context2.store_finding_id("concurrent_2", "SEC-200").await;
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
