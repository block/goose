use anyhow::Result;
use dotenvy::dotenv;
use goose::message::{Message, MessageContent};
use goose::model::ModelConfig;
use goose::providers::base::Provider;
use goose::providers::errors::ProviderError;
use mcp_core::tool::Tool;
use rmcp::model::{AnnotateAble, Content, RawImageContent};
use std::collections::HashMap;
use std::future::Future;
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

#[derive(Debug, Clone)]
struct ProviderConfig {
    name: &'static str,
    factory_name: &'static str,
    required_env_vars: &'static [&'static str],
    env_modifications: Option<HashMap<&'static str, Option<String>>>,
}

// Configuration for all providers
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
        name: "Bedrock AWS Profile",
        factory_name: "aws_bedrock",
        required_env_vars: &["AWS_PROFILE"],
        env_modifications: Some({
            let mut map = HashMap::new();
            map.insert("AWS_ACCESS_KEY_ID", None);
            map.insert("AWS_SECRET_ACCESS_KEY", None);
            map
        }),
    },
    ProviderConfig {
        name: "Databricks",
        factory_name: "databricks",
        required_env_vars: &["DATABRICKS_HOST", "DATABRICKS_TOKEN"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Databricks OAuth",
        factory_name: "databricks",
        required_env_vars: &["DATABRICKS_HOST"],
        env_modifications: Some({
            let mut map = HashMap::new();
            map.insert("DATABRICKS_TOKEN", None);
            map
        }),
    },
    ProviderConfig {
        name: "Google",
        factory_name: "google",
        required_env_vars: &["GOOGLE_API_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "GCP Vertex AI",
        factory_name: "gcp_vertex_ai",
        required_env_vars: &["GOOGLE_APPLICATION_CREDENTIALS"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Groq",
        factory_name: "groq",
        required_env_vars: &["GROQ_API_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "LiteLLM",
        factory_name: "litellm",
        required_env_vars: &[],
        env_modifications: Some({
            let mut map = HashMap::new();
            map.insert("LITELLM_HOST", Some("http://localhost:4000".to_string()));
            map.insert("LITELLM_API_KEY", Some("".to_string()));
            map
        }),
    },
    ProviderConfig {
        name: "Ollama",
        factory_name: "ollama",
        required_env_vars: &["OLLAMA_HOST"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "OpenRouter",
        factory_name: "openrouter",
        required_env_vars: &["OPENROUTER_API_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "SageMaker TGI",
        factory_name: "sagemaker_tgi",
        required_env_vars: &["SAGEMAKER_ENDPOINT_NAME"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Snowflake",
        factory_name: "snowflake",
        required_env_vars: &["SNOWFLAKE_HOST", "SNOWFLAKE_TOKEN"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Venice",
        factory_name: "venice",
        required_env_vars: &["VENICE_API_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "XAI",
        factory_name: "xai",
        required_env_vars: &["XAI_API_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Claude Code",
        factory_name: "claude-code",
        required_env_vars: &["ANTHROPIC_API_KEY"],
        env_modifications: None,
    },
    ProviderConfig {
        name: "Gemini CLI",
        factory_name: "gemini-cli",
        required_env_vars: &["GOOGLE_API_KEY"],
        env_modifications: None,
    },
];

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

fn load_env() {
    if let Ok(path) = dotenv() {
        println!("Loaded environment from {:?}", path);
    }
}

/// Helper function that runs a test function against all available providers
async fn run_all_providers<F, Fut>(test_name: &str, test_fn: F) -> Result<()>
where
    F: Fn(Arc<dyn Provider>, &str) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    let mut results = Vec::new();

    for config in PROVIDER_CONFIGS {
        let result = run_provider_test(config, &test_fn).await;
        results.push((config.name, result));
    }

    // Report results to the global test report
    for (name, result) in &results {
        match result {
            Ok(_) => TEST_REPORT.record_pass(&format!("{} - {}", test_name, name)),
            Err(_) => TEST_REPORT.record_fail(&format!("{} - {}", test_name, name)),
        }
    }

    // Report any failures
    let failures: Vec<_> = results
        .iter()
        .filter(|(_, result)| result.is_err())
        .collect();

    if !failures.is_empty() {
        println!("Failed providers for {}:", test_name);
        for (name, error) in failures {
            println!("  {}: {:?}", name, error);
        }
    }

    Ok(())
}

async fn run_provider_test<F, Fut>(config: &ProviderConfig, test_fn: &F) -> Result<()>
where
    F: Fn(Arc<dyn Provider>, &str) -> Fut,
    Fut: Future<Output = Result<()>>,
{
    // Take exclusive access to environment modifications
    let lock = ENV_LOCK.lock().unwrap();

    load_env();

    // Save current environment state
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

    // Apply environment modifications
    if let Some(mods) = &config.env_modifications {
        for (&var, value) in mods.iter() {
            match value {
                Some(val) => std::env::set_var(var, val),
                None => std::env::remove_var(var),
            }
        }
    }

    // Check if required environment variables are available
    let missing_vars = config
        .required_env_vars
        .iter()
        .any(|var| std::env::var(var).is_err());
    if missing_vars {
        println!(
            "Skipping {} tests - credentials not configured",
            config.name
        );
        TEST_REPORT.record_skip(&format!("skipped - {}", config.name));
        return Ok(()); // Skip, don't fail
    }

    // Special case for LiteLLM - skip if host not set
    if config.factory_name == "litellm" && std::env::var("LITELLM_HOST").is_err() {
        println!("LITELLM_HOST not set, skipping test");
        TEST_REPORT.record_skip(&format!("skipped - {}", config.name));
        return Ok(());
    }

    // Create provider using factory
    let model_config = ModelConfig::default();
    let provider = match goose::providers::create(config.factory_name, model_config) {
        Ok(p) => p,
        Err(e) => {
            println!("Failed to create provider {}: {}", config.name, e);
            return Err(e);
        }
    };

    // Restore original environment
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

    std::mem::drop(lock);

    // Run the actual test
    test_fn(provider, config.name).await
}

// Individual test functions

#[tokio::test]
async fn test_basic_response() -> Result<()> {
    run_all_providers("basic_response", |provider, provider_name| async move {
        let message = Message::user().with_text("Just say hello!");

        let (response, _) = provider
            .complete("You are a helpful assistant.", &[message], &[])
            .await?;

        // For a basic response, we expect a single text response
        assert_eq!(
            response.content.len(),
            1,
            "Expected single content item in response for {}",
            provider_name
        );

        // Verify we got a text response
        assert!(
            matches!(response.content[0], MessageContent::Text(_)),
            "Expected text response for {}",
            provider_name
        );

        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_tool_usage() -> Result<()> {
    run_all_providers("tool_usage", |provider, provider_name| async move {
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
            None,
        );

        let message = Message::user().with_text("What's the weather like in San Francisco?");

        let (response1, _) = provider
            .complete(
                "You are a helpful weather assistant.",
                &[message.clone()],
                &[weather_tool.clone()],
            )
            .await?;

        println!("=== {}::response1 ===", provider_name);
        dbg!(&response1);
        println!("===================");

        // Verify we got a tool request
        assert!(
            response1
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::ToolRequest(_))),
            "Expected tool request in response for {}",
            provider_name
        );

        let id = &response1
            .content
            .iter()
            .filter_map(|message| message.as_tool_request())
            .next_back()
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
        let (response2, _) = provider
            .complete(
                "You are a helpful weather assistant.",
                &[message, response1, weather],
                &[weather_tool],
            )
            .await?;

        println!("=== {}::response2 ===", provider_name);
        dbg!(&response2);
        println!("===================");

        assert!(
            response2
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::Text(_))),
            "Expected text for final response for {}",
            provider_name
        );

        Ok(())
    })
    .await
}

#[tokio::test]
async fn test_context_length_exceeded_error() -> Result<()> {
    run_all_providers(
        "context_length_exceeded",
        |provider, provider_name| async move {
            // Google Gemini has a really long context window
            let large_message_content = if provider_name.to_lowercase().contains("google")
                || provider_name.to_lowercase().contains("gemini")
            {
                "hello ".repeat(1_300_000)
            } else {
                "hello ".repeat(300_000)
            };

            let messages = vec![
            Message::user().with_text("hi there. what is 2 + 2?"),
            Message::assistant().with_text("hey! I think it's 4."),
            Message::user().with_text(&large_message_content),
            Message::assistant().with_text("heyy!!"),
            // Messages before this mark should be truncated
            Message::user().with_text("what's the meaning of life?"),
            Message::assistant().with_text("the meaning of life is 42"),
            Message::user().with_text(
                "did I ask you what's 2+2 in this message history? just respond with 'yes' or 'no'",
            ),
        ];

            // Test that we get ProviderError::ContextLengthExceeded when the context window is exceeded
            let result = provider
                .complete("You are a helpful assistant.", &messages, &[])
                .await;

            // Print some debug info
            println!("=== {}::context_length_exceeded_error ===", provider_name);
            dbg!(&result);
            println!("===================");

            // Ollama truncates by default even when the context window is exceeded
            if provider_name.to_lowercase().contains("ollama") {
                assert!(
                    result.is_ok(),
                    "Expected to succeed because of default truncation for {}",
                    provider_name
                );
                return Ok(());
            }

            assert!(
                result.is_err(),
                "Expected error when context window is exceeded for {}",
                provider_name
            );
            assert!(
                matches!(result.unwrap_err(), ProviderError::ContextLengthExceeded(_)),
                "Expected error to be ContextLengthExceeded for {}",
                provider_name
            );

            Ok(())
        },
    )
    .await
}

#[tokio::test]
async fn test_image_content_support() -> Result<()> {
    run_all_providers(
        "image_content_support",
        |provider, provider_name| async move {
            use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
            use std::fs;

            // Try to read the test image
            let image_path = "crates/goose/examples/test_assets/test_image.png";
            let image_data = match fs::read(image_path) {
                Ok(data) => data,
                Err(_) => {
                    println!(
                        "Test image not found at {}, skipping image test for {}",
                        image_path, provider_name
                    );
                    return Ok(());
                }
            };

            let base64_image = BASE64.encode(image_data);
            let image_content = RawImageContent {
                data: base64_image,
                mime_type: "image/png".to_string(),
            }
            .no_annotation();

            // Test 1: Direct image message
            let message_with_image = Message::user()
                .with_image(image_content.data.clone(), image_content.mime_type.clone());

            let result = provider
                .complete(
                    "You are a helpful assistant. Describe what you see in the image briefly.",
                    &[message_with_image],
                    &[],
                )
                .await;

            println!("=== {}::image_content_support ===", provider_name);
            let (response, _) = result?;
            println!("Image response: {:?}", response);
            // Verify we got a text response
            assert!(
                response
                    .content
                    .iter()
                    .any(|content| matches!(content, MessageContent::Text(_))),
                "Expected text response for image for {}",
                provider_name
            );
            println!("===================");

            // Test 2: Tool response with image (this should be handled gracefully)
            let screenshot_tool = Tool::new(
                "get_screenshot",
                "Get a screenshot of the current screen",
                serde_json::json!({
                    "type": "object",
                    "properties": {}
                }),
                None,
            );

            let user_message = Message::user().with_text("Take a screenshot please");
            let tool_request = Message::assistant().with_tool_request(
                "test_id",
                Ok(mcp_core::tool::ToolCall::new(
                    "get_screenshot",
                    serde_json::json!({}),
                )),
            );
            let tool_response = Message::user().with_tool_response(
                "test_id",
                Ok(vec![Content::image(
                    image_content.data.clone(),
                    image_content.mime_type.clone(),
                )]),
            );

            let result2 = provider
                .complete(
                    "You are a helpful assistant.",
                    &[user_message, tool_request, tool_response],
                    &[screenshot_tool],
                )
                .await;

            println!("=== {}::tool_image_response ===", provider_name);
            let (response, _) = result2?;
            println!("Tool image response: {:?}", response);
            println!("===================");

            Ok(())
        },
    )
    .await
}

// Print the final test report
#[ctor::dtor]
fn print_test_report() {
    TEST_REPORT.print_summary();
}
