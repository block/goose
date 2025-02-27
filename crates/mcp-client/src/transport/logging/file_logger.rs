use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;
use chrono::Local;

use super::LogMessage;

pub struct FileLogger {
    file: Arc<Mutex<File>>,
}

impl FileLogger {
    pub fn new(path: PathBuf) -> std::io::Result<Self> {
        println!("Creating new FileLogger for path: {:?}", path);
        // Create parent directories if they don't exist
        if let Some(parent) = path.parent() {
            println!("Creating parent directories: {:?}", parent);
            std::fs::create_dir_all(parent)?;
        }

        println!("Opening log file...");
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(&path)?;

        println!("FileLogger successfully created");
        Ok(Self {
            file: Arc::new(Mutex::new(file)),
        })
    }

    pub async fn log(&self, message: &LogMessage) -> std::io::Result<()> {
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S%.3f");
        let log_line = format!(
            "[{}] [{}] {}: {}\n",
            timestamp,
            message.level,
            message.logger.as_deref().unwrap_or("unknown"),
            message.message
        );

        println!("Writing log line: {}", log_line.trim());
        let mut file = self.file.lock().await;
        file.write_all(log_line.as_bytes())?;
        file.flush()?;
        println!("Successfully wrote and flushed log line");

        Ok(())
    }
}