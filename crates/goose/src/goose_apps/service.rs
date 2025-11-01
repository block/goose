use super::manager::GooseAppsManager;
use crate::goose_apps::GooseApp;
use anyhow::Result;
use serde_json::{json, Value};
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct GooseAppsService {
    manager: Arc<Mutex<GooseAppsManager>>,
}

#[derive(Debug)]
pub enum GooseAppsError {
    NotFound(String),
    AlreadyExists(String),
    InvalidImplementation(String),
    InvalidParameter(String),
    Internal(String),
}

impl std::fmt::Display for GooseAppsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            GooseAppsError::NotFound(msg) => write!(f, "Not found: {}", msg),
            GooseAppsError::AlreadyExists(msg) => write!(f, "Already exists: {}", msg),
            GooseAppsError::InvalidImplementation(msg) => {
                write!(f, "Invalid implementation: {}", msg)
            }
            GooseAppsError::InvalidParameter(msg) => write!(f, "Invalid parameter: {}", msg),
            GooseAppsError::Internal(msg) => write!(f, "Internal error: {}", msg),
        }
    }
}

impl std::error::Error for GooseAppsError {}

impl From<anyhow::Error> for GooseAppsError {
    fn from(err: anyhow::Error) -> Self {
        let err_str = err.to_string();
        if err_str.contains("not found") {
            GooseAppsError::NotFound(err_str)
        } else if err_str.contains("already exists") {
            GooseAppsError::AlreadyExists(err_str)
        } else if err_str.contains("extends GooseWidget") {
            GooseAppsError::InvalidImplementation(err_str)
        } else {
            GooseAppsError::Internal(err_str)
        }
    }
}

impl GooseAppsService {
    pub fn new() -> Result<Self> {
        let manager = GooseAppsManager::new()?;
        Ok(Self {
            manager: Arc::new(Mutex::new(manager)),
        })
    }

    pub async fn create_app(&self, app: &GooseApp) -> Result<String, GooseAppsError> {
        let manager = self.manager.lock().await;
        manager.update_app(app)?;
        Ok(format!("Successfully created Goose App: {}", app.name))
    }

    pub async fn update_app(
        &self,
        name: &str,
        updates: &GooseAppUpdates,
    ) -> Result<String, GooseAppsError> {
        let manager = self.manager.lock().await;

        // Get existing app to preserve fields not being updated
        let mut existing_app = manager
            .get_app(name)?
            .ok_or_else(|| GooseAppsError::NotFound(format!("App '{}' not found", name)))?;

        // Apply updates
        updates.apply_to(&mut existing_app);

        manager.update_app(&existing_app)?;
        Ok(format!("Successfully updated Goose App: {}", name))
    }

    pub async fn list_apps(&self) -> Result<Vec<GooseApp>, GooseAppsError> {
        let manager = self.manager.lock().await;
        Ok(manager.list_apps()?)
    }

    pub async fn get_app(&self, name: &str) -> Result<Option<GooseApp>, GooseAppsError> {
        let manager = self.manager.lock().await;
        Ok(manager.get_app(name)?)
    }

    pub async fn delete_app(&self, name: &str) -> Result<String, GooseAppsError> {
        let manager = self.manager.lock().await;
        manager.delete_app(name)?;
        Ok(format!("Successfully deleted Goose App: {}", name))
    }

    pub async fn app_exists(&self, name: &str) -> bool {
        let manager = self.manager.lock().await;
        manager.app_exists(name)
    }

    // Formatting helpers
    pub fn format_app_list(apps: &[GooseApp]) -> String {
        if apps.is_empty() {
            return "No Goose Apps found".to_string();
        }

        let mut result = vec!["Available Goose Apps:".to_string()];

        for app in apps.iter() {
            let description = app
                .description
                .as_ref()
                .map(|d| format!(" - {}", d))
                .unwrap_or_default();

            let dimensions = match (app.width, app.height) {
                (Some(w), Some(h)) => format!(" ({}x{})", w, h),
                _ => String::new(),
            };

            result.push(format!("• {}{}{}", app.name, dimensions, description));
        }

        result.join("\n")
    }

    pub fn format_app_details(app: &GooseApp) -> String {
        let info = json!({
            "name": app.name,
            "description": app.description,
            "width": app.width,
            "height": app.height,
            "resizable": app.resizable,
            "js_implementation": app.js_implementation
        });

        serde_json::to_string_pretty(&info).unwrap()
    }
}

#[derive(Debug, Default)]
pub struct GooseAppUpdates {
    pub js_implementation: Option<String>,
    pub description: Option<String>,
    pub width: Option<u32>,
    pub height: Option<u32>,
    pub resizable: Option<bool>,
}

impl GooseAppUpdates {
    pub fn from_json(arguments: &Value) -> Self {
        Self {
            js_implementation: arguments
                .get("js_implementation")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            description: arguments
                .get("description")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string()),
            width: arguments
                .get("width")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            height: arguments
                .get("height")
                .and_then(|v| v.as_u64())
                .map(|v| v as u32),
            resizable: arguments.get("resizable").and_then(|v| v.as_bool()),
        }
    }

    pub fn from_request_fields(
        js_implementation: Option<String>,
        description: Option<String>,
        width: Option<u32>,
        height: Option<u32>,
        resizable: Option<bool>,
    ) -> Self {
        Self {
            js_implementation,
            description,
            width,
            height,
            resizable,
        }
    }

    fn apply_to(&self, app: &mut GooseApp) {
        if let Some(ref js_implementation) = self.js_implementation {
            app.js_implementation = js_implementation.clone();
        }
        if let Some(ref description) = self.description {
            app.description = Some(description.clone());
        }
        if let Some(width) = self.width {
            app.width = Some(width);
        }
        if let Some(height) = self.height {
            app.height = Some(height);
        }
        if let Some(resizable) = self.resizable {
            app.resizable = Some(resizable);
        }
    }
}
