use crate::telemetry::{
    config::{TelemetryConfig, TelemetryProvider},
    events::TelemetryEvent,
};

pub mod console;
pub mod datadog;
pub mod file;
pub mod otlp;

#[async_trait::async_trait]
pub trait TelemetryBackend: Send + Sync {
    async fn initialize(
        &mut self,
        config: &TelemetryConfig,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn send_event(
        &self,
        event: &TelemetryEvent,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;

    async fn shutdown(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>>;
}

pub fn create_backend(config: &TelemetryConfig) -> Box<dyn TelemetryBackend> {
    match config.provider {
        TelemetryProvider::Console => Box::new(console::ConsoleProvider::new()),
        TelemetryProvider::File => Box::new(file::FileProvider::new()),
        TelemetryProvider::Datadog => Box::new(datadog::DatadogProvider::new()),
        TelemetryProvider::Otlp => Box::new(otlp::OtlpProvider::new()),
    }
}
