// tests/providers_extract.rs

use anyhow::Result;
use dotenv::dotenv;
use goose_llm::message::Message;
use goose_llm::providers::base::Provider;
use goose_llm::providers::{databricks::DatabricksProvider, openai::OpenAiProvider};
use goose_llm::ModelConfig;
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(Debug, PartialEq, Copy, Clone)]
enum ProviderType {
    OpenAi,
    Databricks,
}

impl ProviderType {
    fn required_env(&self) -> &'static [&'static str] {
        match self {
            ProviderType::OpenAi => &["OPENAI_API_KEY"],
            ProviderType::Databricks => &["DATABRICKS_HOST", "DATABRICKS_TOKEN"],
        }
    }

    fn create_provider(&self, cfg: ModelConfig) -> Result<Arc<dyn Provider>> {
        Ok(match self {
            ProviderType::OpenAi => Arc::new(OpenAiProvider::from_env(cfg)),
            ProviderType::Databricks => Arc::new(DatabricksProvider::from_env(cfg)),
        })
    }
}

fn check_required_env_vars(required: &[&str]) -> bool {
    let missing: Vec<_> = required
        .iter()
        .filter(|&&v| std::env::var(v).is_err())
        .cloned()
        .collect();
    if !missing.is_empty() {
        println!("Skipping test; missing env vars: {:?}", missing);
        false
    } else {
        true
    }
}

// --- Shared inputs for "paper" task ---
const PAPER_SYSTEM: &str =
    "You are an expert at structured data extraction. Extract the metadata of a research paper into JSON.";
const PAPER_TEXT: &str =
    "Application of Quantum Algorithms in Interstellar Navigation: A New Frontier \
     by Dr. Stella Voyager, Dr. Nova Star, Dr. Lyra Hunter. Abstract: This paper \
     investigates the utilization of quantum algorithms to improve interstellar \
     navigation systems. Keywords: Quantum algorithms, interstellar navigation, \
     space-time anomalies, quantum superposition, quantum entanglement, space travel.";

fn paper_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "title":    { "type": "string" },
            "authors":  { "type": "array",  "items": { "type": "string" } },
            "abstract": { "type": "string" },
            "keywords": { "type": "array",  "items": { "type": "string" } }
        },
        "required": ["title","authors","abstract","keywords"],
        "additionalProperties": false
    })
}

// --- Shared inputs for "UI" task ---
const UI_SYSTEM: &str = "You are a UI generator AI. Convert the user input into a JSON-driven UI.";
const UI_TEXT: &str = "Make a User Profile Form";

fn ui_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "type": {
                "type": "string",
                "enum": ["div","button","header","section","field","form"]
            },
            "label":   { "type": "string" },
            "children": {
                "type": "array",
                "items": { "$ref": "#" }
            },
            "attributes": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "name":  { "type": "string" },
                        "value": { "type": "string" }
                    },
                    "required": ["name","value"],
                    "additionalProperties": false
                }
            }
        },
        "required": ["type","label","children","attributes"],
        "additionalProperties": false
    })
}

/// Generic runner for any extract task
async fn run_extract_test<F>(
    provider_type: ProviderType,
    model: &str,
    system: &'static str,
    user_text: &'static str,
    schema: Value,
    validate: F,
) -> Result<()>
where
    F: Fn(&Value) -> bool,
{
    dotenv().ok();
    if !check_required_env_vars(provider_type.required_env()) {
        return Ok(());
    }

    let cfg = ModelConfig::new(model.to_string()).with_temperature(Some(0.0));
    let provider = provider_type.create_provider(cfg)?;

    let msg = Message::user().with_text(user_text);
    let resp = provider.extract(system, &[msg], &schema, None).await?;

    println!("[{:?}] extract => {}", provider_type, resp.data);

    assert!(
        validate(&resp.data),
        "{:?} failed validation on {}",
        provider_type,
        resp.data
    );
    Ok(())
}

/// Helper for the "paper" task
async fn run_extract_paper_test(provider: ProviderType, model: &str) -> Result<()> {
    run_extract_test(
        provider,
        model,
        PAPER_SYSTEM,
        PAPER_TEXT,
        paper_schema(),
        |v| {
            v.as_object()
                .map(|o| {
                    ["title", "authors", "abstract", "keywords"]
                        .iter()
                        .all(|k| o.contains_key(*k))
                })
                .unwrap_or(false)
        },
    )
    .await
}

/// Helper for the "UI" task
async fn run_extract_ui_test(provider: ProviderType, model: &str) -> Result<()> {
    run_extract_test(provider, model, UI_SYSTEM, UI_TEXT, ui_schema(), |v| {
        v.as_object()
            .and_then(|o| o.get("type").and_then(Value::as_str))
            == Some("form")
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn openai_extract_paper() -> Result<()> {
        run_extract_paper_test(ProviderType::OpenAi, "gpt-4o").await
    }

    #[tokio::test]
    async fn openai_extract_ui() -> Result<()> {
        run_extract_ui_test(ProviderType::OpenAi, "gpt-4o").await
    }

    #[tokio::test]
    async fn databricks_extract_paper() -> Result<()> {
        run_extract_paper_test(ProviderType::Databricks, "goose-gpt-4-1").await
    }

    #[tokio::test]
    async fn databricks_extract_ui() -> Result<()> {
        run_extract_ui_test(ProviderType::Databricks, "goose-gpt-4-1").await
    }
}
