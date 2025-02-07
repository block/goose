use crate::tracing::observation_layer::{BatchManager, ObservationLayer, SpanTracker};
use chrono::Utc;
use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::env;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::Mutex;
use url::Url;
use uuid::Uuid;

const DEFAULT_LANGFUSE_URL: &str = "http://localhost:3000";

#[derive(Debug, Serialize, Deserialize)]
struct LangfuseIngestionResponse {
    successes: Vec<LangfuseIngestionSuccess>,
    errors: Vec<LangfuseIngestionError>,
}

#[derive(Debug, Serialize, Deserialize)]
struct LangfuseIngestionSuccess {
    id: String,
    status: i32,
}

#[derive(Debug, Serialize, Deserialize)]
struct LangfuseIngestionError {
    id: String,
    status: i32,
    message: Option<String>,
    error: Option<Value>,
}

#[derive(Debug, Clone)]
pub struct LangfuseBatchManager {
    pub batch: Vec<Value>,
    pub client: Client,
    pub base_url: String,
    pub public_key: String,
    pub secret_key: String,
}

impl LangfuseBatchManager {
    pub fn new(public_key: String, secret_key: String, base_url: String) -> Self {
        Self {
            batch: Vec::new(),
            client: Client::builder()
                .timeout(Duration::from_secs(10))
                .build()
                .expect("Failed to create HTTP client"),
            base_url,
            public_key,
            secret_key,
        }
    }

    pub fn spawn_sender(manager: Arc<Mutex<Self>>) {
        const BATCH_INTERVAL: Duration = Duration::from_secs(5);

        tokio::spawn(async move {
            loop {
                tokio::time::sleep(BATCH_INTERVAL).await;
                if let Err(e) = manager.lock().await.send() {
                    tracing::error!(
                        error.msg = %e,
                        error.type = %std::any::type_name_of_val(&e),
                        "Failed to send batch to Langfuse"
                    );
                }
            }
        });
    }

    pub async fn send_async(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.batch.is_empty() {
            return Ok(());
        }

        let payload = json!({ "batch": self.batch });
        let base_url = Url::parse(&self.base_url).map_err(|e| format!("Invalid base URL: {e}"))?;
        let url = base_url
            .join("api/public/ingestion")
            .map_err(|e| format!("Failed to construct endpoint URL: {e}"))?;

        let response = self
            .client
            .post(url)
            .basic_auth(&self.public_key, Some(&self.secret_key))
            .json(&payload)
            .send()
            .await?;

        match response.status() {
            status if status.is_success() => {
                let response_body: LangfuseIngestionResponse = response.json().await?;

                for error in &response_body.errors {
                    tracing::error!(
                        id = %error.id,
                        status = error.status,
                        message = error.message.as_deref().unwrap_or("No message"),
                        error = ?error.error,
                        "Partial failure in batch ingestion"
                    );
                }

                if !response_body.successes.is_empty() {
                    self.batch.clear();
                }

                if response_body.successes.is_empty() && !response_body.errors.is_empty() {
                    Err("Langfuse ingestion failed for all items".into())
                } else {
                    Ok(())
                }
            }
            status @ (StatusCode::BAD_REQUEST
            | StatusCode::UNAUTHORIZED
            | StatusCode::FORBIDDEN
            | StatusCode::NOT_FOUND
            | StatusCode::METHOD_NOT_ALLOWED) => {
                let err_text = response.text().await.unwrap_or_default();
                Err(format!("Langfuse API error: {}: {}", status, err_text).into())
            }
            status => {
                let err_text = response.text().await.unwrap_or_default();
                Err(format!("Unexpected status code: {}: {}", status, err_text).into())
            }
        }
    }
}

impl BatchManager for LangfuseBatchManager {
    fn add_event(&mut self, event_type: &str, body: Value) {
        self.batch.push(json!({
            "id": Uuid::new_v4().to_string(),
            "timestamp": Utc::now().to_rfc3339(),
            "type": event_type,
            "body": body
        }));
    }

    fn send(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        tokio::task::block_in_place(|| {
            tokio::runtime::Handle::current().block_on(self.send_async())
        })
    }

    fn is_empty(&self) -> bool {
        self.batch.is_empty()
    }
}

pub fn create_langfuse_observer() -> Option<ObservationLayer> {
    let public_key = env::var("LANGFUSE_PUBLIC_KEY")
        .or_else(|_| env::var("LANGFUSE_INIT_PROJECT_PUBLIC_KEY"))
        .unwrap_or_default(); // Use empty string if not found

    let secret_key = env::var("LANGFUSE_SECRET_KEY")
        .or_else(|_| env::var("LANGFUSE_INIT_PROJECT_SECRET_KEY"))
        .unwrap_or_default(); // Use empty string if not found

    // Return None if either key is empty
    if public_key.is_empty() || secret_key.is_empty() {
        return None;
    }

    let base_url = env::var("LANGFUSE_URL").unwrap_or_else(|_| DEFAULT_LANGFUSE_URL.to_string());

    let batch_manager = Arc::new(Mutex::new(LangfuseBatchManager::new(
        public_key, secret_key, base_url,
    )));

    if !cfg!(test) {
        LangfuseBatchManager::spawn_sender(batch_manager.clone());
    }

    Some(ObservationLayer {
        batch_manager,
        span_tracker: Arc::new(Mutex::new(SpanTracker::new())),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::collections::HashMap;
    use tokio::sync::Mutex;
    use tracing::dispatcher;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    struct TestFixture {
        original_subscriber: Option<dispatcher::Dispatch>,
        original_env_vars: HashMap<String, String>,
        mock_server: Option<MockServer>,
    }

    impl TestFixture {
        async fn new() -> Self {
            Self {
                original_subscriber: Some(dispatcher::get_default(dispatcher::Dispatch::clone)),
                original_env_vars: Self::save_env_vars(),
                mock_server: None,
            }
        }

        fn save_env_vars() -> HashMap<String, String> {
            [
                "LANGFUSE_PUBLIC_KEY",
                "LANGFUSE_INIT_PROJECT_PUBLIC_KEY",
                "LANGFUSE_SECRET_KEY",
                "LANGFUSE_INIT_PROJECT_SECRET_KEY",
                "LANGFUSE_URL",
            ]
            .iter()
            .filter_map(|&var| env::var(var).ok().map(|val| (var.to_string(), val)))
            .collect()
        }

        async fn with_mock_server(mut self) -> Self {
            self.mock_server = Some(MockServer::start().await);
            self
        }

        fn mock_server_uri(&self) -> String {
            self.mock_server
                .as_ref()
                .expect("Mock server not initialized")
                .uri()
        }

        async fn mock_response(&self, status: u16, body: Value) {
            Mock::given(method("POST"))
                .and(path("/api/public/ingestion"))
                .respond_with(ResponseTemplate::new(status).set_body_json(body))
                .mount(self.mock_server.as_ref().unwrap())
                .await;
        }
    }

    impl Drop for TestFixture {
        fn drop(&mut self) {
            // Restore original subscriber
            if let Some(subscriber) = &self.original_subscriber {
                let _ = dispatcher::set_global_default(subscriber.clone());
            }

            // Restore environment
            for var in [
                "LANGFUSE_PUBLIC_KEY",
                "LANGFUSE_INIT_PROJECT_PUBLIC_KEY",
                "LANGFUSE_SECRET_KEY",
                "LANGFUSE_INIT_PROJECT_SECRET_KEY",
                "LANGFUSE_URL",
            ] {
                if let Some(value) = self.original_env_vars.get(var) {
                    env::set_var(var, value);
                } else {
                    env::remove_var(var);
                }
            }
        }
    }

    fn create_test_event() -> Value {
        json!({
            "name": "test_span",
            "type": "SPAN"
        })
    }

    #[tokio::test]
    async fn test_batch_manager_creation() {
        let _fixture = TestFixture::new().await;

        let manager = LangfuseBatchManager::new(
            "test-public".to_string(),
            "test-secret".to_string(),
            "http://test.local".to_string(),
        );

        assert_eq!(manager.public_key, "test-public");
        assert_eq!(manager.secret_key, "test-secret");
        assert_eq!(manager.base_url, "http://test.local");
        assert!(manager.batch.is_empty());
    }

    #[tokio::test]
    async fn test_add_event() {
        let _fixture = TestFixture::new().await;
        let mut manager = LangfuseBatchManager::new(
            "test-public".to_string(),
            "test-secret".to_string(),
            "http://test.local".to_string(),
        );

        manager.add_event("test-event", create_test_event());

        assert_eq!(manager.batch.len(), 1);
        let event = &manager.batch[0];
        assert_eq!(event["type"], "test-event");
        assert_eq!(event["body"], create_test_event());
        assert!(event["id"].as_str().is_some());
        assert!(event["timestamp"].as_str().is_some());
    }

    #[tokio::test]
    async fn test_batch_send_success() {
        let fixture = TestFixture::new().await.with_mock_server().await;

        fixture
            .mock_response(
                200,
                json!({
                    "successes": [{"id": "1", "status": 200}],
                    "errors": []
                }),
            )
            .await;

        let mut manager = LangfuseBatchManager::new(
            "test-public".to_string(),
            "test-secret".to_string(),
            fixture.mock_server_uri(),
        );

        manager.add_event("test-event", create_test_event());

        let result = manager.send_async().await;
        assert!(result.is_ok());
        assert!(manager.batch.is_empty());
    }

    #[tokio::test]
    async fn test_batch_send_partial_failure() {
        let fixture = TestFixture::new().await.with_mock_server().await;

        fixture
            .mock_response(
                200,
                json!({
                    "successes": [{"id": "1", "status": 200}],
                    "errors": [{"id": "2", "status": 400, "message": "Invalid data"}]
                }),
            )
            .await;

        let mut manager = LangfuseBatchManager::new(
            "test-public".to_string(),
            "test-secret".to_string(),
            fixture.mock_server_uri(),
        );

        manager.add_event("test-event", create_test_event());

        let result = manager.send_async().await;
        assert!(result.is_ok());
        assert!(manager.batch.is_empty());
    }

    #[tokio::test]
    async fn test_batch_send_complete_failure() {
        let fixture = TestFixture::new().await.with_mock_server().await;

        fixture
            .mock_response(
                200,
                json!({
                    "successes": [],
                    "errors": [{"id": "1", "status": 400, "message": "Invalid data"}]
                }),
            )
            .await;

        let mut manager = LangfuseBatchManager::new(
            "test-public".to_string(),
            "test-secret".to_string(),
            fixture.mock_server_uri(),
        );

        manager.add_event("test-event", create_test_event());

        let result = manager.send_async().await;
        assert!(result.is_err());
        assert!(!manager.batch.is_empty());
    }

    #[tokio::test]
    async fn test_create_langfuse_observer() {
        let fixture = TestFixture::new().await.with_mock_server().await;

        // Test 1: No environment variables set - remove all possible variables
        for var in &[
            "LANGFUSE_PUBLIC_KEY",
            "LANGFUSE_INIT_PROJECT_PUBLIC_KEY",
            "LANGFUSE_SECRET_KEY",
            "LANGFUSE_INIT_PROJECT_SECRET_KEY",
            "LANGFUSE_URL",
        ] {
            env::remove_var(var);
        }

        let observer = create_langfuse_observer();
        assert!(
            observer.is_none(),
            "Observer should be None without environment variables"
        );

        // Test 2: Only public key set (regular)
        env::set_var("LANGFUSE_PUBLIC_KEY", "test-public-key");
        let observer = create_langfuse_observer();
        assert!(
            observer.is_none(),
            "Observer should be None with only public key"
        );
        env::remove_var("LANGFUSE_PUBLIC_KEY");

        // Test 3: Only secret key set (regular)
        env::set_var("LANGFUSE_SECRET_KEY", "test-secret-key");
        let observer = create_langfuse_observer();
        assert!(
            observer.is_none(),
            "Observer should be None with only secret key"
        );
        env::remove_var("LANGFUSE_SECRET_KEY");

        // Test 4: Only public key set (init project)
        env::set_var("LANGFUSE_INIT_PROJECT_PUBLIC_KEY", "test-public-key");
        let observer = create_langfuse_observer();
        assert!(
            observer.is_none(),
            "Observer should be None with only init project public key"
        );
        env::remove_var("LANGFUSE_INIT_PROJECT_PUBLIC_KEY");

        // Test 5: Only secret key set (init project)
        env::set_var("LANGFUSE_INIT_PROJECT_SECRET_KEY", "test-secret-key");
        let observer = create_langfuse_observer();
        assert!(
            observer.is_none(),
            "Observer should be None with only init project secret key"
        );
        env::remove_var("LANGFUSE_INIT_PROJECT_SECRET_KEY");

        // Test 6: Both regular keys set (should succeed)
        env::set_var("LANGFUSE_PUBLIC_KEY", "test-public-key");
        env::set_var("LANGFUSE_SECRET_KEY", "test-secret-key");
        env::set_var("LANGFUSE_URL", fixture.mock_server_uri());
        let observer = create_langfuse_observer();
        assert!(
            observer.is_some(),
            "Observer should be Some with both regular keys set"
        );

        // Clean up regular keys
        env::remove_var("LANGFUSE_PUBLIC_KEY");
        env::remove_var("LANGFUSE_SECRET_KEY");

        // Test 7: Both init project keys set (should succeed)
        env::set_var("LANGFUSE_INIT_PROJECT_PUBLIC_KEY", "test-public-key");
        env::set_var("LANGFUSE_INIT_PROJECT_SECRET_KEY", "test-secret-key");
        let observer = create_langfuse_observer();
        assert!(
            observer.is_some(),
            "Observer should be Some with both init project keys set"
        );

        // Verify the observer has an empty batch manager
        let batch_manager = observer.unwrap().batch_manager;
        assert!(batch_manager.lock().await.is_empty());
    }
    #[tokio::test]
    async fn test_batch_manager_spawn_sender() {
        let fixture = TestFixture::new().await.with_mock_server().await;

        fixture
            .mock_response(
                200,
                json!({
                    "successes": [{"id": "1", "status": 200}],
                    "errors": []
                }),
            )
            .await;

        let manager = Arc::new(Mutex::new(LangfuseBatchManager::new(
            "test-public".to_string(),
            "test-secret".to_string(),
            fixture.mock_server_uri(),
        )));

        manager
            .lock()
            .await
            .add_event("test-event", create_test_event());

        // Instead of spawning the sender which uses blocking operations,
        // test the async send directly
        let result = manager.lock().await.send_async().await;
        assert!(result.is_ok());
        assert!(manager.lock().await.batch.is_empty());
    }
}
