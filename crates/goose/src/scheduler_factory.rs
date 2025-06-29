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
    pub fn from_config() -> Self {
        let config = Config::global();

        // First check if alpha features are enabled
        // If not, always use legacy scheduler regardless of GOOSE_SCHEDULER_TYPE
        match config.get_param::<String>("ALPHA") {
            Ok(alpha_value) => {
                // Only proceed with temporal if alpha is explicitly enabled
                if alpha_value.to_lowercase() != "true" {
                    tracing::info!("Alpha features disabled, using legacy scheduler");
                    return SchedulerType::Legacy;
                }
            }
            Err(_) => {
                // No ALPHA env var means alpha features are disabled
                tracing::info!("No ALPHA environment variable found, using legacy scheduler");
                return SchedulerType::Legacy;
            }
        }

        // Alpha is enabled, now check scheduler type preference
        match config.get_param::<String>("GOOSE_SCHEDULER_TYPE") {
            Ok(scheduler_type) => match scheduler_type.to_lowercase().as_str() {
                "temporal" => SchedulerType::Temporal,
                "legacy" => SchedulerType::Legacy,
                _ => {
                    tracing::warn!(
                        "Unknown scheduler type '{}', defaulting to legacy scheduler",
                        scheduler_type
                    );
                    SchedulerType::Legacy
                }
            },
            Err(_) => {
                // When alpha is enabled but no explicit scheduler type is set,
                // default to temporal scheduler
                tracing::info!("Alpha enabled, defaulting to temporal scheduler");
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

    /// Create a legacy scheduler (for testing or explicit use)
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

#[cfg(test)]
mod tests {
    use super::*;
    use temp_env::with_vars;

    #[test]
    fn test_scheduler_type_no_alpha_env() {
        // Test that without ALPHA env var, we always get Legacy scheduler
        with_vars(
            [
                ("ALPHA", None::<&str>),
                ("GOOSE_SCHEDULER_TYPE", Some("temporal")),
            ],
            || {
                let scheduler_type = SchedulerType::from_config();
                assert!(matches!(scheduler_type, SchedulerType::Legacy));
            },
        );
    }

    #[test]
    fn test_scheduler_type_alpha_false() {
        // Test that with ALPHA=false, we always get Legacy scheduler
        with_vars(
            [
                ("ALPHA", Some("false")),
                ("GOOSE_SCHEDULER_TYPE", Some("temporal")),
            ],
            || {
                let scheduler_type = SchedulerType::from_config();
                assert!(matches!(scheduler_type, SchedulerType::Legacy));
            },
        );
    }

    #[test]
    fn test_scheduler_type_alpha_true_legacy() {
        // Test that with ALPHA=true and GOOSE_SCHEDULER_TYPE=legacy, we get Legacy scheduler
        with_vars(
            [
                ("ALPHA", Some("true")),
                ("GOOSE_SCHEDULER_TYPE", Some("legacy")),
            ],
            || {
                let scheduler_type = SchedulerType::from_config();
                assert!(matches!(scheduler_type, SchedulerType::Legacy));
            },
        );
    }

    #[test]
    fn test_scheduler_type_alpha_true_unknown_scheduler_type() {
        // Test that with ALPHA=true and unknown scheduler type, we default to Legacy
        with_vars(
            [
                ("ALPHA", Some("true")),
                ("GOOSE_SCHEDULER_TYPE", Some("unknown")),
            ],
            || {
                let scheduler_type = SchedulerType::from_config();
                assert!(matches!(scheduler_type, SchedulerType::Legacy));
            },
        );
    }
}
