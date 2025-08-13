use anyhow::Result;
use dotenvy::dotenv;
use goose::conversation::message::{Message, MessageContent};
use goose::providers::base::Provider;
use goose::providers::errors::ProviderError;
use goose::providers::{
    anthropic, azure, bedrock, databricks, google, groq, litellm, ollama, openai, openrouter,
    snowflake, xai,
};
use rmcp::model::Tool;
use rmcp::model::{AnnotateAble, Content, RawImageContent};
use rmcp::object;
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
    provider: Arc<dyn Provider>,
    name: String,
}

impl ProviderTester {
    fn new<T: Provider + Send + Sync + 'static>(provider: T, name: String) -> Self {
        Self {
            provider: Arc::new(provider),
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
            object!({
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

    async fn test_context_length_exceeded_error(&self) -> Result<()> {
        // Google Gemini has a really long context window
        let large_message_content = if self.name.to_lowercase() == "google" {
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
        let result = self
            .provider
            .complete("You are a helpful assistant.", &messages, &[])
            .await;

        // Print some debug info
        println!("=== {}::context_length_exceeded_error ===", self.name);
        dbg!(&result);
        println!("===================");

        // Ollama truncates by default even when the context window is exceeded
        if self.name.to_lowercase() == "ollama" {
            assert!(
                result.is_ok(),
                "Expected to succeed because of default truncation"
            );
            return Ok(());
        }

        assert!(
            result.is_err(),
            "Expected error when context window is exceeded"
        );
        assert!(
            matches!(result.unwrap_err(), ProviderError::ContextLengthExceeded(_)),
            "Expected error to be ContextLengthExceeded"
        );

        Ok(())
    }

    async fn test_image_content_support(&self) -> Result<()> {
        use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
        use goose::conversation::message::Message;
        use std::fs;

        // Try to read the test image
        let image_path = "crates/goose/examples/test_assets/test_image.png";
        let image_data = match fs::read(image_path) {
            Ok(data) => data,
            Err(_) => {
                println!(
                    "Test image not found at {}, skipping image test",
                    image_path
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
        let message_with_image =
            Message::user().with_image(image_content.data.clone(), image_content.mime_type.clone());

        let result = self
            .provider
            .complete(
                "You are a helpful assistant. Describe what you see in the image briefly.",
                &[message_with_image],
                &[],
            )
            .await;

        println!("=== {}::image_content_support ===", self.name);
        let (response, _) = result?;
        println!("Image response: {:?}", response);
        // Verify we got a text response
        assert!(
            response
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::Text(_))),
            "Expected text response for image"
        );
        println!("===================");

        // Test 2: Tool response with image (this should be handled gracefully)
        let screenshot_tool = Tool::new(
            "get_screenshot",
            "Get a screenshot of the current screen",
            object!({
                "type": "object",
                "properties": {}
            }),
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

        let result2 = self
            .provider
            .complete(
                "You are a helpful assistant.",
                &[user_message, tool_request, tool_response],
                &[screenshot_tool],
            )
            .await;

        println!("=== {}::tool_image_response ===", self.name);
        let (response, _) = result2?;
        println!("Tool image response: {:?}", response);
        println!("===================");

        Ok(())
    }

    /// Run all provider tests
    async fn run_test_suite(&self) -> Result<()> {
        self.test_basic_response().await?;
        self.test_tool_usage().await?;
        self.test_context_length_exceeded_error().await?;
        self.test_image_content_support().await?;
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
    F: FnOnce() -> T,
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
    if missing_vars {
        println!("Skipping {} tests - credentials not configured", name);
        TEST_REPORT.record_skip(name);
        return Ok(());
    }

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

    let tester = ProviderTester::new(provider, name.to_string());
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
        openai::OpenAiProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_azure_provider() -> Result<()> {
    test_provider(
        "Azure",
        &[
            "AZURE_OPENAI_API_KEY",
            "AZURE_OPENAI_ENDPOINT",
            "AZURE_OPENAI_DEPLOYMENT_NAME",
        ],
        None,
        azure::AzureProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_bedrock_provider_long_term_credentials() -> Result<()> {
    test_provider(
        "Bedrock",
        &["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
        None,
        bedrock::BedrockProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_bedrock_provider_aws_profile_credentials() -> Result<()> {
    let env_mods = HashMap::from_iter([
        // Ensure to unset long-term credentials to use AWS Profile provider
        ("AWS_ACCESS_KEY_ID", None),
        ("AWS_SECRET_ACCESS_KEY", None),
    ]);

    test_provider(
        "Bedrock AWS Profile Credentials",
        &["AWS_PROFILE"],
        Some(env_mods),
        bedrock::BedrockProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_databricks_provider() -> Result<()> {
    test_provider(
        "Databricks",
        &["DATABRICKS_HOST", "DATABRICKS_TOKEN"],
        None,
        databricks::DatabricksProvider::default,
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
        databricks::DatabricksProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_ollama_provider() -> Result<()> {
    test_provider(
        "Ollama",
        &["OLLAMA_HOST"],
        None,
        ollama::OllamaProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_groq_provider() -> Result<()> {
    test_provider("Groq", &["GROQ_API_KEY"], None, groq::GroqProvider::default).await
}

#[tokio::test]
async fn test_anthropic_provider() -> Result<()> {
    test_provider(
        "Anthropic",
        &["ANTHROPIC_API_KEY"],
        None,
        anthropic::AnthropicProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_openrouter_provider() -> Result<()> {
    test_provider(
        "OpenRouter",
        &["OPENROUTER_API_KEY"],
        None,
        openrouter::OpenRouterProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_google_provider() -> Result<()> {
    test_provider(
        "Google",
        &["GOOGLE_API_KEY"],
        None,
        google::GoogleProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_snowflake_provider() -> Result<()> {
    test_provider(
        "Snowflake",
        &["SNOWFLAKE_HOST", "SNOWFLAKE_TOKEN"],
        None,
        snowflake::SnowflakeProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_sagemaker_tgi_provider() -> Result<()> {
    test_provider(
        "SageMakerTgi",
        &["SAGEMAKER_ENDPOINT_NAME"],
        None,
        goose::providers::sagemaker_tgi::SageMakerTgiProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_litellm_provider() -> Result<()> {
    if std::env::var("LITELLM_HOST").is_err() {
        println!("LITELLM_HOST not set, skipping test");
        TEST_REPORT.record_skip("LiteLLM");
        return Ok(());
    }

    let env_mods = HashMap::from_iter([
        ("LITELLM_HOST", Some("http://localhost:4000".to_string())),
        ("LITELLM_API_KEY", Some("".to_string())),
    ]);

    test_provider(
        "LiteLLM",
        &[], // No required environment variables
        Some(env_mods),
        litellm::LiteLLMProvider::default,
    )
    .await
}

#[tokio::test]
async fn test_xai_provider() -> Result<()> {
    test_provider("Xai", &["XAI_API_KEY"], None, xai::XaiProvider::default).await
}

// Print the final test report
#[ctor::dtor]
fn print_test_report() {
    TEST_REPORT.print_summary();
}
