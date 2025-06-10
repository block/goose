use std::path::PathBuf;
use std::sync::Arc;

use crate::config::Config;
use crate::scheduler::{Scheduler, SchedulerError};
use crate::scheduler_trait::SchedulerTrait;
use crate::temporal_scheduler::TemporalScheduler;

pub enum SchedulerType {
    Legacy,
    Temporal,
}

impl SchedulerType {
    /// Determine scheduler type from configuration
    pub fn from_config() -> Self {
        let config = Config::global();
        match config.get_param::<String>("GOOSE_SCHEDULER_TYPE") {
            Ok(scheduler_type) => match scheduler_type.to_lowercase().as_str() {
                "temporal" => SchedulerType::Temporal,
                "legacy" => SchedulerType::Legacy,
                _ => {
                    tracing::warn!(
                        "Unknown scheduler type '{}', defaulting to legacy",
                        scheduler_type
                    );
                    SchedulerType::Legacy
                }
            },
            Err(_) => {
                // Default to temporal scheduler
                SchedulerType::Temporal
            }
        }
    }
}

/// Factory for creating scheduler instances
pub struct SchedulerFactory;

impl SchedulerFactory {
    /// Create a scheduler instance based on configuration
    pub async fn create(storage_path: PathBuf) -> Result<Arc<dyn SchedulerTrait>, SchedulerError> {
        let scheduler_type = SchedulerType::from_config();

        match scheduler_type {
            SchedulerType::Legacy => {
                tracing::info!("Creating legacy scheduler");
                let scheduler = Scheduler::new(storage_path).await?;
                Ok(scheduler as Arc<dyn SchedulerTrait>)
            }
            SchedulerType::Temporal => {
                tracing::info!("Attempting to create Temporal scheduler");
                match TemporalScheduler::new().await {
                    Ok(scheduler) => {
                        tracing::info!("Temporal scheduler created successfully");
                        Ok(scheduler as Arc<dyn SchedulerTrait>)
                    }
                    Err(e) => {
                        tracing::warn!("Failed to create Temporal scheduler: {}", e);
                        tracing::info!("Falling back to legacy scheduler");

                        // Print helpful message for users
                        eprintln!(
                            "⚠️  Temporal scheduler unavailable, using legacy scheduler instead."
                        );
                        eprintln!("   To use Temporal scheduling features:");
                        eprintln!("   • Install Temporal CLI: brew install temporal (macOS)");
                        eprintln!(
                            "   • Or download from: https://github.com/temporalio/cli/releases"
                        );
                        eprintln!("   • Then restart Goose");
                        eprintln!();

                        let scheduler = Scheduler::new(storage_path).await?;
                        Ok(scheduler as Arc<dyn SchedulerTrait>)
                    }
                }
            }
        }
    }

    /// Create a specific scheduler type (for testing or explicit use)
    pub async fn create_legacy(
        storage_path: PathBuf,
    ) -> Result<Arc<dyn SchedulerTrait>, SchedulerError> {
        tracing::info!("Creating legacy scheduler (explicit)");
        let scheduler = Scheduler::new(storage_path).await?;
        Ok(scheduler as Arc<dyn SchedulerTrait>)
    }

    /// Create a Temporal scheduler (for testing or explicit use)
    pub async fn create_temporal() -> Result<Arc<dyn SchedulerTrait>, SchedulerError> {
        tracing::info!("Creating Temporal scheduler (explicit)");
        let scheduler = TemporalScheduler::new().await?;
        Ok(scheduler as Arc<dyn SchedulerTrait>)
    }
}
