use crate::telemetry::{
    config::TelemetryConfig,
    events::{CommandExecution, RecipeExecution, SessionExecution, TelemetryEvent},
    providers::{create_backend, TelemetryBackend},
    user::UserIdentity,
};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct TelemetryManager {
    config: TelemetryConfig,
    backend: Option<Arc<Mutex<Box<dyn TelemetryBackend>>>>,
    user_identity: Option<UserIdentity>,
    enabled: bool,
}

impl TelemetryManager {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error + Send + Sync>> {
        let config = TelemetryConfig::from_env();

        if let Err(e) = config.validate() {
            tracing::warn!("Telemetry configuration invalid: {}", e);
            return Ok(Self {
                config,
                backend: None,
                user_identity: None,
                enabled: false,
            });
        }

        let mut manager = Self {
            config: config.clone(),
            backend: None,
            user_identity: None,
            enabled: config.enabled,
        };

        if manager.enabled {
            match manager.initialize().await {
                Ok(()) => {
                    tracing::info!(
                        "Telemetry initialized successfully with provider: {:?}",
                        config.provider
                    );
                }
                Err(e) => {
                    tracing::warn!(
                        "Failed to initialize telemetry: {}. Continuing without telemetry.",
                        e
                    );
                    manager.enabled = false;
                }
            }
        } else {
            tracing::debug!("Telemetry disabled");
        }

        Ok(manager)
    }

    async fn initialize(&mut self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        self.user_identity = Some(UserIdentity::load_or_create().await?);

        let mut backend = create_backend(&self.config);
        backend.initialize(&self.config).await?;
        self.backend = Some(Arc::new(Mutex::new(backend)));

        Ok(())
    }

    pub async fn track_recipe_execution(
        &self,
        mut execution: RecipeExecution,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(user_identity) = &self.user_identity {
            execution.user_id = user_identity.user_id.clone();
            execution.usage_type = user_identity.usage_type.clone();
        }

        if let Some(environment) = &self.config.environment {
            execution.environment = Some(environment.clone());
        }

        execution.complete();

        if let Some(backend) = &self.backend {
            let backend_guard = backend.lock().await;
            let event = TelemetryEvent::RecipeExecution(execution);

            if let Err(e) = backend_guard.send_event(&event).await {
                tracing::warn!("Failed to send telemetry event: {}", e);
            }
        }

        Ok(())
    }

    pub async fn track_recipe_executions(
        &self,
        executions: Vec<RecipeExecution>,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.enabled {
            return Ok(());
        }

        for execution in executions {
            self.track_recipe_execution(execution).await?;
        }

        Ok(())
    }

    pub async fn track_session_execution(
        &self,
        mut execution: SessionExecution,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(user_identity) = &self.user_identity {
            execution.user_id = user_identity.user_id.clone();
            execution.usage_type = user_identity.usage_type.clone();
        }

        if let Some(environment) = &self.config.environment {
            execution.environment = Some(environment.clone());
        }

        execution.complete();

        if let Some(backend) = &self.backend {
            let backend_guard = backend.lock().await;
            let event = TelemetryEvent::SessionExecution(execution);

            if let Err(e) = backend_guard.send_event(&event).await {
                tracing::warn!("Failed to send telemetry event: {}", e);
            }
        }

        Ok(())
    }

    pub async fn track_command_execution(
        &self,
        mut execution: CommandExecution,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.enabled {
            return Ok(());
        }

        if let Some(user_identity) = &self.user_identity {
            execution.user_id = user_identity.user_id.clone();
            execution.usage_type = user_identity.usage_type.clone();
        }

        if let Some(environment) = &self.config.environment {
            execution.environment = Some(environment.clone());
        }

        execution.complete();

        if let Some(backend) = &self.backend {
            let backend_guard = backend.lock().await;
            let event = TelemetryEvent::CommandExecution(execution);

            if let Err(e) = backend_guard.send_event(&event).await {
                tracing::warn!("Failed to send telemetry event: {}", e);
            }
        }

        Ok(())
    }

    pub fn get_user_identity(&self) -> Option<&UserIdentity> {
        self.user_identity.as_ref()
    }

    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    pub fn get_config(&self) -> &TelemetryConfig {
        &self.config
    }

    pub async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(backend) = &self.backend {
            let backend_guard = backend.lock().await;
            if let Err(e) = backend_guard.shutdown().await {
                tracing::warn!("Error during telemetry shutdown: {}", e);
            } else {
                tracing::debug!("Telemetry shutdown successfully");
            }
        }

        Ok(())
    }

    pub fn recipe_execution(&self, name: &str, version: &str) -> RecipeExecutionBuilder {
        RecipeExecutionBuilder::new(name, version)
    }
}

pub struct RecipeExecutionBuilder {
    execution: RecipeExecution,
}

impl RecipeExecutionBuilder {
    fn new(name: &str, version: &str) -> Self {
        Self {
            execution: RecipeExecution::new(name, version),
        }
    }

    pub fn with_metadata(mut self, key: &str, value: &str) -> Self {
        self.execution = self.execution.with_metadata(key, value);
        self
    }

    pub fn with_token_usage(mut self, token_usage: crate::telemetry::events::TokenUsage) -> Self {
        self.execution = self.execution.with_token_usage(token_usage);
        self
    }

    pub fn add_tool_usage(mut self, tool_usage: crate::telemetry::events::ToolUsage) -> Self {
        self.execution.add_tool_usage(tool_usage);
        self
    }

    pub fn with_error_details(
        mut self,
        error_details: crate::telemetry::events::ErrorDetails,
    ) -> Self {
        self.execution = self.execution.with_error_details(error_details);
        self
    }

    pub fn with_result(mut self, result: crate::telemetry::events::RecipeResult) -> Self {
        self.execution = self.execution.with_result(result);
        self
    }

    pub fn with_duration(mut self, duration: std::time::Duration) -> Self {
        self.execution = self.execution.with_duration(duration);
        self
    }

    pub fn with_environment(mut self, environment: &str) -> Self {
        self.execution = self.execution.with_environment(environment);
        self
    }

    pub fn build(self) -> RecipeExecution {
        self.execution
    }

    pub async fn track(
        self,
        manager: &TelemetryManager,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        manager.track_recipe_execution(self.execution).await
    }
}

static GLOBAL_TELEMETRY: once_cell::sync::OnceCell<Arc<TelemetryManager>> =
    once_cell::sync::OnceCell::new();

pub async fn init_global_telemetry() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    let manager = TelemetryManager::new().await?;
    GLOBAL_TELEMETRY
        .set(Arc::new(manager))
        .map_err(|_| "Global telemetry already initialized")?;
    Ok(())
}

pub fn global_telemetry() -> Option<Arc<TelemetryManager>> {
    GLOBAL_TELEMETRY.get().cloned()
}

pub async fn shutdown_global_telemetry() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(manager) = GLOBAL_TELEMETRY.get() {
        manager.shutdown().await?;
    }
    Ok(())
}

#[macro_export]
macro_rules! track_recipe {
    ($name:expr, $version:expr) => {{
        if let Some(manager) = $crate::telemetry::global_telemetry() {
            manager.recipe_execution($name, $version)
        } else {
            $crate::telemetry::RecipeExecutionBuilder::new($name, $version)
        }
    }};
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::telemetry::{
        config::{TelemetryConfig, TelemetryProvider},
        events::{RecipeResult, TokenUsage},
        providers::TelemetryBackend,
    };
    use std::env;
    use std::sync::Arc;
    use tokio::sync::Mutex;

    // Mock backend for fast testing
    struct MockTelemetryBackend {
        events: Arc<Mutex<Vec<TelemetryEvent>>>,
    }

    impl MockTelemetryBackend {
        fn new() -> Self {
            Self {
                events: Arc::new(Mutex::new(Vec::new())),
            }
        }

        async fn get_events(&self) -> Vec<TelemetryEvent> {
            self.events.lock().await.clone()
        }
    }

    #[async_trait::async_trait]
    impl TelemetryBackend for MockTelemetryBackend {
        async fn initialize(
            &mut self,
            _config: &TelemetryConfig,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }

        async fn send_event(
            &self,
            event: &TelemetryEvent,
        ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            self.events.lock().await.push(event.clone());
            Ok(())
        }

        async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
            Ok(())
        }
    }

    // Fast unit tests
    #[test]
    fn test_telemetry_manager_disabled_sync() {
        let config = TelemetryConfig {
            enabled: false,
            provider: TelemetryProvider::Console,
            endpoint: None,
            api_key: None,
            usage_type: None,
            environment: None,
            service_name: "goose".to_string(),
            service_version: "1.0.0".to_string(),
        };

        let manager = TelemetryManager {
            config,
            backend: None,
            user_identity: None,
            enabled: false,
        };

        assert!(!manager.is_enabled());
        assert!(manager.get_user_identity().is_none());
    }

    #[test]
    fn test_recipe_execution_builder_sync() {
        let builder = RecipeExecutionBuilder::new("test-recipe", "1.0.0");
        let execution = builder
            .with_metadata("key1", "value1")
            .with_token_usage(TokenUsage::new(100, 50))
            .with_result(RecipeResult::Success)
            .build();

        assert_eq!(execution.recipe_name, "test-recipe");
        assert_eq!(execution.recipe_version, "1.0.0");
        assert_eq!(execution.metadata.get("key1"), Some(&"value1".to_string()));
        assert!(execution.token_usage.is_some());
        assert_eq!(execution.result, Some(RecipeResult::Success));
    }

    #[tokio::test]
    async fn test_telemetry_manager_with_mock_backend() {
        let mock_backend = MockTelemetryBackend::new();
        let config = TelemetryConfig {
            enabled: true,
            provider: TelemetryProvider::Console,
            endpoint: None,
            api_key: None,
            usage_type: None,
            environment: Some("test".to_string()),
            service_name: "goose".to_string(),
            service_version: "1.0.0".to_string(),
        };

        let manager = TelemetryManager {
            config,
            backend: Some(Arc::new(Mutex::new(Box::new(mock_backend)))),
            user_identity: Some(crate::telemetry::user::UserIdentity {
                user_id: "test-user".to_string(),
                usage_type: crate::telemetry::config::UsageType::Human,
                first_seen: chrono::Utc::now(),
                last_seen: chrono::Utc::now(),
            }),
            enabled: true,
        };

        let execution =
            RecipeExecution::new("test-recipe", "1.0.0").with_result(RecipeResult::Success);

        let result = manager.track_recipe_execution(execution).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_batch_tracking_with_mock() {
        let mock_backend = MockTelemetryBackend::new();
        let events_ref = mock_backend.events.clone();

        let config = TelemetryConfig {
            enabled: true,
            provider: TelemetryProvider::Console,
            endpoint: None,
            api_key: None,
            usage_type: None,
            environment: None,
            service_name: "goose".to_string(),
            service_version: "1.0.0".to_string(),
        };

        let manager = TelemetryManager {
            config,
            backend: Some(Arc::new(Mutex::new(Box::new(mock_backend)))),
            user_identity: None,
            enabled: true,
        };

        let executions = vec![
            RecipeExecution::new("recipe1", "1.0.0").with_result(RecipeResult::Success),
            RecipeExecution::new("recipe2", "1.0.0")
                .with_result(RecipeResult::Error("test error".to_string())),
            RecipeExecution::new("recipe3", "1.0.0").with_result(RecipeResult::Cancelled),
        ];

        let result = manager.track_recipe_executions(executions).await;
        assert!(result.is_ok());

        let events = events_ref.lock().await;
        assert_eq!(events.len(), 3);
    }

    // Integration tests (slower, marked with ignore for optional running)
    #[tokio::test]
    #[ignore = "slow integration test"]
    async fn test_telemetry_manager_disabled_integration() {
        env::remove_var("GOOSE_TELEMETRY_ENABLED");
        env::remove_var("DD_API_KEY");
        env::remove_var("OTEL_EXPORTER_OTLP_ENDPOINT");

        let manager = TelemetryManager::new().await.unwrap();
        assert!(!manager.is_enabled());

        let execution = RecipeExecution::new("test", "1.0.0");
        let result = manager.track_recipe_execution(execution).await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    #[ignore = "slow integration test"]
    async fn test_telemetry_manager_console_integration() {
        env::set_var("GOOSE_TELEMETRY_ENABLED", "true");
        env::set_var("GOOSE_TELEMETRY_PROVIDER", "console");

        let manager = TelemetryManager::new().await.unwrap();
        assert!(manager.is_enabled());
        assert_eq!(manager.get_config().provider, TelemetryProvider::Console);

        let execution = RecipeExecution::new("test-recipe", "1.0.0")
            .with_result(RecipeResult::Success)
            .with_token_usage(TokenUsage::new(100, 50));

        let result = manager.track_recipe_execution(execution).await;
        assert!(result.is_ok());

        manager.shutdown().await.unwrap();
        env::remove_var("GOOSE_TELEMETRY_ENABLED");
        env::remove_var("GOOSE_TELEMETRY_PROVIDER");
    }

    #[tokio::test]
    #[ignore = "slow integration test"]
    async fn test_recipe_execution_builder_integration() {
        env::set_var("GOOSE_TELEMETRY_ENABLED", "true");
        env::set_var("GOOSE_TELEMETRY_PROVIDER", "console");

        let manager = TelemetryManager::new().await.unwrap();

        let execution = manager
            .recipe_execution("test-recipe", "1.0.0")
            .with_metadata("key1", "value1")
            .with_token_usage(TokenUsage::new(100, 50))
            .with_result(RecipeResult::Success)
            .build();

        assert_eq!(execution.recipe_name, "test-recipe");
        assert_eq!(execution.recipe_version, "1.0.0");
        assert_eq!(execution.metadata.get("key1"), Some(&"value1".to_string()));
        assert!(execution.token_usage.is_some());
        assert_eq!(execution.result, Some(RecipeResult::Success));

        let result = manager.track_recipe_execution(execution).await;
        assert!(result.is_ok());

        manager.shutdown().await.unwrap();
        env::remove_var("GOOSE_TELEMETRY_ENABLED");
        env::remove_var("GOOSE_TELEMETRY_PROVIDER");
    }

    #[tokio::test]
    #[ignore = "slow integration test"]
    async fn test_global_telemetry_integration() {
        init_global_telemetry().await.unwrap();

        let manager = global_telemetry().unwrap();
        assert!(!manager.is_enabled());

        shutdown_global_telemetry().await.unwrap();
    }

    #[tokio::test]
    #[ignore = "slow integration test"]
    async fn test_batch_tracking_integration() {
        env::set_var("GOOSE_TELEMETRY_ENABLED", "true");
        env::set_var("GOOSE_TELEMETRY_PROVIDER", "console");

        let manager = TelemetryManager::new().await.unwrap();

        let executions = vec![
            RecipeExecution::new("recipe1", "1.0.0").with_result(RecipeResult::Success),
            RecipeExecution::new("recipe2", "1.0.0")
                .with_result(RecipeResult::Error("test error".to_string())),
            RecipeExecution::new("recipe3", "1.0.0").with_result(RecipeResult::Cancelled),
        ];

        let result = manager.track_recipe_executions(executions).await;
        assert!(result.is_ok());

        manager.shutdown().await.unwrap();
        env::remove_var("GOOSE_TELEMETRY_ENABLED");
        env::remove_var("GOOSE_TELEMETRY_PROVIDER");
    }
}
