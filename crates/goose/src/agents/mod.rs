// =============================================================================
// Domain-organized facade modules (new, preferred import paths)
// =============================================================================
// These provide organized access to agent subsystems. All existing flat imports
// continue to work — these are additive facades, not replacements.

/// Core agent runtime — Agent struct, config, prompt management, retry logic
pub mod core;
/// Extension management — ExtensionManager, ExtensionConfig, built-in extensions
pub mod extensions;
/// Orchestration — delegation strategies, sub-agent management
pub mod orchestration;
/// Agent persona definitions — data-driven configurations (GooseAgent, DeveloperAgent, etc.)
pub mod personas;
/// Message routing — IntentRouter, UniversalMode, routing evaluation
pub mod routing;
/// Tool management — registry, filtering, output tools
pub mod tools;

// =============================================================================
// Original flat module declarations (preserved for backward compatibility)
// =============================================================================

mod agent;
pub(crate) mod apps_extension;
pub(crate) mod builtin_skills;
pub(crate) mod chatrecall_extension;
pub(crate) mod code_execution_extension;
pub mod container;
pub mod delegation;
pub mod developer_agent;
pub mod dispatch;
pub mod execute_commands;
pub mod extension;
pub mod extension_malware_check;
pub mod extension_manager;
pub mod extension_manager_extension;
pub mod extension_registry;
pub mod final_output_tool;
pub(crate) mod genui_extension;
pub mod goose_agent;
pub mod intent_router;
mod large_response_handler;
pub mod mcp_client;
pub mod moim;
pub mod orchestrator_agent;
pub mod platform_tools;
pub mod pm_agent;
pub mod prompt_manager;
pub mod qa_agent;
mod reply_parts;
pub mod research_agent;
pub mod retry;
pub mod routing_eval;
mod schedule_tool;
pub mod security_agent;
pub(crate) mod specialist_config;
pub(crate) mod specialist_handler;
pub mod subagent_execution_tool;
pub mod summon;
pub(crate) mod summon_extension;
pub(crate) mod todo_extension;
pub(crate) mod tom_extension;
mod tool_execution;
pub mod tool_filter;
pub mod tool_registry;
pub mod types;
pub mod universal_mode;

// =============================================================================
// Top-level re-exports (preserved for backward compatibility)
// =============================================================================

pub use agent::{Agent, AgentConfig, AgentEvent, ExtensionLoadResult};
pub use container::Container;
pub use execute_commands::COMPACT_TRIGGERS;
pub use extension::{ExtensionConfig, ExtensionError};
pub use extension_manager::ExtensionManager;
pub use prompt_manager::PromptManager;
pub use specialist_config::TaskConfig;
pub use specialist_handler::SPECIALIST_TOOL_REQUEST_TYPE;
/// Backward compat alias
pub use specialist_handler::SPECIALIST_TOOL_REQUEST_TYPE as SUBAGENT_TOOL_REQUEST_TYPE;
pub use types::{FrontendTool, RetryConfig, SessionConfig, SuccessCheck};
