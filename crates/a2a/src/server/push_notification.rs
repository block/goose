//! Push notification store, sender, and configuration management.

use std::collections::HashMap;
use std::sync::Arc;

use async_trait::async_trait;
use tokio::sync::RwLock;

use crate::error::A2AError;
use crate::types::config::{
    AuthenticationInfo, PushNotificationConfig, TaskPushNotificationConfig,
};

// ---------------------------------------------------------------------------
// PushNotificationStore trait
// ---------------------------------------------------------------------------

/// Storage for push notification configurations, keyed by (task_id, config_id).
#[async_trait]
pub trait PushNotificationStore: Send + Sync {
    async fn save(
        &self,
        task_id: &str,
        config: PushNotificationConfig,
    ) -> Result<TaskPushNotificationConfig, A2AError>;

    async fn load(
        &self,
        task_id: &str,
        config_id: &str,
    ) -> Result<Option<TaskPushNotificationConfig>, A2AError>;

    async fn list(&self, task_id: &str) -> Result<Vec<TaskPushNotificationConfig>, A2AError>;

    async fn delete(&self, task_id: &str, config_id: &str) -> Result<bool, A2AError>;

    /// Return all configs for a task (used by sender to fan-out notifications).
    async fn configs_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<PushNotificationConfig>, A2AError>;
}

// ---------------------------------------------------------------------------
// InMemoryPushNotificationStore
// ---------------------------------------------------------------------------

/// In-memory implementation of `PushNotificationStore`.
#[derive(Clone, Default)]
pub struct InMemoryPushNotificationStore {
    // task_id -> (config_id -> config)
    inner: Arc<RwLock<HashMap<String, HashMap<String, PushNotificationConfig>>>>,
}

#[async_trait]
impl PushNotificationStore for InMemoryPushNotificationStore {
    async fn save(
        &self,
        task_id: &str,
        config: PushNotificationConfig,
    ) -> Result<TaskPushNotificationConfig, A2AError> {
        let config_id = config
            .id
            .clone()
            .unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut config = config;
        config.id = Some(config_id.clone());

        let mut store = self.inner.write().await;
        store
            .entry(task_id.to_string())
            .or_default()
            .insert(config_id, config.clone());

        Ok(TaskPushNotificationConfig {
            task_id: task_id.to_string(),
            config,
        })
    }

    async fn load(
        &self,
        task_id: &str,
        config_id: &str,
    ) -> Result<Option<TaskPushNotificationConfig>, A2AError> {
        let store = self.inner.read().await;
        Ok(store
            .get(task_id)
            .and_then(|configs| configs.get(config_id))
            .map(|config| TaskPushNotificationConfig {
                task_id: task_id.to_string(),
                config: config.clone(),
            }))
    }

    async fn list(&self, task_id: &str) -> Result<Vec<TaskPushNotificationConfig>, A2AError> {
        let store = self.inner.read().await;
        Ok(store
            .get(task_id)
            .map(|configs| {
                configs
                    .values()
                    .map(|config| TaskPushNotificationConfig {
                        task_id: task_id.to_string(),
                        config: config.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default())
    }

    async fn delete(&self, task_id: &str, config_id: &str) -> Result<bool, A2AError> {
        let mut store = self.inner.write().await;
        Ok(store
            .get_mut(task_id)
            .map(|configs| configs.remove(config_id).is_some())
            .unwrap_or(false))
    }

    async fn configs_for_task(
        &self,
        task_id: &str,
    ) -> Result<Vec<PushNotificationConfig>, A2AError> {
        let store = self.inner.read().await;
        Ok(store
            .get(task_id)
            .map(|configs| configs.values().cloned().collect())
            .unwrap_or_default())
    }
}

// ---------------------------------------------------------------------------
// PushNotificationSender
// ---------------------------------------------------------------------------

/// Delivers push notifications to configured webhook URLs.
#[derive(Clone)]
pub struct PushNotificationSender {
    client: reqwest::Client,
}

impl Default for PushNotificationSender {
    fn default() -> Self {
        Self::new()
    }
}

impl PushNotificationSender {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    /// Send a push notification payload to all configured endpoints for a task.
    pub async fn send_notification<P: PushNotificationStore>(
        &self,
        store: &P,
        task_id: &str,
        payload: &serde_json::Value,
    ) -> Vec<Result<(), PushNotificationError>> {
        let configs = match store.configs_for_task(task_id).await {
            Ok(c) => c,
            Err(e) => {
                return vec![Err(PushNotificationError::StoreError(e.to_string()))];
            }
        };

        let mut results = Vec::with_capacity(configs.len());
        for config in &configs {
            results.push(self.send_to_endpoint(config, payload).await);
        }
        results
    }

    async fn send_to_endpoint(
        &self,
        config: &PushNotificationConfig,
        payload: &serde_json::Value,
    ) -> Result<(), PushNotificationError> {
        let mut request = self.client.post(&config.url).json(payload);

        // Apply authentication if configured
        if let Some(ref auth) = config.authentication {
            request = apply_auth(request, auth);
        }

        // Apply token as Bearer if present (shorthand for simple auth)
        if let Some(ref token) = config.token {
            request = request.bearer_auth(token);
        }

        let response = request
            .send()
            .await
            .map_err(|e| PushNotificationError::HttpError(e.to_string()))?;

        if response.status().is_success() {
            Ok(())
        } else {
            Err(PushNotificationError::HttpError(format!(
                "Webhook returned status {}",
                response.status()
            )))
        }
    }
}

fn apply_auth(
    request: reqwest::RequestBuilder,
    auth: &AuthenticationInfo,
) -> reqwest::RequestBuilder {
    match auth.scheme.to_lowercase().as_str() {
        "bearer" => {
            if let Some(ref creds) = auth.credentials {
                request.bearer_auth(creds)
            } else {
                request
            }
        }
        "basic" => {
            if let Some(ref creds) = auth.credentials {
                // credentials in "user:pass" format
                let parts: Vec<&str> = creds.splitn(2, ':').collect();
                if parts.len() == 2 {
                    request.basic_auth(parts[0], Some(parts[1]))
                } else {
                    request.basic_auth(creds, Option::<&str>::None)
                }
            } else {
                request
            }
        }
        _ => {
            // Generic scheme: set Authorization header directly
            if let Some(ref creds) = auth.credentials {
                request.header("Authorization", format!("{} {}", auth.scheme, creds))
            } else {
                request
            }
        }
    }
}

/// Errors specific to push notification delivery.
#[derive(Debug, Clone)]
pub enum PushNotificationError {
    StoreError(String),
    HttpError(String),
}

impl std::fmt::Display for PushNotificationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::StoreError(e) => write!(f, "Push notification store error: {e}"),
            Self::HttpError(e) => write!(f, "Push notification HTTP error: {e}"),
        }
    }
}

impl std::error::Error for PushNotificationError {}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_store_save_and_load() {
        let store = InMemoryPushNotificationStore::default();
        let config = PushNotificationConfig {
            id: Some("pn-1".to_string()),
            url: "https://example.com/webhook".to_string(),
            token: Some("tok".to_string()),
            authentication: None,
        };

        let result = store.save("task-1", config).await.unwrap();
        assert_eq!(result.task_id, "task-1");
        assert_eq!(result.config.url, "https://example.com/webhook");
        assert_eq!(result.config.id, Some("pn-1".to_string()));

        let loaded = store.load("task-1", "pn-1").await.unwrap();
        assert!(loaded.is_some());
        assert_eq!(loaded.unwrap().config.url, "https://example.com/webhook");
    }

    #[tokio::test]
    async fn test_store_auto_generate_id() {
        let store = InMemoryPushNotificationStore::default();
        let config = PushNotificationConfig {
            id: None,
            url: "https://example.com/hook".to_string(),
            token: None,
            authentication: None,
        };

        let result = store.save("task-2", config).await.unwrap();
        assert!(result.config.id.is_some());
        assert!(!result.config.id.as_ref().unwrap().is_empty());
    }

    #[tokio::test]
    async fn test_store_list() {
        let store = InMemoryPushNotificationStore::default();
        for i in 0..3 {
            let config = PushNotificationConfig {
                id: Some(format!("pn-{i}")),
                url: format!("https://example.com/hook/{i}"),
                token: None,
                authentication: None,
            };
            store.save("task-3", config).await.unwrap();
        }

        let configs = store.list("task-3").await.unwrap();
        assert_eq!(configs.len(), 3);
    }

    #[tokio::test]
    async fn test_store_delete() {
        let store = InMemoryPushNotificationStore::default();
        let config = PushNotificationConfig {
            id: Some("del-1".to_string()),
            url: "https://example.com/del".to_string(),
            token: None,
            authentication: None,
        };
        store.save("task-4", config).await.unwrap();

        let deleted = store.delete("task-4", "del-1").await.unwrap();
        assert!(deleted);

        let loaded = store.load("task-4", "del-1").await.unwrap();
        assert!(loaded.is_none());

        let not_deleted = store.delete("task-4", "del-1").await.unwrap();
        assert!(!not_deleted);
    }

    #[tokio::test]
    async fn test_store_empty_list() {
        let store = InMemoryPushNotificationStore::default();
        let configs = store.list("nonexistent").await.unwrap();
        assert!(configs.is_empty());
    }

    #[tokio::test]
    async fn test_configs_for_task() {
        let store = InMemoryPushNotificationStore::default();
        for i in 0..2 {
            let config = PushNotificationConfig {
                id: Some(format!("cfg-{i}")),
                url: format!("https://example.com/notify/{i}"),
                token: None,
                authentication: None,
            };
            store.save("task-5", config).await.unwrap();
        }

        let configs = store.configs_for_task("task-5").await.unwrap();
        assert_eq!(configs.len(), 2);
    }
}
