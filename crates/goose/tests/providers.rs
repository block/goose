use anyhow::Result;
use dotenv::dotenv;
use goose::message::{Message, MessageContent};
use goose::providers::base::Provider;
use goose::providers::{anthropic, databricks, google, groq, ollama, openai, openrouter};
use mcp_core::content::Content;
use mcp_core::tool::Tool;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::Mutex;

#[derive(Debug, Clone, Copy)]
enum TestStatus {
    Passed,
    Skipped,
    Failed,
}

impl std::fmt::Display for TestStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TestStatus::Passed => write!(f, "✅"),
            TestStatus::Skipped => write!(f, "⏭️"),
            TestStatus::Failed => write!(f, "❌"),
        }
    }
}

struct TestReport {
    results: Mutex<HashMap<String, TestStatus>>,
}

impl TestReport {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            results: Mutex::new(HashMap::new()),
        })
    }

    fn record_status(&self, provider: &str, status: TestStatus) {
        let mut results = self.results.lock().unwrap();
        results.insert(provider.to_string(), status);
    }

    fn record_pass(&self, provider: &str) {
        self.record_status(provider, TestStatus::Passed);
    }

    fn record_skip(&self, provider: &str) {
        self.record_status(provider, TestStatus::Skipped);
    }

    fn record_fail(&self, provider: &str) {
        self.record_status(provider, TestStatus::Failed);
    }

    fn print_summary(&self) {
        println!("\n============== Providers ==============");
        let results = self.results.lock().unwrap();
        let mut providers: Vec<_> = results.iter().collect();
        providers.sort_by(|a, b| a.0.cmp(b.0));

        for (provider, status) in providers {
            println!("{} {}", status, provider);
        }
        println!("=======================================\n");
    }
}

lazy_static::lazy_static! {
    static ref TEST_REPORT: Arc<TestReport> = TestReport::new();
    static ref ENV_LOCK: Mutex<()> = Mutex::new(());
}

/// Generic test harness for any Provider implementation
struct ProviderTester {
    provider: Box<dyn Provider + Send + Sync>,
    name: String,
}

impl ProviderTester {
    fn new<T: Provider + Send + Sync + 'static>(provider: T, name: String) -> Self {
        Self {
            provider: Box::new(provider),
            name,
        }
    }

    async fn test_basic_response(&self) -> Result<()> {
        let message = Message::user().with_text("Just say hello!");

        let (response, _) = self
            .provider
            .complete("You are a helpful assistant.", &[message], &[])
            .await?;

        // For a basic response, we expect a single text response
        assert_eq!(
            response.content.len(),
            1,
            "Expected single content item in response"
        );

        // Verify we got a text response
        assert!(
            matches!(response.content[0], MessageContent::Text(_)),
            "Expected text response"
        );

        Ok(())
    }

    async fn test_tool_usage(&self) -> Result<()> {
        let weather_tool = Tool::new(
            "get_weather",
            "Get the weather for a location",
            serde_json::json!({
                "type": "object",
                "required": ["location"],
                "properties": {
                    "location": {
                        "type": "string",
                        "description": "The city and state, e.g. San Francisco, CA"
                    }
                }
            }),
        );

        let message = Message::user().with_text("What's the weather like in San Francisco?");

        let (response1, _) = self
            .provider
            .complete(
                "You are a helpful weather assistant.",
                &[message.clone()],
                &[weather_tool.clone()],
            )
            .await?;

        println!("=== {}::reponse1 ===", self.name);
        dbg!(&response1);
        println!("===================");

        // Verify we got a tool request
        assert!(
            response1
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::ToolRequest(_))),
            "Expected tool request in response"
        );

        let id = &response1
            .content
            .iter()
            .filter_map(|message| message.as_tool_request())
            .last()
            .expect("got tool request")
            .id;

        let weather = Message::user().with_tool_response(
            id,
            Ok(vec![Content::text(
                "
                  50°F°C
                  Precipitation: 0%
                  Humidity: 84%
                  Wind: 2 mph
                  Weather
                  Saturday 9:00 PM
                  Clear",
            )]),
        );

        // Verify we construct a valid payload including the request/response pair for the next inference
        let (response2, _) = self
            .provider
            .complete(
                "You are a helpful weather assistant.",
                &[message, response1, weather],
                &[weather_tool],
            )
            .await?;

        println!("=== {}::reponse2 ===", self.name);
        dbg!(&response2);
        println!("===================");

        assert!(
            response2
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::Text(_))),
            "Expected text for final response"
        );

        Ok(())
    }

    /// Run all provider tests
    async fn run_test_suite(&self) -> Result<()> {
        self.test_basic_response().await?;
        self.test_tool_usage().await?;
        Ok(())
    }
}

fn load_env() {
    if let Ok(path) = dotenv() {
        println!("Loaded environment from {:?}", path);
    }
}

/// Helper function to run a provider test with proper error handling and reporting
async fn test_provider<F, T>(
    name: &str,
    required_vars: &[&str],
    env_modifications: Option<HashMap<&str, Option<String>>>,
    provider_fn: F,
) -> Result<()>
where
    F: FnOnce() -> Result<T>,
    T: Provider + Send + Sync + 'static,
{
    // We start off as failed, so that if the process panics it is seen as a failure
    TEST_REPORT.record_fail(name);

    // Take exclusive access to environment modifications
    let lock = ENV_LOCK.lock().unwrap();

    load_env();

    // Save current environment state for required vars and modified vars
    let mut original_env = HashMap::new();
    for &var in required_vars {
        if let Ok(val) = std::env::var(var) {
            original_env.insert(var, val);
        }
    }
    if let Some(mods) = &env_modifications {
        for &var in mods.keys() {
            if let Ok(val) = std::env::var(var) {
                original_env.insert(var, val);
            }
        }
    }

    // Apply any environment modifications
    if let Some(mods) = &env_modifications {
        for (&var, value) in mods.iter() {
            match value {
                Some(val) => std::env::set_var(var, val),
                None => std::env::remove_var(var),
            }
        }
    }

    // Setup the provider
    let missing_vars = required_vars.iter().any(|var| std::env::var(var).is_err());
    let provider = provider_fn();

    // Restore original environment
    for (&var, value) in original_env.iter() {
        std::env::set_var(var, value);
    }
    if let Some(mods) = env_modifications {
        for &var in mods.keys() {
            if !original_env.contains_key(var) {
                std::env::remove_var(var);
            }
        }
    }

    std::mem::drop(lock);

    if missing_vars {
        println!("Skipping {} tests - credentials not configured", name);
        TEST_REPORT.record_skip(name);
        return Ok(());
    }

    if provider.is_err() {
        println!("Could not setup {} from env", name);
        TEST_REPORT.record_fail(name);
        return Err(provider.err().expect("is error"));
    }

    let tester = ProviderTester::new(provider.expect("already checked"), name.to_string());
    match tester.run_test_suite().await {
        Ok(_) => {
            TEST_REPORT.record_pass(name);
            Ok(())
        }
        Err(e) => {
            println!("{} test failed: {}", name, e);
            TEST_REPORT.record_fail(name);
            Err(e)
        }
    }
}

#[tokio::test]
async fn test_openai_provider() -> Result<()> {
    test_provider(
        "OpenAI",
        &["OPENAI_API_KEY"],
        None,
        openai::OpenAiProvider::from_env,
    )
    .await
}

#[tokio::test]
async fn test_databricks_provider() -> Result<()> {
    test_provider(
        "Databricks",
        &["DATABRICKS_HOST", "DATABRICKS_TOKEN"],
        None,
        databricks::DatabricksProvider::from_env,
    )
    .await
}

#[tokio::test]
async fn test_databricks_provider_oauth() -> Result<()> {
    let mut env_mods = HashMap::new();
    env_mods.insert("DATABRICKS_TOKEN", None);

    test_provider(
        "Databricks OAuth",
        &["DATABRICKS_HOST"],
        Some(env_mods),
        databricks::DatabricksProvider::from_env,
    )
    .await
}

#[tokio::test]
async fn test_ollama_provider() -> Result<()> {
    test_provider(
        "Ollama",
        &["OLLAMA_HOST"],
        None,
        ollama::OllamaProvider::from_env,
    )
    .await
}

#[tokio::test]
async fn test_groq_provider() -> Result<()> {
    test_provider(
        "Groq",
        &["GROQ_API_KEY"],
        None,
        groq::GroqProvider::from_env,
    )
    .await
}

#[tokio::test]
async fn test_anthropic_provider() -> Result<()> {
    test_provider(
        "Anthropic",
        &["ANTHROPIC_API_KEY"],
        None,
        anthropic::AnthropicProvider::from_env,
    )
    .await
}

#[tokio::test]
async fn test_openrouter_provider() -> Result<()> {
    test_provider(
        "OpenRouter",
        &["OPENROUTER_API_KEY"],
        None,
        openrouter::OpenRouterProvider::from_env,
    )
    .await
}

#[tokio::test]
async fn test_google_provider() -> Result<()> {
    test_provider(
        "Google",
        &["GOOGLE_API_KEY"],
        None,
        google::GoogleProvider::from_env,
    )
    .await
}

// Print the final test report
#[ctor::dtor]
fn print_test_report() {
    TEST_REPORT.print_summary();
}
