use dotenvy::dotenv;

use crate::scenario_tests::provider_configs::{get_provider_configs, ProviderConfig};
use crate::session::Session;
use anyhow::Result;
use goose::agents::Agent;
use goose::message::Message;
use goose::model::ModelConfig;
use goose::providers::{create, testprovider::TestProvider};
use std::collections::HashMap;
use std::path::Path;
use std::sync::Arc;

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

pub async fn run_scenario<F>(test_name: &str, inputs: &[&str], validator: F) -> Result<()>
where
    F: Fn(&ScenarioResult) -> Result<()> + Send + Sync + 'static,
{
    let inputs_owned: Vec<String> = inputs.iter().map(|s| s.to_string()).collect();

    if let Ok(only_provider) = std::env::var("GOOSE_TEST_PROVIDER") {
        let active_providers = get_provider_configs();
        let config = active_providers
            .iter()
            .find(|c| c.name.to_lowercase() == only_provider.to_lowercase())
            .ok_or_else(|| {
                anyhow::anyhow!(
                    "Provider '{}' not found. Available: {}",
                    only_provider,
                    get_provider_configs()
                        .iter()
                        .map(|c| c.name)
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            })?;

        println!("Running test '{}' for provider: {}", test_name, config.name);
        run_provider_scenario_with_validation(config, test_name, &inputs_owned, &validator).await?;
        return Ok(());
    }

    let mut failures = Vec::new();

    for config in get_provider_configs() {
        match run_provider_scenario_with_validation(config, test_name, &inputs_owned, &validator)
            .await
        {
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

async fn run_provider_scenario_with_validation<F>(
    config: &ProviderConfig,
    test_name: &str,
    inputs: &[String],
    validator: &F,
) -> Result<()>
where
    F: Fn(&ScenarioResult) -> Result<()>,
{
    if let Ok(path) = dotenv() {
        println!("Loaded environment from {:?}", path);
    }

    let factory_name = config.name_for_factory();
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let file_path = format!(
        "{}/src/scenario_tests/recordings/{}/{}.json",
        manifest_dir,
        factory_name.to_lowercase(),
        test_name
    );

    if let Some(parent) = Path::new(&file_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let replay_mode = Path::new(&file_path).exists();
    let (provider_arc, provider_for_saving, original_env) = if replay_mode {
        match TestProvider::new_replaying(&file_path) {
            Ok(test_provider) => (Arc::new(test_provider), None, None),
            Err(e) => {
                let _ = std::fs::remove_file(&file_path);
                return Err(anyhow::anyhow!(
                    "Test replay failed for '{}' ({}): {}. File deleted - re-run test to record fresh data.",
                    test_name, factory_name, e
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

        let original_env = setup_environment(config)?;

        let inner_provider = create(
            &factory_name,
            ModelConfig::new(config.model_name.to_string()),
        )?;

        let test_provider = Arc::new(TestProvider::new_recording(inner_provider, &file_path));
        (
            test_provider.clone(),
            Some(test_provider),
            Some(original_env),
        )
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
                test_name, factory_name, err_msg
            ));
        }
    }

    let result = ScenarioResult { messages, error };

    validator(&result)?;

    drop(session);

    if let Some(provider) = provider_for_saving {
        if result.error.is_none() {
            Arc::try_unwrap(provider)
                .map_err(|_| anyhow::anyhow!("Failed to unwrap provider for recording"))?
                .finish_recording()?;
        }
    }

    if let Some(env) = original_env {
        restore_environment(config, &env);
    }

    Ok(())
}

fn setup_environment(config: &ProviderConfig) -> Result<HashMap<&'static str, String>> {
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
        return Err(anyhow::anyhow!("Missing required environment variables"));
    }

    Ok(original_env)
}

fn restore_environment(config: &ProviderConfig, original_env: &HashMap<&'static str, String>) {
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
}
