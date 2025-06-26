// Agent generator for goose-bench with full functionality
use crate::bench_session::{BenchAgent, BenchBaseSession};
use crate::eval_suites::ExtensionRequirements;
use crate::interaction_limited_agent::InteractionLimitedAgent;
use async_trait::async_trait;
use base64::Engine;
use goose::agents::extension::{Envs, ExtensionConfig};
use goose::agents::Agent;
use goose::config::extensions::ExtensionConfigManager;
use goose::config::Config;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::create;
use mcp_core::Tool;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::Mutex;


fn get_tools_base64(tools: &[Tool]) -> String {
    let tools_json = serde_json::to_string(tools).expect("Failed to serialize tools");
    base64::engine::general_purpose::STANDARD.encode(tools_json)
}

fn get_mock_binary_path() -> String {
    std::env::current_exe()
        .expect("Failed to get current executable path")
        .parent()
        .expect("Failed to get parent directory")
        .join("mock_mcp_server")
        .to_string_lossy()
        .to_string()
}

fn create_default_agent(session_id: String, max_interactions: Option<usize>) -> BenchAgent {
    let agent = Agent::new();
    let interaction_limited_agent = InteractionLimitedAgent::new(agent, max_interactions);
    let session_file = std::env::temp_dir().join(format!("bench_session_{}.json", session_id));
    let wrapper = InteractionLimitedAgentWrapper::new(interaction_limited_agent, session_file);
    BenchAgent::new(Box::new(wrapper))
}

// Wrapper for InteractionLimitedAgent to implement BenchBaseSession
struct InteractionLimitedAgentWrapper {
    agent: InteractionLimitedAgent,
    session_file: PathBuf,
    dummy_messages: Vec<Message>,
}

impl InteractionLimitedAgentWrapper {
    fn new(agent: InteractionLimitedAgent, session_file: PathBuf) -> Self {
        Self {
            agent,
            session_file,
            dummy_messages: Vec::new(),
        }
    }
}

#[async_trait]
impl BenchBaseSession for InteractionLimitedAgentWrapper {
    async fn headless(&mut self, message: String) -> anyhow::Result<()> {
        self.agent.prompt(message).await?;
        Ok(())
    }

    fn session_file(&self) -> PathBuf {
        self.session_file.clone()
    }

    fn message_history(&self) -> Vec<Message> {
        self.agent.message_history()
    }

    async fn override_system_prompt(&self, override_prompt: String) {
        self.agent.override_system_prompt(override_prompt).await;
    }

    fn get_total_token_usage(&self) -> anyhow::Result<Option<i32>> {
        Ok(None)
    }

    async fn cleanup_extensions(&self) -> anyhow::Result<()> {
        self.agent.cleanup_extensions().await
    }

    fn get_agent(&self) -> &goose::agents::Agent {
        self.agent.get_agent()
    }

    fn get_messages_mut(&mut self) -> &mut Vec<Message> {
        &mut self.dummy_messages
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        &mut self.agent
    }
}

pub async fn agent_generator(
    requirements: ExtensionRequirements,
    session_id: String,
    dataset: bool,
    available_extensions: Option<HashMap<String, Vec<Tool>>>,
) -> BenchAgent {
    if dataset {
        dataset_agent(requirements, session_id, available_extensions).await
    } else {
        standard_agent(requirements, session_id, available_extensions).await
    }
}

async fn standard_agent(
    requirements: ExtensionRequirements,
    session_id: String,
    _: Option<HashMap<String, Vec<Tool>>>,
) -> BenchAgent {
    // Create a basic agent
    let agent = Agent::new();
    
    // Configure provider for the agent
    let config = Config::global();
    let provider_name = config
        .get_param::<String>("GOOSE_PROVIDER")
        .unwrap_or_else(|_| "anthropic".to_string());
    let model_name = config
        .get_param::<String>("GOOSE_MODEL")
        .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string());

    let model_config = ModelConfig::new(model_name.clone());
    let provider = match create(&provider_name, model_config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to create provider '{}': {}", provider_name, e);
            return create_default_agent(session_id, None);
        }
    };

    if let Err(e) = agent.update_provider(provider).await {
        eprintln!("❌ Failed to update provider: {}", e);
        return create_default_agent(session_id, None);
    }
    
    // Add extensions for standard agents
    for ext_name in requirements.external {
        if let Ok(Some(extension_config)) = ExtensionConfigManager::get_config_by_name(&ext_name) {
            let _ = agent.add_extension(extension_config).await;
        }
    }
    
    for ext_name in requirements.remote {
        if let Ok(Some(extension_config)) = ExtensionConfigManager::get_config_by_name(&ext_name) {
            let _ = agent.add_extension(extension_config).await;
        }
    }
    
    // Handle builtin extensions
    for ext_name in requirements.builtin {
        if let Ok(Some(extension_config)) = ExtensionConfigManager::get_config_by_name(&ext_name) {
            let _ = agent.add_extension(extension_config).await;
        }
    }

    let interaction_limited_agent = InteractionLimitedAgent::new(agent, None);
    
    let session_file = {
        let session_dir = std::env::current_dir().expect("Failed to get current directory");
        session_dir.join(format!("{}.session", session_id))
    };

    let bench_agent = BenchAgent::new(Box::new(InteractionLimitedAgentWrapper::new(
        interaction_limited_agent,
        session_file,
    )));

    // Register error capture
    let errors = Arc::new(Mutex::new(bench_agent.get_errors().await));
    crate::error_capture::ErrorCaptureLayer::register_error_vector(errors);

    bench_agent
}

async fn dataset_agent(
    _requirements: ExtensionRequirements,
    session_id: String,
    available_extensions: Option<HashMap<String, Vec<Tool>>>,
) -> BenchAgent {

    // DISABLED: Clean config setup interferes with normal credential resolution
    // Instead, we'll rely on the agent's normal extension management
    // let _config_guard = setup_clean_config_for_datasets_with_provider()
    //     .unwrap_or_else(|_| EnvGuard::new("GOOSE_DATASET_FALLBACK", "1"));

    // Create Agent directly
    let agent = Agent::new();

    // For dataset benchmarking, use the global config instance 
    let config = Config::global();
    let provider_name = config
        .get_param::<String>("GOOSE_PROVIDER")
        .unwrap_or_else(|_| "anthropic".to_string());
    let model_name = config
        .get_param::<String>("GOOSE_MODEL")
        .unwrap_or_else(|_| "claude-3-5-sonnet-20241022".to_string());

    let model_config = ModelConfig::new(model_name.clone());
    let provider = match create(&provider_name, model_config) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("❌ Failed to create provider '{}': {}", provider_name, e);
            return create_default_agent(session_id, Some(2));
        }
    };

    if let Err(e) = agent.update_provider(provider).await {
        let error_msg = e.to_string();
        if error_msg.contains("already exists") {
            // Table already exists - this is fine, another agent created it first
            // Continue with this agent using the existing table
        } else {
            // This is a real error
            eprintln!("❌ Failed to update provider: {}", e);
            return create_default_agent(session_id, Some(2));
        }
    }


    // Set up extensions for this specific agent if provided
    if let Some(extensions) = available_extensions {
        for (ext_name, tools) in extensions {
            let tools_base64 = get_tools_base64(&tools);
            let binary_path = get_mock_binary_path();

            // Fallback to cargo run if binary doesn't exist
            let (cmd, args) = if std::path::Path::new(&binary_path).exists() {
                (binary_path, vec![])
            } else {
                (
                    "cargo".to_string(),
                    vec![
                        "run".to_string(),
                        "-p".to_string(),
                        "goose-bench".to_string(),
                        "--bin".to_string(),
                        "mock_mcp_server".to_string(),
                    ],
                )
            };

            let extension_config = ExtensionConfig::Stdio {
                name: ext_name.clone(),
                cmd,
                args,
                envs: Envs::new(HashMap::from([
                    ("EXTENSION_NAME".to_string(), ext_name.clone()),
                    ("EXTENSION_TOOLS".to_string(), tools_base64),
                ])),
                env_keys: vec![],
                timeout: None,
                bundled: None,
                description: None,
            };

            // Add extension directly to agent
            let _ = agent.add_extension(extension_config).await;
        }
    }

    // Create an InteractionLimitedAgent with the configured agent
    let interaction_limited = InteractionLimitedAgent::new(agent, Some(2));

    // Create a dummy session file path for compatibility
    let session_file = std::env::temp_dir().join(format!("bench_session_{}.json", session_id));

    // Wrap in BenchBaseSession trait wrapper
    let wrapper = InteractionLimitedAgentWrapper::new(interaction_limited, session_file);

    // Wrap in BenchAgent for compatibility
    let bench_agent = BenchAgent::new(Box::new(wrapper));

    // Register error capture
    let errors = Arc::new(Mutex::new(bench_agent.get_errors().await));
    crate::error_capture::ErrorCaptureLayer::register_error_vector(errors);

    bench_agent
}