use anyhow::{anyhow, Result};
use clap::Parser;
use goose::agents::{Agent, SessionConfig};
use goose::config::Config;
use goose::message::Message;
use goose::providers::create;
use goose::recipe::Recipe;
use goose::session;
use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};
use tracing::{info, warn};

#[derive(Serialize, Deserialize, Debug)]
struct ScheduleConfig {
    /// Whether this schedule should run in foreground (desktop window) when possible
    pub foreground: bool,
    /// Fallback to background execution if foreground is not available
    pub fallback_to_background: bool,
    /// Custom window title for foreground execution
    pub window_title: Option<String>,
    /// Working directory for the session
    pub working_directory: Option<String>,
}

impl Default for ScheduleConfig {
    fn default() -> Self {
        Self {
            foreground: false,
            fallback_to_background: true,
            window_title: None,
            working_directory: None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct EnhancedRecipe {
    #[serde(flatten)]
    pub recipe: Recipe,
    /// Schedule-specific configuration
    pub schedule: Option<ScheduleConfig>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Job ID for the scheduled job
    job_id: String,

    /// Path to the recipe file to execute
    recipe_path: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let args = Args::parse();

    info!("Starting goose-scheduler-executor for job: {}", args.job_id);
    info!("Recipe path: {}", args.recipe_path);

    // Execute the recipe and get session ID
    let session_id = execute_recipe(&args.job_id, &args.recipe_path).await?;

    // Output session ID to stdout (this is what the Go service expects)
    println!("{}", session_id);

    Ok(())
}

/// Check if the Goose desktop app is currently running
fn is_desktop_app_running() -> bool {
    info!("Checking if desktop app is running...");
    
    // Try to detect if the Goose desktop app is running by checking for processes
    #[cfg(target_os = "macos")]
    {
        info!("Running macOS process detection: pgrep -f 'Goose.app'");
        let output = Command::new("pgrep")
            .args(["-f", "Goose.app"])
            .output();
        
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                info!("pgrep stdout: '{}'", stdout.trim());
                if !stderr.is_empty() {
                    info!("pgrep stderr: '{}'", stderr.trim());
                }
                let is_running = !output.stdout.is_empty();
                info!("Desktop app running: {}", is_running);
                is_running
            }
            Err(e) => {
                warn!("Failed to run pgrep: {}", e);
                false
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        info!("Running Windows process detection: tasklist /FI 'IMAGENAME eq Goose.exe'");
        let output = Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq Goose.exe"])
            .output();
            
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                info!("tasklist stdout: '{}'", stdout.trim());
                if !stderr.is_empty() {
                    info!("tasklist stderr: '{}'", stderr.trim());
                }
                let is_running = stdout.contains("Goose.exe");
                info!("Desktop app running: {}", is_running);
                is_running
            }
            Err(e) => {
                warn!("Failed to run tasklist: {}", e);
                false
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        info!("Running Linux process detection: pgrep -f 'goose'");
        let output = Command::new("pgrep")
            .args(["-f", "goose"])
            .output();
            
        match output {
            Ok(output) => {
                let stdout = String::from_utf8_lossy(&output.stdout);
                let stderr = String::from_utf8_lossy(&output.stderr);
                info!("pgrep stdout: '{}'", stdout.trim());
                if !stderr.is_empty() {
                    info!("pgrep stderr: '{}'", stderr.trim());
                }
                let is_running = !output.stdout.is_empty();
                info!("Desktop app running: {}", is_running);
                is_running
            }
            Err(e) => {
                warn!("Failed to run pgrep: {}", e);
                false
            }
        }
    }
}

/// Generate a deep link for the recipe that can be opened by the desktop app
fn generate_deep_link(recipe: &Recipe, job_id: &str) -> Result<String> {
    use base64::Engine;
    
    info!("Generating deep link for job: {}", job_id);
    info!("Recipe title: {}", recipe.title);
    
    // Create the recipe config for the deep link
    let recipe_config = serde_json::json!({
        "id": job_id, // Use job_id as the recipe id
        "title": recipe.title,
        "description": recipe.description,
        "instructions": recipe.instructions,
        "activities": [], // Recipe doesn't have activities field
        "prompt": recipe.prompt
    });
    
    info!("Recipe config JSON: {}", serde_json::to_string_pretty(&recipe_config)?);
    
    // Encode the config as base64
    let config_json = serde_json::to_string(&recipe_config)?;
    let config_base64 = base64::engine::general_purpose::STANDARD.encode(config_json);
    
    // Create the deep link URL
    let deep_link = format!("goose://recipe?config={}&scheduledJob={}", config_base64, job_id);
    
    info!("Generated deep link (length: {}): {}", deep_link.len(), deep_link);
    
    Ok(deep_link)
}

/// Open a deep link using the system's default protocol handler
fn open_deep_link(deep_link: &str) -> Result<()> {
    info!("Attempting to open deep link with system handler");
    info!("Deep link: {}", deep_link);
    
    #[cfg(target_os = "macos")]
    {
        info!("Using macOS 'open' command");
        let mut cmd = Command::new("open");
        cmd.arg(deep_link);
        info!("Running command: open '{}'", deep_link);
        
        match cmd.spawn() {
            Ok(mut child) => {
                info!("Successfully spawned 'open' command");
                // Don't wait for the process to finish, let it run in background
                match child.try_wait() {
                    Ok(Some(status)) => info!("Command completed immediately with status: {}", status),
                    Ok(None) => info!("Command is running in background"),
                    Err(e) => warn!("Error checking command status: {}", e),
                }
            }
            Err(e) => {
                warn!("Failed to spawn 'open' command: {}", e);
                return Err(anyhow::anyhow!("Failed to open deep link on macOS: {}", e));
            }
        }
    }
    
    #[cfg(target_os = "windows")]
    {
        info!("Using Windows 'cmd /c start' command");
        let mut cmd = Command::new("cmd");
        cmd.args(["/c", "start", "", deep_link]);
        info!("Running command: cmd /c start \"\" '{}'", deep_link);
        
        match cmd.spawn() {
            Ok(mut child) => {
                info!("Successfully spawned Windows start command");
                match child.try_wait() {
                    Ok(Some(status)) => info!("Command completed immediately with status: {}", status),
                    Ok(None) => info!("Command is running in background"),
                    Err(e) => warn!("Error checking command status: {}", e),
                }
            }
            Err(e) => {
                warn!("Failed to spawn Windows start command: {}", e);
                return Err(anyhow::anyhow!("Failed to open deep link on Windows: {}", e));
            }
        }
    }
    
    #[cfg(target_os = "linux")]
    {
        info!("Using Linux 'xdg-open' command");
        let mut cmd = Command::new("xdg-open");
        cmd.arg(deep_link);
        info!("Running command: xdg-open '{}'", deep_link);
        
        match cmd.spawn() {
            Ok(mut child) => {
                info!("Successfully spawned 'xdg-open' command");
                match child.try_wait() {
                    Ok(Some(status)) => info!("Command completed immediately with status: {}", status),
                    Ok(None) => info!("Command is running in background"),
                    Err(e) => warn!("Error checking command status: {}", e),
                }
            }
            Err(e) => {
                warn!("Failed to spawn 'xdg-open' command: {}", e);
                return Err(anyhow::anyhow!("Failed to open deep link on Linux: {}", e));
            }
        }
    }
    
    info!("Deep link opening command initiated successfully");
    Ok(())
}

/// Execute the recipe in foreground mode (desktop app window)
async fn execute_foreground(recipe: &Recipe, job_id: &str) -> Result<String> {
    info!("Executing recipe in foreground mode: {}", job_id);
    
    // Generate deep link
    let deep_link = generate_deep_link(recipe, job_id)?;
    
    // Open the deep link
    open_deep_link(&deep_link)?;
    
    // Create a session ID for tracking
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();
    let session_id = format!("scheduled-fg-{}-{}", job_id, timestamp);
    
    info!("Foreground execution initiated for job: {} with session: {}", job_id, session_id);
    Ok(session_id)
}

async fn execute_recipe(job_id: &str, recipe_path: &str) -> Result<String> {
    let recipe_path_buf = Path::new(recipe_path);

    // Check if recipe file exists
    if !recipe_path_buf.exists() {
        return Err(anyhow!("Recipe file not found: {}", recipe_path));
    }

    // Read and parse recipe with schedule configuration
    let recipe_content = fs::read_to_string(recipe_path_buf)?;
    let enhanced_recipe: EnhancedRecipe = {
        let extension = recipe_path_buf
            .extension()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("yaml")
            .to_lowercase();

        match extension.as_str() {
            "json" | "jsonl" => serde_json::from_str::<EnhancedRecipe>(&recipe_content)
                .map_err(|e| anyhow!("Failed to parse JSON recipe '{}': {}", recipe_path, e))?,
            "yaml" | "yml" => serde_yaml::from_str::<EnhancedRecipe>(&recipe_content)
                .map_err(|e| anyhow!("Failed to parse YAML recipe '{}': {}", recipe_path, e))?,
            _ => {
                return Err(anyhow!(
                    "Unsupported recipe file extension '{}' for: {}",
                    extension,
                    recipe_path
                ));
            }
        }
    };

    let recipe = &enhanced_recipe.recipe;
    
    info!("Loaded recipe successfully:");
    info!("  Title: {}", recipe.title);
    info!("  Description: {}", recipe.description);
    info!("  Has prompt: {}", recipe.prompt.is_some());
    info!("  Has instructions: {}", recipe.instructions.as_ref().map_or(false, |s| !s.is_empty()));
    
    // Determine execution mode based on schedule configuration
    let default_config = ScheduleConfig::default();
    let schedule_config = enhanced_recipe.schedule.as_ref().unwrap_or(&default_config);
    
    info!("Schedule configuration:");
    info!("  Foreground: {}", schedule_config.foreground);
    info!("  Fallback to background: {}", schedule_config.fallback_to_background);
    info!("  Window title: {:?}", schedule_config.window_title);
    info!("  Working directory: {:?}", schedule_config.working_directory);
    
    if schedule_config.foreground {
        info!("Recipe configured for foreground execution");
        
        // Check if desktop app is running
        if is_desktop_app_running() {
            info!("Desktop app detected, executing in foreground");
            match execute_foreground(recipe, job_id).await {
                Ok(session_id) => return Ok(session_id),
                Err(e) => {
                    warn!("Foreground execution failed: {}", e);
                    if !schedule_config.fallback_to_background {
                        return Err(e);
                    }
                    warn!("Falling back to background execution");
                }
            }
        } else {
            info!("Desktop app not detected");
            if !schedule_config.fallback_to_background {
                return Err(anyhow!(
                    "Recipe requires foreground execution but desktop app is not running"
                ));
            }
            info!("Falling back to background execution");
        }
    } else {
        info!("Recipe configured for background execution");
    }
    
    // Continue with background execution (original logic)
    execute_recipe_background(job_id, recipe, schedule_config).await
}

async fn execute_recipe_background(job_id: &str, recipe: &Recipe, schedule_config: &ScheduleConfig) -> Result<String> {
    info!("Executing recipe in background mode: {}", job_id);

    // Create agent
    let agent = Agent::new();

    // Get provider configuration
    let global_config = Config::global();
    let provider_name: String = global_config.get_param("GOOSE_PROVIDER").map_err(|_| {
        anyhow!("GOOSE_PROVIDER not configured. Run 'goose configure' or set env var.")
    })?;
    let model_name: String = global_config.get_param("GOOSE_MODEL").map_err(|_| {
        anyhow!("GOOSE_MODEL not configured. Run 'goose configure' or set env var.")
    })?;

    let model_config = goose::model::ModelConfig::new(model_name);
    let provider = create(&provider_name, model_config)
        .map_err(|e| anyhow!("Failed to create provider '{}': {}", provider_name, e))?;

    // Set provider on agent
    agent
        .update_provider(provider)
        .await
        .map_err(|e| anyhow!("Failed to set provider on agent: {}", e))?;

    info!(
        "Agent configured with provider '{}' for job '{}'",
        provider_name, job_id
    );

    // Generate session ID
    let session_id = session::generate_session_id();

    // Check if recipe has a prompt
    let Some(prompt_text) = &recipe.prompt else {
        info!(
            "Recipe has no prompt to execute for job '{}'",
            job_id
        );

        // Create empty session for consistency
        let session_file_path = goose::session::storage::get_path(
            goose::session::storage::Identifier::Name(session_id.clone()),
        );

        let metadata = goose::session::storage::SessionMetadata {
            working_dir: env::current_dir().unwrap_or_default(),
            description: "Empty job - no prompt".to_string(),
            schedule_id: Some(job_id.to_string()),
            message_count: 0,
            ..Default::default()
        };

        goose::session::storage::save_messages_with_metadata(&session_file_path, &metadata, &[])
            .map_err(|e| anyhow!("Failed to persist metadata for empty job: {}", e))?;

        return Ok(session_id);
    };

    // Determine working directory
    let working_dir = if let Some(custom_dir) = &schedule_config.working_directory {
        Path::new(custom_dir).to_path_buf()
    } else {
        env::current_dir().map_err(|e| anyhow!("Failed to get current directory: {}", e))?
    };

    // Create session configuration
    let session_config = SessionConfig {
        id: goose::session::storage::Identifier::Name(session_id.clone()),
        working_dir: working_dir.clone(),
        schedule_id: Some(job_id.to_string()),
    };

    // Execute the recipe
    let mut messages = vec![Message::user().with_text(prompt_text.clone())];

    info!("Executing recipe for job '{}' with prompt", job_id);

    let mut stream = agent
        .reply(&messages, Some(session_config))
        .await
        .map_err(|e| anyhow!("Agent failed to reply for recipe: {}", e))?;

    // Process the response stream
    use futures::StreamExt;
    use goose::agents::AgentEvent;

    while let Some(message_result) = stream.next().await {
        match message_result {
            Ok(AgentEvent::Message(msg)) => {
                if msg.role == mcp_core::role::Role::Assistant {
                    info!("[Job {}] Assistant response received", job_id);
                }
                messages.push(msg);
            }
            Ok(AgentEvent::McpNotification(_)) => {
                // Handle notifications if needed
            }
            Err(e) => {
                return Err(anyhow!("Error receiving message from agent: {}", e));
            }
        }
    }

    // Save session
    let session_file_path = goose::session::storage::get_path(
        goose::session::storage::Identifier::Name(session_id.clone()),
    );

    // Try to read updated metadata, or create fallback
    match goose::session::storage::read_metadata(&session_file_path) {
        Ok(mut updated_metadata) => {
            updated_metadata.message_count = messages.len();
            goose::session::storage::save_messages_with_metadata(
                &session_file_path,
                &updated_metadata,
                &messages,
            )
            .map_err(|e| anyhow!("Failed to persist final messages: {}", e))?;
        }
        Err(_) => {
            let fallback_metadata = goose::session::storage::SessionMetadata {
                working_dir,
                description: format!("Scheduled job: {}", job_id),
                schedule_id: Some(job_id.to_string()),
                message_count: messages.len(),
                ..Default::default()
            };
            goose::session::storage::save_messages_with_metadata(
                &session_file_path,
                &fallback_metadata,
                &messages,
            )
            .map_err(|e| anyhow!("Failed to persist messages with fallback metadata: {}", e))?;
        }
    }

    info!(
        "Finished executing background job '{}', session: {}",
        job_id, session_id
    );
    Ok(session_id)
}
