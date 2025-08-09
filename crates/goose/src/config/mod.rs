pub mod base;
pub mod env_registry;
mod experiments;
pub mod extensions;
pub mod permission;
pub mod signup_openrouter;

pub use crate::agents::ExtensionConfig;
pub use base::{Config, ConfigError, APP_STRATEGY};
pub use env_registry::{
    discover_extension_env_vars, discover_provider_env_vars, EnvCategory, EnvRegistry, EnvVarSpec,
    TrackedValue, ValueSource, ENV_REGISTRY, KNOWN_ENV_VARS,
};
pub use experiments::ExperimentManager;
pub use extensions::{ExtensionConfigManager, ExtensionEntry};
pub use permission::PermissionManager;
pub use signup_openrouter::configure_openrouter;

pub use extensions::DEFAULT_DISPLAY_NAME;
pub use extensions::DEFAULT_EXTENSION;
pub use extensions::DEFAULT_EXTENSION_DESCRIPTION;
pub use extensions::DEFAULT_EXTENSION_TIMEOUT;
