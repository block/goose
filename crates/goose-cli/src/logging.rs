use anyhow::{Context, Result};
use etcetera::{choose_app_strategy, AppStrategy};
use std::fs;
use std::path::PathBuf;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
    Registry,
};

use goose::tracing::langfuse_layer;

/// Returns the directory where log files should be stored.
/// Creates the directory structure if it doesn't exist.
fn get_log_directory() -> Result<PathBuf> {
    // choose_app_strategy().state_dir()
    // - macOS/Linux: ~/.local/state/goose/logs/cli
    // - Windows:     ~\AppData\Roaming\Block\goose\data\logs\cli
    // - Windows has no convention for state_dir, use data_dir instead
    let home_dir = choose_app_strategy(crate::APP_STRATEGY.clone())
        .context("HOME environment variable not set")?;

    let base_log_dir = home_dir
        .in_state_dir("logs/cli")
        .unwrap_or_else(|| home_dir.in_data_dir("logs/cli"));

    // Create date-based subdirectory
    let now = chrono::Local::now();
    let date_dir = base_log_dir.join(now.format("%Y-%m-%d").to_string());

    // Ensure log directory exists
    fs::create_dir_all(&date_dir).context("Failed to create log directory")?;

    Ok(date_dir)
}

/// Sets up the logging infrastructure for the application.
/// This includes:
/// - File-based logging with JSON formatting (DEBUG level)
/// - Console output for development (INFO level)
/// - Optional Langfuse integration (DEBUG level)
pub fn setup_logging(name: Option<&str>) -> Result<()> {
    // Set up file appender for goose module logs
    let log_dir = get_log_directory()?;
    let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();

    // Create log file name by prefixing with timestamp
    let log_filename = if name.is_some() {
        format!("{}-{}.log", timestamp, name.unwrap())
    } else {
        format!("{}.log", timestamp)
    };

    // Create non-rolling file appender for detailed logs
    let file_appender =
        tracing_appender::rolling::RollingFileAppender::new(Rotation::NEVER, log_dir, log_filename);

    // Create JSON file logging layer with all logs (DEBUG and above)
    let file_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_writer(file_appender)
        .with_ansi(false)
        .with_file(true)
        .pretty();

    // Create console logging layer for development - INFO and above only
    let console_layer = fmt::layer()
        .with_target(true)
        .with_level(true)
        .with_ansi(true)
        .with_file(true)
        .with_line_number(true)
        .pretty();

    // Base filter
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
        // Set default levels for different modules
        EnvFilter::new("")
            // Set mcp-server module to DEBUG
            .add_directive("mcp_server=debug".parse().unwrap())
            // Set mcp-client to DEBUG
            .add_directive("mcp_client=debug".parse().unwrap())
            // Set goose module to DEBUG
            .add_directive("goose=debug".parse().unwrap())
            // Set goose-cli to INFO
            .add_directive("goose_cli=info".parse().unwrap())
            // Set everything else to WARN
            .add_directive(LevelFilter::WARN.into())
    });

    // Build the subscriber with required layers
    let subscriber = Registry::default()
        .with(file_layer.with_filter(env_filter)) // Gets all logs
        .with(console_layer.with_filter(LevelFilter::WARN)); // Controls log levels

    // Initialize with Langfuse if available
    if let Some(langfuse) = langfuse_layer::create_langfuse_observer() {
        subscriber
            .with(langfuse.with_filter(LevelFilter::DEBUG))
            .try_init()
            .context("Failed to set global subscriber")?;
    } else {
        subscriber
            .try_init()
            .context("Failed to set global subscriber")?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::env;
    use tempfile::TempDir;
    use test_case::test_case;
    use tokio::runtime::Runtime;

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
    fn test_log_directory_creation() {
        let _temp_dir = setup_temp_home();
        let log_dir = get_log_directory().unwrap();
        assert!(log_dir.exists());
        assert!(log_dir.is_dir());

        // Verify directory structure
        let path_components: Vec<_> = log_dir.components().collect();
        assert!(path_components.iter().any(|c| c.as_os_str() == "goose"));
        assert!(path_components.iter().any(|c| c.as_os_str() == "logs"));
        assert!(path_components.iter().any(|c| c.as_os_str() == "cli"));
    }

    #[test_case(Some("test_session") ; "with session name")]
    #[test_case(None ; "without session name")]
    fn test_log_file_name(session_name: Option<&str>) {
        let _rt = Runtime::new().unwrap();
        let _temp_dir = setup_temp_home();

        // Create a test-specific log directory and file
        let log_dir = get_log_directory().unwrap();
        let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
        let file_name = format!("{}.log", session_name.unwrap_or(&timestamp));

        // Create the log file
        let file_path = log_dir.join(&file_name);
        fs::write(&file_path, "test").unwrap();

        // Verify the file exists and has the correct name
        let entries = fs::read_dir(log_dir).unwrap();
        let log_files: Vec<_> = entries
            .filter_map(Result::ok)
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "log"))
            .collect();

        assert_eq!(log_files.len(), 1, "Expected exactly one log file");

        let log_file_name = log_files[0].file_name().to_string_lossy().into_owned();
        println!("Log file name: {}", log_file_name);

        if let Some(name) = session_name {
            assert_eq!(log_file_name, format!("{}.log", name));
        } else {
            // Extract just the filename without extension for comparison
            let name_without_ext = log_file_name.trim_end_matches(".log");
            // Verify it's a valid timestamp format
            assert_eq!(
                name_without_ext.len(),
                15,
                "Expected 15 characters (YYYYMMDD_HHMMSS)"
            );
            assert!(
                name_without_ext[8..9].contains('_'),
                "Expected underscore at position 8"
            );
            assert!(
                name_without_ext
                    .chars()
                    .all(|c| c.is_ascii_digit() || c == '_'),
                "Expected only digits and underscore"
            );
        }
    }

    #[tokio::test]
    async fn test_langfuse_layer_creation() {
        let _temp_dir = setup_temp_home();

        // Store original environment variables (both sets)
        let original_vars = [
            ("LANGFUSE_PUBLIC_KEY", env::var("LANGFUSE_PUBLIC_KEY").ok()),
            ("LANGFUSE_SECRET_KEY", env::var("LANGFUSE_SECRET_KEY").ok()),
            ("LANGFUSE_URL", env::var("LANGFUSE_URL").ok()),
            (
                "LANGFUSE_INIT_PROJECT_PUBLIC_KEY",
                env::var("LANGFUSE_INIT_PROJECT_PUBLIC_KEY").ok(),
            ),
            (
                "LANGFUSE_INIT_PROJECT_SECRET_KEY",
                env::var("LANGFUSE_INIT_PROJECT_SECRET_KEY").ok(),
            ),
        ];

        // Clear all Langfuse environment variables
        for (var, _) in &original_vars {
            env::remove_var(var);
        }

        // Test without any environment variables
        assert!(langfuse_layer::create_langfuse_observer().is_none());

        // Test with standard Langfuse variables
        env::set_var("LANGFUSE_PUBLIC_KEY", "test_public_key");
        env::set_var("LANGFUSE_SECRET_KEY", "test_secret_key");
        assert!(langfuse_layer::create_langfuse_observer().is_some());

        // Clear and test with init project variables
        env::remove_var("LANGFUSE_PUBLIC_KEY");
        env::remove_var("LANGFUSE_SECRET_KEY");
        env::set_var("LANGFUSE_INIT_PROJECT_PUBLIC_KEY", "test_public_key");
        env::set_var("LANGFUSE_INIT_PROJECT_SECRET_KEY", "test_secret_key");
        assert!(langfuse_layer::create_langfuse_observer().is_some());

        // Test fallback behavior
        env::remove_var("LANGFUSE_INIT_PROJECT_PUBLIC_KEY");
        assert!(langfuse_layer::create_langfuse_observer().is_none());

        // Restore original environment variables
        for (var, value) in original_vars {
            match value {
                Some(val) => env::set_var(var, val),
                None => env::remove_var(var),
            }
        }
    }
}
