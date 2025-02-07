mod base;
mod extensions;

pub use crate::agents::ExtensionConfig;
pub use base::{Config, ConfigError, APP_STRATEGY};
pub use extensions::{ExtensionEntry, ExtensionManager};
