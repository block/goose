// Training data collection module
pub mod collector;
pub mod quality;
pub mod schema;
pub mod storage;

pub use collector::TrainingDataCollector;
pub use quality::QualityScorer;
pub use schema::{QualityMetrics, TrainingExample, UserFeedback};
pub use storage::TrainingDataStorage;
