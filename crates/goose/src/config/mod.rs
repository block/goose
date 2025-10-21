pub mod base;
pub mod declarative_providers;
mod experiments;
pub mod extensions;
pub mod paths;
pub mod permission;
pub mod signup_openrouter;
pub mod signup_tetrate;

pub use crate::agents::ExtensionConfig;
pub use base::{Config, ConfigError};
pub use declarative_providers::DeclarativeProviderConfig;
pub use experiments::ExperimentManager;
pub use extensions::{
    get_all_extension_names, get_all_extensions, get_enabled_extensions, get_extension_by_name,
    is_extension_enabled, remove_extension, set_extension, set_extension_enabled, ExtensionEntry,
};
pub use permission::PermissionManager;
use serde::Deserialize;
use serde::Serialize;
pub use signup_openrouter::configure_openrouter;
pub use signup_tetrate::configure_tetrate;

pub use extensions::DEFAULT_DISPLAY_NAME;
pub use extensions::DEFAULT_EXTENSION;
pub use extensions::DEFAULT_EXTENSION_DESCRIPTION;
pub use extensions::DEFAULT_EXTENSION_TIMEOUT;

#[derive(Copy, Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GooseMode {
    Auto,
    Approve,
    SmartApprove,
    Chat,
}

impl TryFrom<&String> for GooseMode {
    type Error = String;

    fn try_from(value: &String) -> Result<Self, Self::Error> {
        match value.as_str() {
            "auto" => Ok(GooseMode::Auto),
            "approve" => Ok(GooseMode::Approve),
            "smart_approve" => Ok(GooseMode::SmartApprove),
            "chat" => Ok(GooseMode::Chat),
            _ => Err(format!("invalid mode: {}", value)),
        }
    }
}
