// Adaptive learning system that connects all components
pub mod orchestrator;
pub mod conversation_hooks;
pub mod feedback_collector;
pub mod learning_triggers;
pub mod model_swapper;
pub mod performance_monitor;

pub use orchestrator::AdaptiveLearningOrchestrator;
pub use conversation_hooks::{ConversationHook, ConversationMiddleware};
pub use feedback_collector::{FeedbackCollector, FeedbackEvent};
pub use learning_triggers::{LearningTrigger, TriggerCondition};
pub use model_swapper::{ModelSwapper, SwapStrategy};
pub use performance_monitor::{PerformanceMonitor, PerformanceAlert};
