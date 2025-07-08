use crate::telemetry::{
    config::TelemetryConfig,
    events::TelemetryEvent,
};
use serde_json;
use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::sync::Mutex;

/// File-based telemetry provider that writes JSON events to a file. mostly for debugging
pub struct FileProvider {
    file_path: Option<String>,
    file_handle: Option<Mutex<std::fs::File>>,
    initialized: bool,
}

impl FileProvider {
    pub fn new() -> Self {
        Self {
            file_path: None,
            file_handle: None,
            initialized: false,
        }
    }

    fn write_event_to_file(&self, event: &TelemetryEvent) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if let Some(file_handle) = &self.file_handle {
            let mut file = file_handle.lock().map_err(|e| format!("Failed to lock file: {}", e))?;

            let event_record = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "event_type": match event {
                    TelemetryEvent::RecipeExecution(_) => "recipe_execution",
                    TelemetryEvent::SystemMetrics(_) => "system_metrics", 
                    TelemetryEvent::UserSession(_) => "user_session",
                },
                "data": event
            });

            writeln!(file, "{}", serde_json::to_string(&event_record)?)?;
            file.flush()?;
        }
        
        Ok(())
    }
}

#[async_trait::async_trait]
impl super::TelemetryBackend for FileProvider {
    async fn initialize(
        &mut self,
        config: &TelemetryConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized {
            return Ok(());
        }

        let file_path = config.get_endpoint()
            .ok_or("File provider requires a file path in GOOSE_TELEMETRY_ENDPOINT")?;

        if let Some(parent) = Path::new(&file_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        self.file_path = Some(file_path.clone());
        self.file_handle = Some(Mutex::new(file));
        self.initialized = true;

        let init_record = serde_json::json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "event_type": "telemetry_init",
            "data": {
                "service_name": config.service_name,
                "service_version": config.service_version,
                "provider": "file",
                "file_path": file_path
            }
        });

        if let Some(file_handle) = &self.file_handle {
            let mut file = file_handle.lock().map_err(|e| format!("Failed to lock file: {}", e))?;
            writeln!(file, "{}", serde_json::to_string(&init_record)?)?;
            file.flush()?;
        }

        tracing::info!("File telemetry provider initialized: {}", file_path);
        Ok(())
    }

    async fn send_event(
        &self,
        event: &TelemetryEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if !self.initialized {
            return Err("File provider not initialized".into());
        }

        self.write_event_to_file(event)?;
        Ok(())
    }

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if self.initialized {
            let shutdown_record = serde_json::json!({
                "timestamp": chrono::Utc::now().to_rfc3339(),
                "event_type": "telemetry_shutdown",
                "data": {}
            });

            if let Some(file_handle) = &self.file_handle {
                let mut file = file_handle.lock().map_err(|e| format!("Failed to lock file: {}", e))?;
                writeln!(file, "{}", serde_json::to_string(&shutdown_record)?)?;
                file.flush()?;
            }

            if let Some(file_path) = &self.file_path {
                tracing::info!("File telemetry provider shutdown: {}", file_path);
            }
        }
        Ok(())
    }
}
