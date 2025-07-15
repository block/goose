mod scenarios;

use crate::session::Session;
use anyhow::Result;
use goose::agents::Agent;
use goose::config::Config;
use goose::model::ModelConfig;
use goose::providers::{create, testprovider::TestProvider};
use std::path::Path;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct ScenarioResult {
    pub messages: Vec<goose::message::Message>,
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

pub async fn run_test_scenario(test_name: &str, inputs: &[&str]) -> Result<ScenarioResult> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let file_path = format!(
        "{}/src/scenario_tests/recordings/{}.json",
        manifest_dir, test_name
    );

    if let Some(parent) = Path::new(&file_path).parent() {
        std::fs::create_dir_all(parent)?;
    }

    let replay_mode = Path::new(&file_path).exists();
    let provider = if replay_mode {
        match TestProvider::new_replaying(&file_path) {
            Ok(test_provider) => {
                Arc::new(test_provider) as Arc<dyn goose::providers::base::Provider>
            }
            Err(e) => {
                let _ = std::fs::remove_file(&file_path);
                return Err(anyhow::anyhow!(
                    "Test replay failed for '{}': {}. File deleted - re-run test to record fresh data.",
                    test_name, e
                ));
            }
        }
    } else {
        let config = Config::global();

        let provider_name: String = config
            .get_param("GOOSE_PROVIDER")
            .expect("No provider configured. Run 'goose configure' first");
        let model_name = config
            .get_param("GOOSE_MODEL")
            .expect("No model configured. Run 'goose configure' first");

        let model_config = ModelConfig::new(model_name);

        let inner_provider = create(&provider_name, model_config)?;
        Arc::new(TestProvider::new_recording(inner_provider, &file_path))
    };

    let agent = Agent::new();
    agent.update_provider(provider).await?;

    let mut session = Session::new(agent, None, false, None, None, None);

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
                "Test replay failed for '{}' - missing recorded interaction: {}. File deleted - re-run test to record fresh data.",
                test_name, err_msg
            ));
        }
    }

    Ok(ScenarioResult { messages, error })
}
