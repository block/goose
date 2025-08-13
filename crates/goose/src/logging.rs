use anyhow::{Context, Result};
use etcetera::{choose_app_strategy, AppStrategy};
use std::fs;
use std::path::PathBuf;

use crate::config::APP_STRATEGY;

/// Returns the base directory where log files should be stored.
/// This is the common base for all goose logging.
///
/// Returns:
/// - macOS/Linux: ~/.local/state/goose/logs
/// - Windows:     ~\AppData\Roaming\Block\goose\data\logs
pub fn get_base_log_directory() -> Result<PathBuf> {
    let home_dir =
        choose_app_strategy(APP_STRATEGY.clone()).context("HOME environment variable not set")?;

    let base_log_dir = home_dir
        .in_state_dir("logs")
        .unwrap_or_else(|| home_dir.in_data_dir("logs"));

    Ok(base_log_dir)
}

/// Returns the directory where log files should be stored for a specific component.
/// Creates the directory structure if it doesn't exist.
///
/// # Arguments
///
/// * `component` - The component name (e.g., "cli", "server", "debug")
/// * `use_date_subdir` - Whether to create a date-based subdirectory
///
/// # Returns
///
/// The path to the log directory for the specified component
pub fn get_log_directory(component: &str, use_date_subdir: bool) -> Result<PathBuf> {
    let base_log_dir = get_base_log_directory()?;
    let component_dir = base_log_dir.join(component);

    let log_dir = if use_date_subdir {
        // Create date-based subdirectory
        let now = chrono::Local::now();
        component_dir.join(now.format("%Y-%m-%d").to_string())
    } else {
        component_dir
    };

    // Ensure log directory exists
    fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    Ok(log_dir)
}

/// Returns the directory where log files should be stored for a specific component,
/// with a custom date string for testing purposes.
///
/// # Arguments
///
/// * `component` - The component name (e.g., "cli", "server", "debug")
/// * `test_date` - Optional custom date string for testing
///
/// # Returns
///
/// The path to the log directory for the specified component
#[cfg(test)]
pub fn get_log_directory_with_date(component: &str, test_date: Option<String>) -> Result<PathBuf> {
    let base_log_dir = get_base_log_directory()?;
    let component_dir = base_log_dir.join(component);

    let log_dir = if let Some(date_str) = test_date {
        component_dir.join(date_str)
    } else {
        // Create date-based subdirectory
        let now = chrono::Local::now();
        component_dir.join(now.format("%Y-%m-%d").to_string())
    };

    // Ensure log directory exists
    fs::create_dir_all(&log_dir).context("Failed to create log directory")?;

    Ok(log_dir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;

    fn setup_temp_home() -> TempDir {
        let temp_dir = TempDir::new().unwrap();
        if cfg!(windows) {
            env::set_var("USERPROFILE", temp_dir.path());
        } else {
            env::set_var("HOME", temp_dir.path());
        }
        temp_dir
    }

    #[test]
    fn test_base_log_directory() {
        let _temp_dir = setup_temp_home();
        let base_dir = get_base_log_directory().unwrap();
        assert!(base_dir.to_string_lossy().contains("goose"));
        assert!(base_dir.to_string_lossy().contains("logs"));
    }

    #[test]
    fn test_component_log_directory() {
        let _temp_dir = setup_temp_home();

        // Test without date subdirectory
        let log_dir = get_log_directory("test-component", false).unwrap();
        assert!(log_dir.exists());
        assert!(log_dir.is_dir());
        assert!(log_dir.to_string_lossy().contains("test-component"));

        // Test with date subdirectory
        let log_dir_with_date = get_log_directory("test-component", true).unwrap();
        assert!(log_dir_with_date.exists());
        assert!(log_dir_with_date.is_dir());
        assert!(log_dir_with_date
            .to_string_lossy()
            .contains("test-component"));

        // Should contain a date pattern
        let date_str = chrono::Local::now().format("%Y-%m-%d").to_string();
        assert!(log_dir_with_date.to_string_lossy().contains(&date_str));
    }

    #[test]
    fn test_directory_creation() {
        let _temp_dir = setup_temp_home();

        // Verify directory is created if it doesn't exist
        let log_dir = get_log_directory("new-component", true).unwrap();
        assert!(log_dir.exists());
        assert!(log_dir.is_dir());

        // Verify nested structure
        let path_components: Vec<_> = log_dir.components().collect();
        assert!(path_components.iter().any(|c| c.as_os_str() == "goose"));
        assert!(path_components.iter().any(|c| c.as_os_str() == "logs"));
        assert!(path_components
            .iter()
            .any(|c| c.as_os_str() == "new-component"));
    }

    #[test]
    fn test_custom_date_for_testing() {
        let _temp_dir = setup_temp_home();

        let custom_date = "2024-01-15".to_string();
        let log_dir =
            get_log_directory_with_date("test-component", Some(custom_date.clone())).unwrap();

        assert!(log_dir.exists());
        assert!(log_dir.to_string_lossy().contains(&custom_date));
    }
}
