use anyhow::{Context, Result};
use std::sync::Once;
use tracing_appender::rolling::Rotation;
use tracing_subscriber::{
    filter::LevelFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
    Registry,
};

use goose::otel::otlp;
use goose::tracing::langfuse_layer;

// Used to ensure we only set up tracing once
static INIT: Once = Once::new();

/// Sets up the logging infrastructure for the application.
/// This includes:
/// - File-based logging with JSON formatting (DEBUG level)
/// - No console output (all logs go to files only)
/// - Optional Langfuse integration (DEBUG level)
pub fn setup_logging(name: Option<&str>) -> Result<()> {
    setup_logging_internal(name, false)
}

/// Internal function that allows bypassing the Once check for testing
fn setup_logging_internal(name: Option<&str>, force: bool) -> Result<()> {
    let mut result = Ok(());

    let mut setup = || {
        result = (|| {
            let log_dir = goose::logging::prepare_log_directory("cli", true)?;
            let timestamp = chrono::Local::now().format("%Y%m%d_%H%M%S").to_string();
            let log_filename = if let Some(n) = name {
                format!("{}-{}.log", timestamp, n)
            } else {
                format!("{}.log", timestamp)
            };
            let file_appender = tracing_appender::rolling::RollingFileAppender::new(
                Rotation::NEVER, // we do manual rotation via file naming and cleanup_old_logs
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

            // Base filter
            let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                // Set default levels for different modules
                EnvFilter::new("")
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
                // Console logging disabled for CLI - all logs go to files only
            ];

            if !force {
                layers.extend(otlp::init_otlp_layers(goose::config::Config::global()));
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
        let log_dir = goose::logging::prepare_log_directory("cli", true).unwrap();
        assert!(log_dir.exists());
        assert!(log_dir.is_dir());

        // Verify directory structure
        let path_components: Vec<_> = log_dir.components().collect();
        assert!(path_components.iter().any(|c| c.as_os_str() == "goose"));
        assert!(path_components.iter().any(|c| c.as_os_str() == "logs"));
        assert!(path_components.iter().any(|c| c.as_os_str() == "cli"));
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
