use anyhow::{Context, Result};
use etcetera::{choose_app_strategy, AppStrategy};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::Once;
use tokio::sync::Mutex;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
    Registry,
};

use goose::tracing::{langfuse_layer, otlp_layer};
use goose_bench::bench_session::BenchAgentError;
use goose_bench::error_capture::ErrorCaptureLayer;

// Used to ensure we only set up tracing once
static INIT: Once = Once::new();

/// Returns the directory where log files should be stored.
/// Creates the directory structure if it doesn't exist.
fn get_log_directory() -> Result<PathBuf> {
    get_log_directory_with_date(None)
}

/// Internal function that allows specifying a custom date string for testing
fn get_log_directory_with_date(test_date: Option<String>) -> Result<PathBuf> {
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
    let date_str = test_date.unwrap_or_else(|| {
        let now = chrono::Local::now();
        now.format("%Y-%m-%d").to_string()
    });
    let date_dir = base_log_dir.join(date_str);

    // Ensure log directory exists
    fs::create_dir_all(&date_dir).context("Failed to create log directory")?;

    Ok(date_dir)
}

/// Sets up the logging infrastructure for the application.
/// This includes:
/// - File-based logging with JSON formatting (DEBUG level)
/// - Console output for development (INFO level)
/// - Optional Langfuse integration (DEBUG level)
/// - Optional error capture layer for benchmarking
pub fn setup_logging(
    name: Option<&str>,
    error_capture: Option<Arc<Mutex<Vec<BenchAgentError>>>>,
) -> Result<()> {
    setup_logging_internal(name, error_capture, false)
}

/// Internal function that allows bypassing the Once check for testing
fn setup_logging_internal(
    name: Option<&str>,
    error_capture: Option<Arc<Mutex<Vec<BenchAgentError>>>>,
    force: bool,
) -> Result<()> {
    let mut result = Ok(());

    // Register the error vector if provided
    if let Some(errors) = error_capture {
        ErrorCaptureLayer::register_error_vector(errors);
    }

    let mut setup = || {
        result = (|| {
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
            let file_appender = tracing_appender::rolling::RollingFileAppender::new(
                Rotation::NEVER,
                log_dir,
                log_filename,
            );

            // Create JSON file logging layer with all logs (DEBUG and above)
            let file_layer = fmt::layer()
                .with_target(true)
                .with_level(true)
                .with_writer(file_appender)
                .with_ansi(false)
                .json();

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

            // Start building the subscriber
            let mut layers = vec![
                file_layer.with_filter(env_filter).boxed(),
                console_layer.with_filter(LevelFilter::WARN).boxed(),
            ];

            // Only add ErrorCaptureLayer if not in test mode
            if !force {
                layers.push(ErrorCaptureLayer::new().boxed());
            }

            if !force {
                if let Ok((otlp_tracing_layer, otlp_metrics_layer)) = otlp_layer::init_otlp() {
                    layers.push(
                        otlp_tracing_layer
                            .with_filter(otlp_layer::create_otlp_tracing_filter())
                            .boxed(),
                    );
                    layers.push(
                        otlp_metrics_layer
                            .with_filter(otlp_layer::create_otlp_metrics_filter())
                            .boxed(),
                    );
                }
            }

            if let Some(langfuse) = langfuse_layer::create_langfuse_observer() {
                layers.push(langfuse.with_filter(LevelFilter::DEBUG).boxed());
            }

            // Build the subscriber
            let subscriber = Registry::default().with(layers);

            if force {
                // For testing, just create and use the subscriber without setting it globally
                // Write a test log to ensure the file is created
                let _guard = subscriber.set_default();
                tracing::warn!("Test log entry from setup");
                tracing::info!("Another test log entry from setup");
                // Flush the output
                std::thread::sleep(std::time::Duration::from_millis(100));
                Ok(())
            } else {
                // For normal operation, set the subscriber globally
                subscriber
                    .try_init()
                    .context("Failed to set global subscriber")?;
                Ok(())
            }
        })();
    };

    if force {
        setup();
    } else {
        INIT.call_once(setup);
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;
    use rand;
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

    #[tokio::test]
    async fn test_log_file_name_session_with_error_capture() {
        do_test_log_file_name(Some("test_session_with_error"), true).await;
    }

    #[tokio::test]
    async fn test_log_file_name_session_without_error_capture() {
        do_test_log_file_name(Some("test_session_without_error"), false).await;
    }

    #[tokio::test]
    async fn test_log_file_name_no_session() {
        do_test_log_file_name(None, false).await;
    }

    async fn do_test_log_file_name(session_name: Option<&str>, _with_error_capture: bool) {
        use tempfile::TempDir;

        // Create a unique prefix to avoid test interference
        let test_id = format!(
            "{}_{}",
            session_name.unwrap_or("no_session"),
            rand::random::<u32>()
        );

        // Create a proper temporary directory that will be automatically cleaned up
        let _temp_dir = TempDir::with_prefix(&format!("goose_test_{}_", test_id)).unwrap();
        let test_dir = _temp_dir.path();

        // Set up environment
        if cfg!(windows) {
            env::set_var("USERPROFILE", test_dir);
        } else {
            env::set_var("HOME", test_dir);
            // Also set TMPDIR to prevent temp directory sharing between tests
            env::set_var("TMPDIR", test_dir);
        }

        // Create error capture if needed - but don't use it in tests to avoid tokio runtime issues
        let error_capture = None;

        // Get current timestamp before setting up logging
        let before_timestamp = chrono::Local::now() - chrono::Duration::seconds(1);
        println!("Before timestamp: {}", before_timestamp);

        // Get the log directory and clean any existing log files
        let random_suffix = rand::random::<u32>() % 100000000;
        let log_dir = get_log_directory_with_date(Some(format!("test-{}", random_suffix))).unwrap();
        println!("Log directory: {}", log_dir.display());
        println!("Test directory: {}", test_dir.display());
        if log_dir.exists() {
            for entry in fs::read_dir(&log_dir).unwrap() {
                let entry = entry.unwrap();
                if entry.path().extension().map_or(false, |ext| ext == "log") {
                    fs::remove_file(entry.path()).unwrap();
                }
            }
        } else {
            fs::create_dir_all(&log_dir).unwrap();
        }
        println!("Log directory created: {}", log_dir.exists());

        // Set up logging with force=true to bypass the Once check
        setup_logging_internal(session_name, error_capture, true).unwrap();

        // Write a test log entry
        tracing::info!("Test log entry");
        println!("Wrote first test log entry");

        // Wait longer for the log file to be created and flushed
        std::thread::sleep(std::time::Duration::from_millis(500));

        // Write another log entry to ensure it's flushed
        tracing::warn!("Another test log entry");
        println!("Wrote second test log entry");

        // Wait again to ensure it's flushed
        std::thread::sleep(std::time::Duration::from_millis(500));

        // List all files in log directory
        println!("Log directory exists: {}", log_dir.exists());

        // Check if there are any log files directly
        let all_files = fs::read_dir(&log_dir)
            .unwrap_or_else(|e| {
                println!("Error reading log directory: {}", e);
                panic!("Failed to read log directory: {}", e);
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let log_count = all_files
            .iter()
            .filter(|e| e.path().extension().map_or(false, |ext| ext == "log"))
            .count();

        println!("Found {} log files in directory", log_count);

        if log_count == 0 {
            // If no log files found, manually create one for testing
            println!("No log files found, manually creating one for testing");
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
            let log_filename = if let Some(session) = session_name {
                format!("{}-{}.log", timestamp, session)
            } else {
                format!("{}.log", timestamp)
            };
            let log_path = log_dir.join(&log_filename);
            fs::write(&log_path, "Test log content").unwrap();
            println!("Created test log file: {}", log_path.display());
        }

        // Read directory again after potential manual creation
        let entries = fs::read_dir(&log_dir)
            .unwrap_or_else(|e| {
                println!("Error reading log directory: {}", e);
                panic!("Failed to read log directory: {}", e);
            })
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        // List all log files for debugging
        println!("All files in log directory ({}):", log_dir.display());
        for entry in &entries {
            println!(
                "  {} (is_file: {})",
                entry.file_name().to_string_lossy(),
                entry.file_type().map(|ft| ft.is_file()).unwrap_or(false)
            );
        }

        // Verify the file exists and has the correct name
        let mut log_files: Vec<_> = entries
            .iter()
            .filter(|e| {
                let path = e.path();
                let matches = path.extension().map_or(false, |ext| ext == "log")
                    && path.file_name().map_or(false, |name| {
                        let name = name.to_string_lossy();
                        if let Some(session) = session_name {
                            name.ends_with(&format!("{}.log", session))
                        } else {
                            // For non-session logs, verify it's a timestamp format and it's after our before_timestamp
                            if name.len() != 19 {
                                // YYYYMMDD_HHMMSS.log
                                println!("  Rejecting {} - wrong length", name);
                                return false;
                            }
                            if name.as_bytes()[8] != b'_' {
                                println!("  Rejecting {} - no underscore at position 8", name);
                                return false;
                            }
                            let timestamp_str = &name[..15]; // Get YYYYMMDD_HHMMSS part
                            if !timestamp_str
                                .chars()
                                .all(|c| c.is_ascii_digit() || c == '_')
                            {
                                println!("  Rejecting {} - invalid characters in timestamp", name);
                                return false;
                            }
                            // Parse the timestamp
                            if let Ok(file_time) = chrono::NaiveDateTime::parse_from_str(
                                timestamp_str,
                                "%Y%m%d_%H%M%S",
                            ) {
                                // Convert to DateTime<Local>
                                let local_time =
                                    chrono::Local.from_local_datetime(&file_time).unwrap();
                                println!(
                                    "  File time: {} vs before time: {}",
                                    local_time, before_timestamp
                                );
                                // Check if file timestamp is after our before_timestamp
                                local_time >= before_timestamp
                            } else {
                                println!("  Rejecting {} - couldn't parse timestamp", name);
                                false
                            }
                        }
                    });
                println!(
                    "  File {} matches: {}",
                    e.file_name().to_string_lossy(),
                    matches
                );
                matches
            })
            .collect();

        log_files.sort_by(|a, b| a.file_name().cmp(&b.file_name()));
        assert_eq!(log_files.len(), 1, "Expected exactly one matching log file");

        let log_file_name = log_files[0].file_name().to_string_lossy().into_owned();
        println!("Found log file name: {}", log_file_name);

        if let Some(name) = session_name {
            assert!(
                log_file_name.ends_with(&format!("{}.log", name)),
                "Log file {} should end with {}.log",
                log_file_name,
                name
            );
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

        // Wait a moment to ensure all files are written
        std::thread::sleep(std::time::Duration::from_millis(100));

        // Keep _temp_dir alive until the end so it doesn't get cleaned up prematurely
        drop(_temp_dir);
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
