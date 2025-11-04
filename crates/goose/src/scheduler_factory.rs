use std::path::PathBuf;
use std::sync::Arc;

use crate::scheduler::{Scheduler, SchedulerError};
use crate::scheduler_trait::SchedulerTrait;

/// Factory for creating scheduler instances
pub struct SchedulerFactory;

impl SchedulerFactory {
    /// Create a scheduler instance
    pub async fn create(storage_path: PathBuf) -> Result<Arc<dyn SchedulerTrait>, SchedulerError> {
        tracing::info!("Creating scheduler");
        let scheduler = Scheduler::new(storage_path).await?;
        Ok(scheduler as Arc<dyn SchedulerTrait>)
    }

    /// Create a scheduler (for testing or explicit use)
    pub async fn create_legacy(
        storage_path: PathBuf,
    ) -> Result<Arc<dyn SchedulerTrait>, SchedulerError> {
        tracing::info!("Creating scheduler (explicit)");
        let scheduler = Scheduler::new(storage_path).await?;
        Ok(scheduler as Arc<dyn SchedulerTrait>)
    }
}
