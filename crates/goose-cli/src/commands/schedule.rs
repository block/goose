use anyhow::{bail, Context, Result};
use base64::engine::{general_purpose::STANDARD as BASE64_STANDARD, Engine};
use goose::scheduler::{
    get_default_scheduled_recipes_dir, get_default_scheduler_storage_path, ScheduledJob, Scheduler,
    SchedulerError,
};
use std::fs;
use std::path::Path;

async fn read_recipe_content(source: &str) -> Result<String> {
    if Path::new(source).exists() {
        fs::read_to_string(source)
            .with_context(|| format!("Failed to read recipe file from path: {}", source))
    } else {
        let bytes = BASE64_STANDARD
            .decode(source.as_bytes())
            .with_context(|| "Recipe source is not a valid path and not valid Base64.")?;
        String::from_utf8(bytes).with_context(|| "Decoded Base64 recipe source is not valid UTF-8.")
    }
}

pub async fn handle_schedule_add(
    id: String,
    cron: String,
    recipe_source_arg: String,
) -> Result<()> {
    let scheduled_recipes_dir =
        get_default_scheduled_recipes_dir().context("Failed to get scheduled recipes directory")?;
    fs::create_dir_all(&scheduled_recipes_dir).with_context(|| {
        format!(
            "Failed to create scheduled recipes directory at {:?}",
            scheduled_recipes_dir
        )
    })?;

    let recipe_content = read_recipe_content(&recipe_source_arg)
        .await
        .context("Failed to read or decode recipe source")?;

    let extension = Path::new(&recipe_source_arg)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("yaml");

    let recipe_filename_in_store = format!("{}.{}", id, extension);
    let recipe_path_in_store = scheduled_recipes_dir.join(&recipe_filename_in_store);

    fs::write(&recipe_path_in_store, &recipe_content).with_context(|| {
        format!(
            "Failed to write recipe to central store at {:?}",
            recipe_path_in_store
        )
    })?;

    let job = ScheduledJob {
        id: id.clone(),
        source: recipe_path_in_store.to_string_lossy().to_string(),
        cron,
        last_run: None,
    };

    let scheduler_storage_path =
        get_default_scheduler_storage_path().context("Failed to get scheduler storage path")?;
    let scheduler = Scheduler::new(scheduler_storage_path)
        .await
        .context("Failed to initialize scheduler")?;

    match scheduler.add_scheduled_job(job.clone()).await {
        Ok(_) => {
            println!(
                "Scheduled job '{}' added. Recipe stored at {:?}",
                id, recipe_path_in_store
            );
            Ok(())
        }
        Err(e) => {
            let _ = fs::remove_file(&recipe_path_in_store);
            match e {
                SchedulerError::JobIdExists(job_id) => {
                    bail!("Error: Job with ID '{}' already exists.", job_id);
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
                "- ID: {}\n  Cron: {}\n  Recipe Source: {}\n  Last Run: {}",
                job.id,
                job.cron,
                job.source,
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
