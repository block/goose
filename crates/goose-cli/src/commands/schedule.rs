use anyhow::{bail, Context, Result};
use base64::engine::{general_purpose::STANDARD as BASE64_STANDARD, Engine};
use goose::scheduler::{
    get_default_scheduled_recipes_dir, get_default_scheduler_storage_path, ScheduledJob, Scheduler,
    SchedulerError,
};
use std::path::Path;

// Base64 decoding function - might be needed if recipe_source_arg can be base64
// For now, handle_schedule_add will assume it's a path.
async fn _decode_base64_recipe(source: &str) -> Result<String> {
    let bytes = BASE64_STANDARD
        .decode(source.as_bytes())
        .with_context(|| "Recipe source is not a valid path and not valid Base64.")?;
    String::from_utf8(bytes).with_context(|| "Decoded Base64 recipe source is not valid UTF-8.")
}

pub async fn handle_schedule_add(
    id: String,
    cron: String,
    recipe_source_arg: String, // This is expected to be a file path by the Scheduler
) -> Result<()> {
    println!(
        "[CLI Debug] Scheduling job ID: {}, Cron: {}, Recipe Source Path: {}",
        id, cron, recipe_source_arg
    );

    // The Scheduler's add_scheduled_job will handle copying the recipe from recipe_source_arg
    // to its internal storage and validating the path.
    let job = ScheduledJob {
        id: id.clone(),
        source: recipe_source_arg.clone(), // Pass the original user-provided path
        cron,
        last_run: None,
    };

    let scheduler_storage_path =
        get_default_scheduler_storage_path().context("Failed to get scheduler storage path")?;
    let scheduler = Scheduler::new(scheduler_storage_path)
        .await
        .context("Failed to initialize scheduler")?;

    match scheduler.add_scheduled_job(job).await {
        Ok(_) => {
            // The scheduler has copied the recipe to its internal directory.
            // We can reconstruct the likely path for display if needed, or adjust success message.
            let scheduled_recipes_dir = get_default_scheduled_recipes_dir()
                .unwrap_or_else(|_| Path::new("./.goose_scheduled_recipes").to_path_buf()); // Fallback for display
            let extension = Path::new(&recipe_source_arg)
                .extension()
                .and_then(|ext| ext.to_str())
                .unwrap_or("yaml");
            let final_recipe_path = scheduled_recipes_dir.join(format!("{}.{}", id, extension));

            println!(
                "Scheduled job '{}' added. Recipe expected at {:?}",
                id, final_recipe_path
            );
            Ok(())
        }
        Err(e) => {
            // No local file to clean up by the CLI in this revised flow.
            match e {
                SchedulerError::JobIdExists(job_id) => {
                    bail!("Error: Job with ID '{}' already exists.", job_id);
                }
                SchedulerError::RecipeLoadError(msg) => {
                    bail!(
                        "Error with recipe source: {}. Path: {}",
                        msg,
                        recipe_source_arg
                    );
                }
                _ => Err(anyhow::Error::new(e))
                    .context(format!("Failed to add job '{}' to scheduler", id)),
            }
        }
    }
}

pub async fn handle_schedule_list() -> Result<()> {
    let scheduler_storage_path =
        get_default_scheduler_storage_path().context("Failed to get scheduler storage path")?;
    let scheduler = Scheduler::new(scheduler_storage_path)
        .await
        .context("Failed to initialize scheduler")?;

    let jobs = scheduler.list_scheduled_jobs().await;
    if jobs.is_empty() {
        println!("No scheduled jobs found.");
    } else {
        println!("Scheduled Jobs:");
        for job in jobs {
            println!(
                "- ID: {}\n  Cron: {}\n  Recipe Source (in store): {}\n  Last Run: {}",
                job.id,
                job.cron,
                job.source, // This source is now the path within scheduled_recipes_dir
                job.last_run
                    .map_or_else(|| "Never".to_string(), |dt| dt.to_rfc3339())
            );
        }
    }
    Ok(())
}

pub async fn handle_schedule_remove(id: String) -> Result<()> {
    let scheduler_storage_path =
        get_default_scheduler_storage_path().context("Failed to get scheduler storage path")?;
    let scheduler = Scheduler::new(scheduler_storage_path)
        .await
        .context("Failed to initialize scheduler")?;

    match scheduler.remove_scheduled_job(&id).await {
        Ok(_) => {
            println!("Scheduled job '{}' and its associated recipe removed.", id);
            Ok(())
        }
        Err(e) => match e {
            SchedulerError::JobNotFound(job_id) => {
                bail!("Error: Job with ID '{}' not found.", job_id);
            }
            _ => Err(anyhow::Error::new(e))
                .context(format!("Failed to remove job '{}' from scheduler", id)),
        },
    }
}

pub async fn handle_schedule_sessions(id: String, limit: Option<u32>) -> Result<()> {
    let scheduler_storage_path =
        get_default_scheduler_storage_path().context("Failed to get scheduler storage path")?;
    let scheduler = Scheduler::new(scheduler_storage_path)
        .await
        .context("Failed to initialize scheduler")?;

    match scheduler.sessions(&id, limit.unwrap_or(50) as usize).await {
        Ok(sessions) => {
            if sessions.is_empty() {
                println!("No sessions found for schedule ID '{}'.", id);
            } else {
                println!("Sessions for schedule ID '{}':", id);
                // sessions is now Vec<(String, SessionMetadata)>
                for (session_name, metadata) in sessions {
                    println!(
                        "  - Session ID: {}, Working Dir: {}, Description: \"{}\", Messages: {}, Schedule ID: {:?}",
                        session_name, // Display the session_name as Session ID
                        metadata.working_dir.display(),
                        metadata.description,
                        metadata.message_count,
                        metadata.schedule_id.as_deref().unwrap_or("N/A")
                    );
                }
            }
        }
        Err(e) => {
            bail!("Failed to get sessions for schedule '{}': {:?}", id, e);
        }
    }
    Ok(())
}

pub async fn handle_schedule_run_now(id: String) -> Result<()> {
    let scheduler_storage_path =
        get_default_scheduler_storage_path().context("Failed to get scheduler storage path")?;
    let scheduler = Scheduler::new(scheduler_storage_path)
        .await
        .context("Failed to initialize scheduler")?;

    match scheduler.run_now(&id).await {
        Ok(session_id) => {
            println!(
                "Successfully triggered schedule '{}'. New session ID: {}",
                id, session_id
            );
        }
        Err(e) => match e {
            SchedulerError::JobNotFound(job_id) => {
                bail!("Error: Job with ID '{}' not found.", job_id);
            }
            _ => bail!("Failed to run schedule '{}' now: {:?}", id, e),
        },
    }
    Ok(())
}
