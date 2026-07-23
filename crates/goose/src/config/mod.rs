pub mod base;
pub mod declarative_providers;
mod experiments;
pub mod extensions;
mod migrations;
pub mod paths;
pub mod permission;
pub mod providers;
pub mod search_path;
pub mod signup_openrouter;
pub mod signup_tetrate;
pub mod tls;

pub use crate::agents::ExtensionConfig;
pub use base::{Config, ConfigError, merge_config_values};
pub use declarative_providers::DeclarativeProviderConfig;
pub use experiments::ExperimentManager;
pub use extensions::{
    ExtensionEntry, get_all_extension_names, get_all_extensions, get_available_extensions,
    get_enabled_extensions, get_extension_by_name, get_warnings, is_extension_enabled,
    remove_extension, resolve_extensions_for_new_session, set_extension, set_extension_enabled,
};
pub use goose_providers::goose_mode::GooseMode;
pub use permission::PermissionManager;
pub use signup_openrouter::configure_openrouter;
pub use signup_tetrate::configure_tetrate;

pub use extensions::DEFAULT_DISPLAY_NAME;
pub use extensions::DEFAULT_EXTENSION;
pub use extensions::DEFAULT_EXTENSION_DESCRIPTION;
pub use extensions::DEFAULT_EXTENSION_TIMEOUT;
pub use providers::{
    ProviderEntry, clear_active_provider, get_active_model, get_active_provider,
    get_provider_entry, set_active_provider, set_provider_entry,
};
