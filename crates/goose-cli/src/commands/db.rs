use anyhow::Result;
use chrono::{DateTime, Utc};
use cliclack::confirm;
use colored::Colorize;
use comfy_table::{presets::UTF8_FULL, Cell, CellAlignment, ContentArrangement, Table};
use goose::config::paths::Paths;
use goose::session::session_manager::SessionManager;
use humansize::{format_size, BINARY};
use std::path::PathBuf;

fn format_age(created_at: DateTime<Utc>) -> String {
    let age = chrono::Utc::now() - created_at;
    if age.num_days() > 0 {
        format!("{} days ago", age.num_days())
    } else if age.num_hours() > 0 {
        format!("{} hours ago", age.num_hours())
    } else if age.num_minutes() > 0 {
        format!("{} mins ago", age.num_minutes())
    } else {
        "just now".to_string()
    }
}

pub async fn handle_db_status() -> Result<()> {
    let stats = SessionManager::get_database_stats().await?;

    println!("\n{}", "Goose Database Status".bold().cyan());

    let mut db_info_table = Table::new();
    db_info_table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    db_info_table.add_row(vec!["Path", &stats.db_path.display().to_string()]);
    db_info_table.add_row(vec!["Size", &format_size(stats.db_size, BINARY)]);
    db_info_table.add_row(vec![
        "Schema Version",
        &if stats.is_latest_version {
            format!("{} (up to date ✓)", stats.schema_version)
        } else {
            format!("{} (update available)", stats.schema_version)
        },
    ]);
    db_info_table.add_row(vec![
        "Backup Directory",
        &stats.backup_dir.display().to_string(),
    ]);

    let mut stats_table = Table::new();
    stats_table
        .load_preset(UTF8_FULL)
        .set_content_arrangement(ContentArrangement::Dynamic);

    stats_table.add_row(vec!["Sessions", &stats.session_count.to_string()]);
    stats_table.add_row(vec!["Messages", &stats.message_count.to_string()]);
    if stats.total_tokens > 0 {
        stats_table.add_row(vec!["Total Tokens", &stats.total_tokens.to_string()]);
        if stats.session_count > 0 {
            stats_table.add_row(vec![
                "Avg Tokens/Session",
                &(stats.total_tokens / stats.session_count as i64).to_string(),
            ]);
        }
    }

    println!("\n{}", "Database Information".green().bold());
    println!("{}", db_info_table);

    println!("\n{}", "Statistics".green().bold());
    println!("{}", stats_table);

    if let Some(backup) = stats.latest_backup {
        let mut backup_table = Table::new();
        backup_table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);

        let filename = backup
            .path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown");
        let age_str = format_age(backup.created_at);

        backup_table.add_row(vec!["Latest Backup", filename]);
        backup_table.add_row(vec!["Age", &age_str]);
        backup_table.add_row(vec!["Size", &format_size(backup.size, BINARY)]);
        backup_table.add_row(vec!["Total Backups", &stats.backup_count.to_string()]);

        println!("\n{}", "Backup Information".green().bold());
        println!("{}", backup_table);
    } else {
        let mut backup_table = Table::new();
        backup_table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic);
        backup_table.add_row(vec!["Latest Backup", "None"]);
        backup_table.add_row(vec!["Total Backups", &stats.backup_count.to_string()]);

        println!("\n{}", "Backup Information".green().bold());
        println!("{}", backup_table);
    }

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
                    Cell::new("Schema Version").set_alignment(CellAlignment::Right),
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
                let age_str = format_age(backup.created_at);

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

async fn resolve_backup_files_for_deletion(
    backup_files: Vec<PathBuf>,
    all: bool,
    cleanup_only: bool,
) -> Result<Vec<PathBuf>> {
    if all {
        let backups = SessionManager::list_backups().await?;
        if backups.is_empty() {
            println!("{}", "No backups found to delete.".yellow());
            return Ok(vec![]);
        }
        return Ok(backups.into_iter().map(|b| b.path).collect());
    }

    if backup_files.is_empty() {
        if cleanup_only {
            return Ok(vec![]);
        }
        anyhow::bail!("No backup files specified. Use --all to delete all backups, or provide specific filenames.");
    }

    let backup_dir = Paths::backup_dir();
    let canonical_backup_dir = backup_dir
        .canonicalize()
        .map_err(|e| anyhow::anyhow!("Failed to resolve backup directory: {}", e))?;

    backup_files
        .into_iter()
        .map(|file| {
            let path = if file.components().count() == 1 {
                backup_dir.join(&file)
            } else {
                file.clone()
            };

            if !path.exists() {
                anyhow::bail!(
                    "Backup file '{}' not found{}",
                    file.display(),
                    if file.components().count() == 1 {
                        ". Run 'goose db list-backups' to see available backups."
                    } else {
                        ""
                    }
                );
            }

            let canonical_path = path.canonicalize().map_err(|e| {
                anyhow::anyhow!("Failed to resolve path '{}': {}", path.display(), e)
            })?;

            if !canonical_path.starts_with(&canonical_backup_dir) {
                anyhow::bail!(
                    "Security error: Path '{}' is outside the backup directory",
                    file.display()
                );
            }

            Ok(path)
        })
        .collect()
}

fn report_deletion_results(success_count: usize, failed_files: &[(&PathBuf, std::io::Error)]) {
    if success_count > 0 {
        println!(
            "\n{}",
            format!(
                "✓ Deleted {} backup{} successfully",
                success_count,
                if success_count == 1 { "" } else { "s" }
            )
            .green()
            .bold()
        );
    }

    if !failed_files.is_empty() {
        println!(
            "\n{} Failed to delete {} backup{}:",
            "⚠️".yellow(),
            failed_files.len(),
            if failed_files.len() == 1 { "" } else { "s" }
        );
        for (path, err) in failed_files {
            println!(
                "  {} - {}",
                path.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown"),
                err
            );
        }
    }
}

async fn cleanup_orphaned_files(backup_dir: &std::path::Path) -> Result<usize> {
    let mut orphaned_count = 0;
    let mut entries = tokio::fs::read_dir(backup_dir).await?;

    while let Some(entry) = entries.next_entry().await? {
        let path = entry.path();
        if let Some(ext) = path.extension() {
            if ext == "db-wal" || ext == "db-shm" {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let db_path = backup_dir.join(format!("{}.db", stem));
                    if !db_path.exists() && tokio::fs::remove_file(&path).await.is_ok() {
                        orphaned_count += 1;
                    }
                }
            }
        }
    }

    Ok(orphaned_count)
}

pub async fn handle_db_delete_backup(
    backup_files: Vec<PathBuf>,
    all: bool,
    cleanup: bool,
    force: bool,
) -> Result<()> {
    let files_to_delete = resolve_backup_files_for_deletion(backup_files, all, cleanup).await?;

    if !files_to_delete.is_empty() {
        let mut table = Table::new();
        table
            .load_preset(UTF8_FULL)
            .set_content_arrangement(ContentArrangement::Dynamic)
            .set_header(vec![
                Cell::new("Filename").set_alignment(CellAlignment::Left),
                Cell::new("Size").set_alignment(CellAlignment::Right),
                Cell::new("Age").set_alignment(CellAlignment::Right),
            ]);

        for path in &files_to_delete {
            let metadata = std::fs::metadata(path)?;
            let age_secs = std::time::SystemTime::now()
                .duration_since(metadata.modified()?)
                .unwrap_or_default()
                .as_secs();

            table.add_row(vec![
                Cell::new(
                    path.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown"),
                ),
                Cell::new(format_size(metadata.len(), BINARY)),
                Cell::new(match age_secs {
                    s if s > 86400 => format!("{} days ago", s / 86400),
                    s if s > 3600 => format!("{} hours ago", s / 3600),
                    s if s > 60 => format!("{} mins ago", s / 60),
                    _ => "just now".to_string(),
                }),
            ]);
        }

        println!("\n{}", "Backups to Delete".bold().yellow());
        println!("{}", table);

        if !force
            && !confirm(format!(
                "Do you want to delete {} backup{}?",
                files_to_delete.len(),
                if files_to_delete.len() == 1 { "" } else { "s" }
            ))
            .initial_value(false)
            .interact()?
        {
            println!("\n{}", "Deletion cancelled.".yellow());
            return Ok(());
        }

        println!("\n{}", "Deleting backups...".cyan());
    }

    let (successes, failures): (Vec<_>, Vec<_>) =
        futures::future::join_all(files_to_delete.iter().map(|path| async move {
            let result = tokio::fs::remove_file(path)
                .await
                .map(|_| path)
                .map_err(|e| (path, e));

            if result.is_ok() {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    let wal_path = path.with_file_name(format!("{}.db-wal", stem));
                    let shm_path = path.with_file_name(format!("{}.db-shm", stem));
                    let _ = tokio::fs::remove_file(wal_path).await;
                    let _ = tokio::fs::remove_file(shm_path).await;
                }
            }

            result
        }))
        .await
        .into_iter()
        .partition(Result::is_ok);

    let success_count = successes.len();
    let failed_files: Vec<_> = failures.into_iter().filter_map(Result::err).collect();

    report_deletion_results(success_count, &failed_files);

    if cleanup {
        println!("\n{}", "Cleaning up orphaned auxiliary files...".cyan());
        let backup_dir = Paths::backup_dir();
        let orphaned_count = cleanup_orphaned_files(&backup_dir).await?;

        if orphaned_count > 0 {
            println!(
                "{}",
                format!(
                    "✓ Cleaned up {} orphaned auxiliary file{}",
                    orphaned_count,
                    if orphaned_count == 1 { "" } else { "s" }
                )
                .green()
                .bold()
            );
        } else {
            println!("{}", "No orphaned auxiliary files found".bright_black());
        }
    }

    println!();
    Ok(())
}
