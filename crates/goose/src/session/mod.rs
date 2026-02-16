mod chat_history_search;
mod diagnostics;
pub mod eval_storage;
pub(crate) mod extension_data;
mod legacy;
pub mod session_manager;
pub mod tool_analytics;

pub use diagnostics::{generate_diagnostics, get_system_info, SystemInfo};
pub use eval_storage::{
    AgentDelta, CorrelationInsight, CreateDatasetRequest, EvalDataset, EvalDatasetSummary,
    EvalOverview, EvalRunDetail, EvalRunSummary, EvalStorage, FixedCase, RunComparison,
    RunComparisonSide, RunEvalRequest, TopicAnalytics,
};
pub use extension_data::{EnabledExtensionsState, ExtensionData, ExtensionState, TodoState};
pub use session_manager::{
    DailyActivity, DirectoryUsage, ProviderUsage, Session, SessionAnalytics, SessionInsights,
    SessionManager, SessionType, SessionUpdateBuilder,
};
pub use tool_analytics::{
    AgentPerformanceMetrics, HotTool, LiveMetrics, MinuteActivity, RecentError, ToolAnalytics,
    ToolAnalyticsStore, ToolUsageStat,
};
