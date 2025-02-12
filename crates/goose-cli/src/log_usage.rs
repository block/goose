use etcetera::{choose_app_strategy, AppStrategy};
use goose::providers::base::ProviderUsage;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct SessionLog {
    session_file: String,
    usage: Vec<ProviderUsage>,
}

pub fn log_usage(session_file: String, usage: Vec<ProviderUsage>) {
    let log = SessionLog {
        session_file,
        usage,
    };

    // Ensure log directory exists
    if let Ok(home_dir) = choose_app_strategy(crate::APP_STRATEGY.clone()) {
        // choose_app_strategy().state_dir()
        // - macOS/Linux: ~/.local/state/goose/logs/
        // - Windows:     ~\AppData\Roaming\Block\goose\data\logs
        // - Windows has no convention for state_dir, use data_dir instead
        let log_dir = home_dir
            .in_state_dir("logs")
            .unwrap_or_else(|| home_dir.in_data_dir("logs"));

        if let Err(e) = std::fs::create_dir_all(&log_dir) {
            eprintln!("Failed to create log directory: {}", e);
            return;
        }

        let log_file = log_dir.join("goose.log");
        let serialized = match serde_json::to_string(&log) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("Failed to serialize usage log: {}", e);
                return;
            }
        };

        // Append to log file
        if let Err(e) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_file)
            .and_then(|mut file| {
                std::io::Write::write_all(&mut file, serialized.as_bytes())?;
                std::io::Write::write_all(&mut file, b"\n")?;
                Ok(())
            })
        {
            eprintln!("Failed to write to usage log file: {}", e);
        }
    } else {
        eprintln!("Failed to write to usage log file: Failed to determine home directory");
    }
}

#[cfg(test)]
mod tests {
    use etcetera::{choose_app_strategy, AppStrategy};
    use goose::providers::base::{ProviderUsage, Usage};

    use crate::log_usage::{log_usage, SessionLog};

    pub fn run_with_tmp_dir<F: FnOnce() -> T, T>(func: F) -> T {
        use tempfile::tempdir;

        let temp_dir = tempdir().unwrap();
        let temp_dir_path = temp_dir.path().to_path_buf();

        temp_env::with_vars([("HOME", Some(temp_dir_path.as_os_str()))], func)
    }

    #[test]
    fn test_session_logging() {
        run_with_tmp_dir(|| {
            let home_dir = choose_app_strategy(crate::APP_STRATEGY.clone()).unwrap();
            let log_file = home_dir
                .in_state_dir("logs")
                .unwrap_or_else(|| home_dir.in_data_dir("logs"))
                .join("goose.log");

            log_usage(
                "path.txt".to_string(),
                vec![ProviderUsage::new(
                    "model".to_string(),
                    Usage::new(Some(10), Some(20), Some(30)),
                )],
            );

            // Check if log file exists and contains the expected content
            assert!(log_file.exists(), "Log file should exist");

            let log_content = std::fs::read_to_string(&log_file).unwrap();
            let log: SessionLog = serde_json::from_str(&log_content).unwrap();

            assert!(log.session_file.contains("path.txt"));
            assert_eq!(log.usage[0].usage.input_tokens, Some(10));
            assert_eq!(log.usage[0].usage.output_tokens, Some(20));
            assert_eq!(log.usage[0].usage.total_tokens, Some(30));
            assert_eq!(log.usage[0].model, "model");

            // Remove the log file after test
            std::fs::remove_file(&log_file).ok();
        })
    }
}
