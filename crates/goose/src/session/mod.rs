mod chat_history_search;
mod diagnostics;
pub mod extension_data;
mod extension_resolver;
mod legacy;
pub mod session_manager;

pub use diagnostics::{generate_diagnostics, get_system_info, SystemInfo};
pub use extension_data::{EnabledExtensionsState, ExtensionData, ExtensionState, TodoState};
pub use extension_resolver::resolve_extensions_for_new_session;
pub use session_manager::{
    Session, SessionInsights, SessionManager, SessionType, SessionUpdateBuilder,
};
