use anyhow::Result;
use cliclack::confirm;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, CellAlignment, ContentArrangement, Table};
use goose::config::paths::Paths;
use goose::session::session_manager::SessionManager;
use humansize::{format_size, BINARY};
use std::path::PathBuf;

pub async fn handle_db_status() -> Result<()> {
    let stats = SessionManager::get_database_stats().await?;

    println!("\n{}", "Goose Database Status".bold().cyan());
    println!("{}", "─".repeat(50));

    println!("\n{}:", "Database Location".green().bold());
    println!("  Path: {}", stats.db_path.display());
    println!("  Size: {}", format_size(stats.db_size, BINARY));

    println!("\n{}:", "Schema".green().bold());
    let version_status = if stats.is_latest_version {
        format!("{} (up to date ✓)", stats.schema_version).bright_green()
    } else {
        format!("{} (update available)", stats.schema_version).yellow()
    };
    println!("  Version: {}", version_status);

    println!("\n{}:", "Content".green().bold());
    println!("  Sessions: {}", stats.session_count);
    println!("  Messages: {}", stats.message_count);

    if stats.total_tokens > 0 {
        println!("\n{}:", "Token Usage".green().bold());
        println!("  Total: {}", stats.total_tokens.to_string().bright_cyan());
        if stats.session_count > 0 {
            let avg = stats.total_tokens / stats.session_count as i64;
            println!("  Average per session: {}", avg.to_string().bright_cyan());
        }
    }

    println!("\n{}:", "Backup Information".green().bold());
    println!("  Backup directory: {}", stats.backup_dir.display());

    if let Some(backup) = stats.latest_backup {
        let filename = backup
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let age = chrono::Utc::now() - backup.created_at;
        let age_str = if age.num_days() > 0 {
            format!("{} days ago", age.num_days())
        } else if age.num_hours() > 0 {
            format!("{} hours ago", age.num_hours())
        } else {
            format!("{} minutes ago", age.num_minutes())
        };
        println!("  Latest backup: {} ({})", filename, age_str);
        println!("    Size: {}", format_size(backup.size, BINARY));
    } else {
        println!("  Latest backup: {}", "None".yellow());
    }
    println!("  Number of backups: {}", stats.backup_count);

    println!();

    Ok(())
}

pub async fn handle_db_backup(name: Option<String>) -> Result<()> {
    println!("{}", "Creating database backup...".cyan());

    let backup_path = SessionManager::create_backup(name).await?;

    let size = std::fs::metadata(&backup_path)?.len();

    println!("\n{}", "✓ Backup created successfully".green().bold());
    println!("  Location: {}", backup_path.display());
    println!("  Size: {}", format_size(size, BINARY));
    println!();

    Ok(())
}

pub async fn handle_db_restore(backup_file: PathBuf, force: bool) -> Result<()> {
    let backup_path = if backup_file.components().count() == 1 {
        Paths::backup_dir().join(&backup_file)
    } else {
        backup_file.clone()
    };

    if !backup_path.exists() {
        if backup_file.components().count() == 1 {
            anyhow::bail!(
                "Backup file '{}' not found in backup directory.\nRun 'goose db list-backups' to see available backups.",
                backup_file.display()
            );
        } else {
            anyhow::bail!("Backup file not found: {}", backup_path.display());
        }
    }

    println!("{}", "Database Restore".bold().cyan());
    println!("{}", "─".repeat(50));
    println!("\n{}:", "Source".green());
    println!("  {}", backup_path.display());

    let stats = SessionManager::get_database_stats().await?;
    println!("\n{}:", "Current Database".yellow());
    println!("  Schema version: {}", stats.schema_version);
    println!("  Sessions: {}", stats.session_count);
    println!("  Messages: {}", stats.message_count);

    println!("\n{}", "⚠️  Warning:".yellow().bold());
    println!("  This will replace your current database with the backup.");
    println!("  A safety backup will be created first.");

    if !force {
        let should_restore = confirm("Do you want to proceed with the restore?")
            .initial_value(false)
            .interact()?;

        if !should_restore {
            println!("\n{}", "Restore cancelled.".yellow());
            return Ok(());
        }
    }

    println!("\n{}", "Restoring database...".cyan());

    SessionManager::restore_backup(&backup_path).await?;

    println!("\n{}", "✓ Database restored successfully".green().bold());
    println!(
        "  {}",
        "Important: Please restart goose for changes to take full effect."
            .yellow()
            .bold()
    );
    println!();

    Ok(())
}

pub async fn handle_db_path() -> Result<()> {
    let session_dir = goose::session::session_manager::ensure_session_dir().await?;
    let db_path = session_dir.join("sessions.db");
    println!("{}", db_path.display());
    Ok(())
}

pub async fn handle_db_list_backups(format: String) -> Result<()> {
    let backups = SessionManager::list_backups().await?;

    if backups.is_empty() {
        println!("{}", "No backups found.".yellow());
        return Ok(());
    }

    match format.as_str() {
        "json" => {
            println!("{}", serde_json::to_string_pretty(&backups)?);
        }
        "table" => {
            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec![
                    Cell::new("Filename").set_alignment(CellAlignment::Left),
                    Cell::new("Version").set_alignment(CellAlignment::Right),
                    Cell::new("Size").set_alignment(CellAlignment::Right),
                    Cell::new("Age").set_alignment(CellAlignment::Right),
                ]);

            for backup in &backups {
                let filename = backup
                    .path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown");

                let version_str = backup
                    .schema_version
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "?".to_string());

                let size_str = format_size(backup.size, BINARY);

                let age = chrono::Utc::now() - backup.created_at;
                let age_str = if age.num_days() > 0 {
                    format!("{} days ago", age.num_days())
                } else if age.num_hours() > 0 {
                    format!("{} hours ago", age.num_hours())
                } else if age.num_minutes() > 0 {
                    format!("{} mins ago", age.num_minutes())
                } else {
                    "just now".to_string()
                };

                table.add_row(vec![
                    Cell::new(filename).set_alignment(CellAlignment::Left),
                    Cell::new(version_str).set_alignment(CellAlignment::Right),
                    Cell::new(size_str).set_alignment(CellAlignment::Right),
                    Cell::new(age_str).set_alignment(CellAlignment::Right),
                ]);
            }

            println!("\n{}", "Database Backups".bold().cyan());
            println!("{}", table);
            println!("{} backups found\n", backups.len());
        }
        _ => {
            println!("Invalid format: {}", format);
            anyhow::bail!("Invalid format: {}", format);
        }
    }

    Ok(())
}
