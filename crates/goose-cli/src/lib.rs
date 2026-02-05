pub mod cli;
pub mod commands;
pub mod computer_use;
pub mod computer_use_real;
pub mod project_context;
pub mod agent_onboarding;
pub mod external_access;
pub mod agentic_ai_core;
pub mod advanced_reasoning;
pub mod real_time_vision;
pub mod logging;
pub mod project_tracker;
pub mod recipes;
pub mod scenario_tests;
pub mod session;
pub mod signal;

// Re-export commonly used types
pub use session::CliSession;
pub use project_context::ProjectContextManager;
pub use computer_use_real::ComputerUseController;
