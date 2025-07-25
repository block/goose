mod scenarios;
use dotenvy::dotenv;

use crate::session::Session;
use anyhow::Result;
use goose::agents::Agent;
use goose::config::Config;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::{create, testprovider::TestProvider};
use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
struct ProviderConfig {
    name: &'static str,
    factory_name: &'static str,
    required_env_vars: &'static [&'static str],
    env_modifications: Option<HashMap<&'static str, Option<String>>>,
}

static PROVIDER_CONFIGS: &[ProviderConfig] = &[
    ProviderConfig {
        name: "OpenAI",
        factory_name: "openai",
        required_env_vars: &["OPENAI_API_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Anthropic",
        factory_name: "anthropic",
        required_env_vars: &["ANTHROPIC_API_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Azure",
        factory_name: "azure_openai",
        required_env_vars: &[
            "AZURE_OPENAI_API_KEY",
            "AZURE_OPENAI_ENDPOINT",
            "AZURE_OPENAI_DEPLOYMENT_NAME",
        ],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Bedrock",
        factory_name: "aws_bedrock",
        required_env_vars: &["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Google",
        factory_name: "google",
        required_env_vars: &["GOOGLE_API_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Groq",
        factory_name: "groq",
        required_env_vars: &["GROQ_API_KEY"],
        env_modifications: None,
    },
];

#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub messages: Vec<Message>,
    pub error: Option<String>,
}

impl ScenarioResult {
    pub fn message_contents(&self) -> Vec<String> {
        self.messages
            .iter()
            .flat_map(|msg| &msg.content)
            .map(|content| content.as_text().unwrap_or("").to_string())
            .collect()
    }
}

async fn run_all_providers_scenario<F, Fut>(test_name: &str, test_fn: F) -> Result<()>
where
    F: Fn(String, String) -> Fut,
    Fut: Future<Output = Result<ScenarioResult>>,
{
    if let Ok(only_provider) = std::env::var("GOOSE_TEST_PROVIDER") {
        let config = PROVIDER_CONFIGS
            .iter()
            .find(|c| c.name.to_lowercase() == only_provider.to_lowercase())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Provider '{}' not found. Available: {}",
                    only_provider,
                    PROVIDER_CONFIGS
                        .iter()
                        .map(|c| c.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        println!("Running test '{}' for provider: {}", test_name, config.name);
        return run_provider_scenario(config, &test_fn, test_name)
            .await
            .map(|_| ());
    }

    let mut failures = Vec::new();

    for config in PROVIDER_CONFIGS {
        match run_provider_scenario(config, &test_fn, test_name).await {
            Ok(_) => println!("✅ {} - {}", test_name, config.name),
            Err(e) => {
                println!("❌ {} - {} FAILED: {}", test_name, config.name, e);
                failures.push((config.name, e));
            }
        }
    }

    if !failures.is_empty() {
        println!("\n=== Test Failures for {} ===", test_name);
        for (provider, error) in &failures {
            println!("❌ {}: {}", provider, error);
        }
        return Err(anyhow::anyhow!(
            "Test '{}' failed for {} provider(s)",
            test_name,
            failures.len()
        ));
    }

    Ok(())
}

async fn run_provider_scenario<F, Fut>(
    config: &ProviderConfig,
    test_fn: &F,
    test_name: &str,
) -> Result<ScenarioResult>
where
    F: Fn(String, String) -> Fut,
    Fut: Future<Output = Result<ScenarioResult>>,
{
    if let Ok(path) = dotenvy::dotenv() {
        println!("Loaded environment from {:?}", path);
    }

    let mut original_env = HashMap::new();

    for &var in config.required_env_vars {
        if let Ok(val) = std::env::var(var) {
            original_env.insert(var, val);
        }
    }

    if let Some(mods) = &config.env_modifications {
        for &var in mods.keys() {
            if let Ok(val) = std::env::var(var) {
                original_env.insert(var, val);
            }
        }
    }

    if let Some(mods) = &config.env_modifications {
        for (&var, value) in mods.iter() {
            match value {
                Some(val) => std::env::set_var(var, val),
                None => std::env::remove_var(var),
            }
        }
    }

    let missing_vars = config
        .required_env_vars
        .iter()
        .any(|var| std::env::var(var).is_err());

    if missing_vars {
        println!(
            "Skipping {} scenario - credentials not configured",
            config.name
        );
        return Ok(ScenarioResult {
            messages: vec![],
            error: Some("Skipped - credentials not configured".to_string()),
        });
    }

    std::env::set_var("GOOSE_PROVIDER", config.factory_name);

    let result = test_fn(test_name.to_string(), config.name.to_string()).await;

    for (&var, value) in original_env.iter() {
        std::env::set_var(var, value);
    }
    if let Some(mods) = &config.env_modifications {
        for &var in mods.keys() {
            if !original_env.contains_key(var) {
                std::env::remove_var(var);
            }
        }
    }

    result
}

pub async fn run_test_scenario(test_name: &str, inputs: &[&str]) -> Result<ScenarioResult> {
    let (result, provider_for_saving) =
        run_single_provider_scenario(test_name, "default", inputs).await?;

    if let Some(provider) = provider_for_saving {
        if result.error.is_none() {
            if result.error.is_none() {
                Arc::try_unwrap(provider)
                    .map_err(|_| anyhow::anyhow!("Failed to unwrap provider for recording"))?
                    .finish_recording()?;
            }
        }
    }

    Ok(result)
}

async fn run_single_provider_scenario(
    test_name: &str,
    provider_name: &str,
    inputs: &[&str],
) -> Result<(ScenarioResult, Option<Arc<TestProvider>>)> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let file_path = format!(
        "{}/src/scenario_tests/recordings/{}/{}.json",
        manifest_dir,
        provider_name.to_lowercase(),
        test_name
    );

    if let Some(parent) = Path::new(&file_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let replay_mode = Path::new(&file_path).exists();
    let (provider_arc, provider_for_saving) = if replay_mode {
        match TestProvider::new_replaying(&file_path) {
            Ok(test_provider) => (Arc::new(test_provider), None),
            Err(e) => {
                let _ = std::fs::remove_file(&file_path);
                return Err(anyhow::anyhow!(
                    "Test replay failed for '{}' ({}): {}. File deleted - re-run test to record fresh data.",
                    test_name, provider_name, e
                ));
            }
        }
    } else {
        if std::env::var("GITHUB_ACTIONS").is_ok() {
            panic!(
                "Test recording is not supported on CI. \
            Did you forget to add the file {} to the repository and were expecting that to replay?",
                file_path
            );
        }
        let config = Config::global();

        let (provider_name, model_name): (String, String) = match (
            config.get_param::<String>("GOOSE_PROVIDER"),
            config.get_param::<String>("GOOSE_MODEL"),
        ) {
            (Ok(provider), Ok(model)) => (provider, model),
            _ => {
                panic!("Provider or model not configured. Run 'goose configure' first");
            }
        };

        let model_config = ModelConfig::new(model_name);
        let inner_provider = create(&provider_name, model_config)?;
        let test_provider = Arc::new(TestProvider::new_recording(inner_provider, &file_path));
        (test_provider.clone(), Some(test_provider))
    };

    let agent = Agent::new();
    agent
        .update_provider(provider_arc as Arc<dyn goose::providers::base::Provider>)
        .await?;

    let mut session = Session::new(agent, None, false, None, None, None, None);

    let mut error = None;
    for input in inputs {
        if let Err(e) = session.headless(input.to_string()).await {
            error = Some(e.to_string());
            break;
        }
    }

    let messages = session.message_history().to_vec();

    if let Some(ref err_msg) = error {
        if err_msg.contains("No recorded response found") {
            let _ = std::fs::remove_file(&file_path);
            return Err(anyhow::anyhow!(
                "Test replay failed for '{}' ({}) - missing recorded interaction: {}. File deleted - re-run test to record fresh data.",
                test_name, provider_name, err_msg
            ));
        }
    }

    let result = ScenarioResult { messages, error };
    Ok((result, provider_for_saving))
}

pub async fn run_multi_provider_scenario<F>(
    test_name: &str,
    inputs: &[&str],
    validator: F,
) -> Result<()>
where
    F: Fn(&ScenarioResult) -> Result<()> + Send + Sync + 'static,
{
    let inputs_owned: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();

    run_all_providers_scenario(test_name, |name, provider| {
        let inputs = inputs_owned.clone();
        let validator = &validator;
        async move {
            let (result, provider_for_saving) = run_single_provider_scenario(
                &name,
                &provider,
                &inputs.iter().map(|s| s.as_str()).collect::<Vec<_>>(),
            )
            .await?;

            validator(&result)?;

            if let Some(provider) = provider_for_saving {
                if result.error.is_none() {
                    if result.error.is_none() {
                        Arc::try_unwrap(provider)
                            .map_err(|_| {
                                anyhow::anyhow!("Failed to unwrap provider for recording")
                            })?
                            .finish_recording()?;
                    }
                }
            }

            Ok(result)
        }
    })
    .await
}
