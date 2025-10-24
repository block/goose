use anyhow::Result;
use clap::Subcommand;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Subcommand)]
pub enum LogCommand {
    /// Show current log size and path
    Size,
    /// Clear/archive log files
    Clear,
    /// Show current log configuration
    Config,
    /// Set log configuration
    Set {
        /// Archive threshold in GB
        #[arg(long)]
        archive_threshold_gb: Option<f64>,
        /// Delete threshold in GB
        #[arg(long)]
        delete_threshold_gb: Option<f64>,
        /// Check interval in seconds
        #[arg(long)]
        check_interval_secs: Option<u64>,
        /// Cooldown between actions in hours
        #[arg(long)]
        cooldown_hours: Option<u64>,
        /// Enable or disable auto-rotation
        #[arg(long)]
        auto_rotation: Option<bool>,
    },
}

#[derive(Deserialize)]
struct LogSizeResponse {
    total_bytes: u64,
    total_mb: f64,
    total_gb: f64,
    file_count: usize,
    log_path: String,
}

#[derive(Deserialize)]
struct LogConfig {
    archive_threshold_bytes: u64,
    delete_threshold_bytes: u64,
    check_interval_secs: u64,
    cooldown_secs: u64,
    auto_rotation_enabled: bool,
}

#[derive(Serialize)]
struct UpdateLogConfig {
    archive_threshold_bytes: Option<u64>,
    delete_threshold_bytes: Option<u64>,
    check_interval_secs: Option<u64>,
    cooldown_secs: Option<u64>,
    auto_rotation_enabled: Option<bool>,
}

async fn get_server_url() -> String {
    std::env::var("GOOSE_SERVER_URL").unwrap_or_else(|_| "http://localhost:3000".to_string())
}

pub async fn handle_log(command: LogCommand) -> Result<()> {
    let client = Client::new();
    let base_url = get_server_url().await;

    match command {
        LogCommand::Size => {
            let response: LogSizeResponse = client
                .get(&format!("{}/logs/size", base_url))
                .send()
                .await?
                .json()
                .await?;
            
            println!("Log Directory: {}", response.log_path);
            println!("Total Files: {}", response.file_count);
            println!("Total Size: {:.2} GB ({:.2} MB, {} bytes)", 
                    response.total_gb, response.total_mb, response.total_bytes);
        }
        LogCommand::Clear => {
            let response = client
                .post(&format!("{}/logs/clear", base_url))
                .send()
                .await?;
            
            if response.status().is_success() {
                println!("Logs cleared successfully.");
            } else {
                let error_text = response.text().await?;
                eprintln!("Failed to clear logs: {}", error_text);
                std::process::exit(1);
            }
        }
        LogCommand::Config => {
            let config: LogConfig = client
                .get(&format!("{}/logs/config", base_url))
                .send()
                .await?
                .json()
                .await?;
            
            println!("Auto Rotation: {}", if config.auto_rotation_enabled { "Enabled" } else { "Disabled" });
            println!("Archive Threshold: {:.2} GB", config.archive_threshold_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
            println!("Delete Threshold: {:.2} GB", config.delete_threshold_bytes as f64 / (1024.0 * 1024.0 * 1024.0));
            println!("Check Interval: {} seconds", config.check_interval_secs);
            println!("Cooldown: {} hours", config.cooldown_secs / 3600);
        }
        LogCommand::Set {
            archive_threshold_gb,
            delete_threshold_gb,
            check_interval_secs,
            cooldown_hours,
            auto_rotation,
        } => {
            let mut update = UpdateLogConfig {
                archive_threshold_bytes: archive_threshold_gb.map(|gb| (gb * 1024.0 * 1024.0 * 1024.0) as u64),
                delete_threshold_bytes: delete_threshold_gb.map(|gb| (gb * 1024.0 * 1024.0 * 1024.0) as u64),
                check_interval_secs,
                cooldown_secs: cooldown_hours.map(|h| h * 3600),
                auto_rotation_enabled: auto_rotation,
            };

            let response = client
                .put(&format!("{}/logs/config", base_url))
                .json(&update)
                .send()
                .await?;
            
            if response.status().is_success() {
                println!("Log configuration updated successfully.");
            } else {
                let error_text = response.text().await?;
                eprintln!("Failed to update log configuration: {}", error_text);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}