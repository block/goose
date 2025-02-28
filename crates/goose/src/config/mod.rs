mod base;
mod experiments;
mod extensions;

pub use crate::agents::ExtensionConfig;
pub use base::{Config, ConfigError, APP_STRATEGY};
pub use experiments::ExperimentManager;
pub use extensions::{ExtensionEntry, ExtensionManager};

pub use extensions::DEFAULT_EXTENSION;
pub use extensions::DEFAULT_EXTENSION_TIMEOUT;
