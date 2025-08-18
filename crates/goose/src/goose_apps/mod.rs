//! Goose Apps - JavaScript apps management system
pub mod app;
pub mod manager;
pub mod mcp_server;
pub mod service;

pub use app::GooseApp;
pub use manager::GooseAppsManager;
pub use mcp_server::GooseAppsClient;
pub use service::{goose_app_from_json, GooseAppUpdates, GooseAppsError, GooseAppsService};
