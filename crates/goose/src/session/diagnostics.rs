use crate::config::paths::Paths;
use crate::providers::utils::LOGS_TO_KEEP;
use crate::session::SessionManager;
use std::fs::{self};
use std::io::Cursor;
use std::io::Write;
use zip::write::FileOptions;
use zip::ZipWriter;

pub async fn generate_diagnostics(session_id: &str) -> anyhow::Result<Vec<u8>> {
    let logs_dir = Paths::in_state_dir("logs");
    let config_dir = Paths::config_dir();
    let config_path = config_dir.join("config.yaml");
    let data_dir = Paths::data_dir();

    let system_info = format!(
        "App Version: {}\n\
     OS: {}\n\
     OS Version: {}\n\
     Architecture: {}\n\
     Timestamp: {}\n",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::OS,
        sys_info::os_release().unwrap_or_else(|_| "unknown".to_string()),
        std::env::consts::ARCH,
        chrono::Utc::now().to_rfc3339()
    );

    let mut buffer = Vec::new();
    {
        let mut zip = ZipWriter::new(Cursor::new(&mut buffer));
        let options = FileOptions::default().compression_method(zip::CompressionMethod::Deflated);

        let mut log_files: Vec<_> = fs::read_dir(&logs_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().is_some_and(|ext| ext == "jsonl"))
            .collect();

        log_files.sort_by_key(|e| e.metadata().ok().and_then(|m| m.modified().ok()));

        for entry in log_files.iter().rev().take(LOGS_TO_KEEP) {
            let path = entry.path();
            let name = path.file_name().unwrap().to_str().unwrap();
            zip.start_file(format!("logs/{}", name), options)?;
            zip.write_all(&fs::read(&path)?)?;
        }

        let session_data = SessionManager::export_session(session_id).await?;
        zip.start_file("session.json", options)?;
        zip.write_all(session_data.as_bytes())?;

        if config_path.exists() {
            zip.start_file("config.yaml", options)?;
            zip.write_all(&fs::read(&config_path)?)?;
        }

        zip.start_file("system.txt", options)?;
        zip.write_all(system_info.as_bytes())?;

        let schedule_json = data_dir.join("schedule.json");
        if schedule_json.exists() {
            zip.start_file("schedule.json", options)?;
            zip.write_all(&fs::read(&schedule_json)?)?;
        }

        let schedules_json = data_dir.join("schedules.json");
        if schedules_json.exists() {
            zip.start_file("schedules.json", options)?;
            zip.write_all(&fs::read(&schedules_json)?)?;
        }

        let scheduled_recipes_dir = data_dir.join("scheduled_recipes");
        if scheduled_recipes_dir.exists() && scheduled_recipes_dir.is_dir() {
            for entry in fs::read_dir(&scheduled_recipes_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.is_file() {
                    let name = path.file_name().unwrap().to_str().unwrap();
                    zip.start_file(format!("scheduled_recipes/{}", name), options)?;
                    zip.write_all(&fs::read(&path)?)?;
                }
            }
        }

        zip.finish()?;
    }

    Ok(buffer)
}
