use anyhow::{anyhow, Result};
use clap::Parser;
use goose::agents::{Agent, SessionConfig};
use goose::config::Config;
use goose::message::Message;
use goose::providers::create;
use goose::recipe::Recipe;
use goose::session;
use std::env;
use std::fs;
use std::path::Path;
use tracing::info;

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

async fn execute_recipe(job_id: &str, recipe_path: &str) -> Result<String> {
    let recipe_path_buf = Path::new(recipe_path);

    // Check if recipe file exists
    if !recipe_path_buf.exists() {
        return Err(anyhow!("Recipe file not found: {}", recipe_path));
    }

    // Read and parse recipe
    let recipe_content = fs::read_to_string(recipe_path_buf)?;
    let recipe: Recipe = {
        let extension = recipe_path_buf
            .extension()
            .and_then(|os_str| os_str.to_str())
            .unwrap_or("yaml")
            .to_lowercase();

        match extension.as_str() {
            "json" | "jsonl" => serde_json::from_str::<Recipe>(&recipe_content)
                .map_err(|e| anyhow!("Failed to parse JSON recipe '{}': {}", recipe_path, e))?,
            "yaml" | "yml" => serde_yaml::from_str::<Recipe>(&recipe_content)
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
    let Some(prompt_text) = recipe.prompt else {
        info!(
            "Recipe '{}' has no prompt to execute for job '{}'",
            recipe_path, job_id
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

    // Create session configuration
    let current_dir =
        env::current_dir().map_err(|e| anyhow!("Failed to get current directory: {}", e))?;

    let session_config = SessionConfig {
        id: goose::session::storage::Identifier::Name(session_id.clone()),
        working_dir: current_dir.clone(),
        schedule_id: Some(job_id.to_string()),
    };

    // Execute the recipe
    let mut messages = vec![Message::user().with_text(prompt_text)];

    info!("Executing recipe for job '{}' with prompt", job_id);

    let mut stream = agent
        .reply(&messages, Some(session_config))
        .await
        .map_err(|e| anyhow!("Agent failed to reply for recipe '{}': {}", recipe_path, e))?;

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
                working_dir: current_dir,
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
        "Finished executing job '{}', session: {}",
        job_id, session_id
    );
    Ok(session_id)
}
