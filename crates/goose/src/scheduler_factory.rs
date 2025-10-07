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

        // Check scheduler type preference from GOOSE_SCHEDULER_TYPE
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
                tracing::info!("No scheduler type specified, defaulting to legacy scheduler");
                SchedulerType::Legacy
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
                let scheduler = Scheduler::new(storage_path).await?;
                Ok(scheduler as Arc<dyn SchedulerTrait>)
            }
            SchedulerType::Temporal => match TemporalScheduler::new().await {
                Ok(scheduler) => Ok(scheduler as Arc<dyn SchedulerTrait>),
                Err(_e) => {
                    eprintln!(
                        "⚠️  Temporal scheduler unavailable, using legacy scheduler instead."
                    );
                    eprintln!("   To use Temporal scheduling features:");
                    eprintln!("   • Install Temporal CLI: brew install temporal (macOS)");
                    eprintln!("   • Or download from: https://github.com/temporalio/cli/releases");
                    eprintln!("   • Then restart goose");
                    eprintln!();

                    let scheduler = Scheduler::new(storage_path).await?;
                    Ok(scheduler as Arc<dyn SchedulerTrait>)
                }
            },
        }
    }

    pub async fn create_legacy(
        storage_path: PathBuf,
    ) -> Result<Arc<dyn SchedulerTrait>, SchedulerError> {
        let scheduler = Scheduler::new(storage_path).await?;
        Ok(scheduler as Arc<dyn SchedulerTrait>)
    }

    pub async fn create_temporal() -> Result<Arc<dyn SchedulerTrait>, SchedulerError> {
        let scheduler = TemporalScheduler::new().await?;
        Ok(scheduler as Arc<dyn SchedulerTrait>)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use temp_env::with_vars;

    #[test]
    fn test_scheduler_type_no_env() {
        // Test that without GOOSE_SCHEDULER_TYPE env var, we get Legacy scheduler
        with_vars([("GOOSE_SCHEDULER_TYPE", None::<&str>)], || {
            let scheduler_type = SchedulerType::from_config();
            assert!(matches!(scheduler_type, SchedulerType::Legacy));
        });
    }

    #[test]
    fn test_scheduler_type_legacy() {
        // Test that with GOOSE_SCHEDULER_TYPE=legacy, we get Legacy scheduler
        with_vars([("GOOSE_SCHEDULER_TYPE", Some("legacy"))], || {
            let scheduler_type = SchedulerType::from_config();
            assert!(matches!(scheduler_type, SchedulerType::Legacy));
        });
    }

    #[test]
    fn test_scheduler_type_temporal() {
        // Test that with GOOSE_SCHEDULER_TYPE=temporal, we get Temporal scheduler
        with_vars([("GOOSE_SCHEDULER_TYPE", Some("temporal"))], || {
            let scheduler_type = SchedulerType::from_config();
            assert!(matches!(scheduler_type, SchedulerType::Temporal));
        });
    }

    #[test]
    fn test_scheduler_type_unknown() {
        // Test that with unknown scheduler type, we default to Legacy
        with_vars([("GOOSE_SCHEDULER_TYPE", Some("unknown"))], || {
            let scheduler_type = SchedulerType::from_config();
            assert!(matches!(scheduler_type, SchedulerType::Legacy));
        });
    }
}
