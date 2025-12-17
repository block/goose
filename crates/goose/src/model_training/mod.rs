// Model training and fine-tuning module
// Only includes minimal data structures and Axolotl integration
// Actual training is handled by Axolotl, inference by Ollama
pub mod trainer;
pub mod job_manager;
pub mod axolotl;
pub mod inference_manager;

pub use job_manager::{TrainingJobManager, TrainingJob, JobStatus};
pub use inference_manager::{InferenceManager, InferenceServerStatus, ServerStatus, INFERENCE_MANAGER};
