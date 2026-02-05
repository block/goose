use anyhow::Result;
use dotenvy::dotenv;
use goose::config::ExtensionConfig;
use goose::conversation::message::{Message, MessageContent};
use goose::model::ModelConfig;
use goose::providers::anthropic::ANTHROPIC_DEFAULT_MODEL;
use goose::providers::azure::AZURE_DEFAULT_MODEL;
use goose::providers::base::Provider;
use goose::providers::bedrock::BEDROCK_DEFAULT_MODEL;
use goose::providers::claude_code::CLAUDE_CODE_DEFAULT_MODEL;
use goose::providers::codex::CODEX_DEFAULT_MODEL;
use goose::providers::create;
use goose::providers::databricks::DATABRICKS_DEFAULT_MODEL;
use goose::providers::errors::ProviderError;
use goose::providers::google::GOOGLE_DEFAULT_MODEL;
use goose::providers::litellm::LITELLM_DEFAULT_MODEL;
use goose::providers::ollama::OLLAMA_DEFAULT_MODEL;
use goose::providers::openai::OPEN_AI_DEFAULT_MODEL;
use goose::providers::sagemaker_tgi::SAGEMAKER_TGI_DEFAULT_MODEL;
use goose::providers::snowflake::SNOWFLAKE_DEFAULT_MODEL;
use goose::providers::xai::XAI_DEFAULT_MODEL;
use goose_test_support::{ExpectedSessionId, McpFixture, FAKE_CODE, FAKE_SCREENSHOT};
use rmcp::model::{Content, Tool};
use rmcp::object;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

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

struct ProviderTester {
    provider: Arc<dyn Provider>,
    name: String,
}

impl ProviderTester {
    fn new(provider: Arc<dyn Provider>, name: String) -> Self {
        Self { provider, name }
    }

    async fn test_basic_response(&self) -> Result<()> {
        let message = Message::user().with_text("Just say hello!");

        let (response, _) = self
            .provider
            .complete(
                "test-session-id",
                "You are a helpful assistant.",
                &[message],
                &[],
            )
            .await?;

        assert_eq!(
            response.content.len(),
            1,
            "Expected single content item in response"
        );

        assert!(
            matches!(response.content[0], MessageContent::Text(_)),
            "Expected text response"
        );

        Ok(())
    }

    async fn test_tool_usage(&self) -> Result<()> {
        // Invoke get_code from the test MCP server
        let lookup_tool = Tool::new(
            "get_code",
            "Get the lookup code",
            object!({
                "type": "object",
                "properties": {}
            }),
        );

        let message = Message::user().with_text("Use get_code to fetch the lookup code.");

        let (response1, _) = self
            .provider
            .complete(
                "test-session-id",
                "Use the get_code tool and output only its result.",
                std::slice::from_ref(&message),
                std::slice::from_ref(&lookup_tool),
            )
            .await?;

        println!("=== {}::reponse1 ===", self.name);
        dbg!(&response1);
        println!("===================");

        let tool_request = response1
            .content
            .iter()
            .filter_map(|message| message.as_tool_request())
            .next_back();

        let response_text: String = response1
            .content
            .iter()
            .filter_map(|c| c.as_text())
            .collect();

        // Agentic providers call tools internally
        if tool_request.is_none() {
            assert!(
                response_text.contains(FAKE_CODE),
                "Expected MCP lookup code in response"
            );
            return Ok(());
        }

        // LLM providers need us to invoke the tool directly and add its
        // response to the message list.
        let id = &tool_request.expect("got tool request").id;

        let lookup = Message::user().with_tool_response(
            id,
            Ok(rmcp::model::CallToolResult {
                content: vec![Content::text(FAKE_CODE)],
                structured_content: None,
                is_error: Some(false),
                meta: None,
            }),
        );

        let (response2, _) = self
            .provider
            .complete(
                "test-session-id",
                "Use the get_code tool and output only its result.",
                &[message, response1, lookup],
                &[lookup_tool],
            )
            .await?;

        println!("=== {}::reponse2 ===", self.name);
        dbg!(&response2);
        println!("===================");

        assert!(
            response2.as_concat_text().contains(FAKE_CODE),
            "Expected lookup code in final response"
        );

        Ok(())
    }

    async fn test_context_length_exceeded_error(&self) -> Result<()> {
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
            Message::user().with_text("what's the meaning of life?"),
            Message::assistant().with_text("the meaning of life is 42"),
            Message::user().with_text(
                "did I ask you what's 2+2 in this message history? just respond with 'yes' or 'no'",
            ),
        ];

        let result = self
            .provider
            .complete(
                "test-session-id",
                "You are a helpful assistant.",
                &messages,
                &[],
            )
            .await;

        println!("=== {}::context_length_exceeded_error ===", self.name);
        dbg!(&result);
        println!("===================");

        if self.name.to_lowercase() == "ollama"
            || self.name.to_lowercase() == "openrouter"
            || self.name.to_lowercase() == "claude-code"
        {
            assert!(
                result.is_ok(),
                "Expected to succeed because of default truncation or large context window"
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
        // Part 1: Inline image in user message
        let message_with_image = Message::user().with_image(
            FAKE_SCREENSHOT.as_str().to_string(),
            "image/png".to_string(),
        );

        let result = self
            .provider
            .complete(
                "test-session-id",
                "You are a helpful assistant. Describe what you see in the image briefly.",
                &[message_with_image],
                &[],
            )
            .await;

        println!("=== {}::image_content_support ===", self.name);
        let (response, _) = result?;
        println!("Image response: {:?}", response);
        assert!(
            response
                .content
                .iter()
                .any(|content| matches!(content, MessageContent::Text(_))),
            "Expected text response for image"
        );
        println!("===================");

        // Part 2: Image from tool response (follows test_tool_usage pattern)
        let screenshot_tool = Tool::new(
            "get_screenshot",
            "Get a screenshot of the current screen",
            object!({
                "type": "object",
                "properties": {}
            }),
        );

        let message = Message::user().with_text("Take a screenshot please");

        let (response1, _) = self
            .provider
            .complete(
                "test-session-id",
                "Use the get_screenshot tool and describe the result.",
                std::slice::from_ref(&message),
                std::slice::from_ref(&screenshot_tool),
            )
            .await?;

        println!("=== {}::tool_image_response1 ===", self.name);
        dbg!(&response1);
        println!("===================");

        let tool_request = response1
            .content
            .iter()
            .filter_map(|message| message.as_tool_request())
            .next_back();

        // Agentic providers call tools internally
        if tool_request.is_none() {
            return Ok(());
        }

        // LLM providers need us to invoke the tool and add its response
        let id = &tool_request.expect("got tool request").id;

        let tool_response = Message::user().with_tool_response(
            id,
            Ok(rmcp::model::CallToolResult {
                content: vec![Content::image(FAKE_SCREENSHOT.as_str(), "image/png")],
                structured_content: None,
                is_error: Some(false),
                meta: None,
            }),
        );

        let (response2, _) = self
            .provider
            .complete(
                "test-session-id",
                "Use the get_screenshot tool and describe the result.",
                &[message, response1, tool_response],
                &[screenshot_tool],
            )
            .await?;

        println!("=== {}::tool_image_response2 ===", self.name);
        dbg!(&response2);
        println!("===================");

        Ok(())
    }

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

async fn test_provider(
    name: &str,
    model_name: &str,
    required_vars: &[&str],
    env_modifications: Option<HashMap<&str, Option<String>>>,
) -> Result<()> {
    TEST_REPORT.record_fail(name);

    let original_env = {
        let _lock = ENV_LOCK.lock().unwrap();

        load_env();

        // Check required_vars BEFORE applying env_modifications to avoid
        // leaving the environment mutated when skipping
        let missing_vars = required_vars.iter().any(|var| std::env::var(var).is_err());
        if missing_vars {
            println!("Skipping {} tests - credentials not configured", name);
            TEST_REPORT.record_skip(name);
            return Ok(());
        }

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

        if let Some(mods) = &env_modifications {
            for (&var, value) in mods.iter() {
                match value {
                    Some(val) => std::env::set_var(var, val),
                    None => std::env::remove_var(var),
                }
            }
        }

        original_env
    };

    let expected_session_id = ExpectedSessionId::default();
    let provider_name = name.to_lowercase();
    let expected_session_id_for_mcp = if matches!(provider_name.as_str(), "codex" | "claude-code") {
        None
    } else {
        Some(expected_session_id.clone())
    };
    let mcp = McpFixture::new(expected_session_id_for_mcp).await;
    let extensions = vec![ExtensionConfig::streamable_http(
        "lookup",
        &mcp.url,
        "Lookup server",
        30_u64,
    )];
    let model_config = ModelConfig::new(model_name)?;
    expected_session_id.set("test-session-id");

    let provider = match create(&provider_name, model_config, extensions).await {
        Ok(p) => p,
        Err(e) => {
            println!("Skipping {} tests - failed to create provider: {}", name, e);
            TEST_REPORT.record_skip(name);
            return Ok(());
        }
    };

    {
        let _lock = ENV_LOCK.lock().unwrap();
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
    }

    let tester = ProviderTester::new(provider, name.to_string());
    let _mcp = mcp;
    let result = tester.run_test_suite().await;

    match result {
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
    test_provider("openai", OPEN_AI_DEFAULT_MODEL, &["OPENAI_API_KEY"], None).await
}

#[tokio::test]
async fn test_azure_provider() -> Result<()> {
    test_provider(
        "Azure",
        AZURE_DEFAULT_MODEL,
        &[
            "AZURE_OPENAI_API_KEY",
            "AZURE_OPENAI_ENDPOINT",
            "AZURE_OPENAI_DEPLOYMENT_NAME",
        ],
        None,
    )
    .await
}

#[tokio::test]
async fn test_bedrock_provider_long_term_credentials() -> Result<()> {
    test_provider(
        "aws_bedrock",
        BEDROCK_DEFAULT_MODEL,
        &["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
        None,
    )
    .await
}

#[tokio::test]
async fn test_bedrock_provider_aws_profile_credentials() -> Result<()> {
    let env_mods =
        HashMap::from_iter([("AWS_ACCESS_KEY_ID", None), ("AWS_SECRET_ACCESS_KEY", None)]);

    test_provider(
        "aws_bedrock",
        BEDROCK_DEFAULT_MODEL,
        &["AWS_PROFILE"],
        Some(env_mods),
    )
    .await
}

#[tokio::test]
async fn test_bedrock_provider_bearer_token() -> Result<()> {
    // Clear standard AWS credentials to ensure bearer token auth is used
    let env_mods = HashMap::from_iter([
        ("AWS_ACCESS_KEY_ID", None),
        ("AWS_SECRET_ACCESS_KEY", None),
        ("AWS_PROFILE", None),
    ]);

    test_provider(
        "aws_bedrock",
        BEDROCK_DEFAULT_MODEL,
        &["AWS_BEARER_TOKEN_BEDROCK", "AWS_REGION"],
        Some(env_mods),
    )
    .await
}

#[tokio::test]
async fn test_databricks_provider() -> Result<()> {
    test_provider(
        "Databricks",
        DATABRICKS_DEFAULT_MODEL,
        &["DATABRICKS_HOST", "DATABRICKS_TOKEN"],
        None,
    )
    .await
}

#[tokio::test]
async fn test_ollama_provider() -> Result<()> {
    test_provider("Ollama", OLLAMA_DEFAULT_MODEL, &["OLLAMA_HOST"], None).await
}

#[tokio::test]
async fn test_anthropic_provider() -> Result<()> {
    test_provider(
        "Anthropic",
        ANTHROPIC_DEFAULT_MODEL,
        &["ANTHROPIC_API_KEY"],
        None,
    )
    .await
}

#[tokio::test]
async fn test_openrouter_provider() -> Result<()> {
    test_provider(
        "OpenRouter",
        OPEN_AI_DEFAULT_MODEL,
        &["OPENROUTER_API_KEY"],
        None,
    )
    .await
}

#[tokio::test]
async fn test_google_provider() -> Result<()> {
    test_provider("Google", GOOGLE_DEFAULT_MODEL, &["GOOGLE_API_KEY"], None).await
}

#[tokio::test]
async fn test_snowflake_provider() -> Result<()> {
    test_provider(
        "Snowflake",
        SNOWFLAKE_DEFAULT_MODEL,
        &["SNOWFLAKE_HOST", "SNOWFLAKE_TOKEN"],
        None,
    )
    .await
}

#[tokio::test]
async fn test_sagemaker_tgi_provider() -> Result<()> {
    test_provider(
        "SageMakerTgi",
        SAGEMAKER_TGI_DEFAULT_MODEL,
        &["SAGEMAKER_ENDPOINT_NAME"],
        None,
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

    test_provider("LiteLLM", LITELLM_DEFAULT_MODEL, &[], Some(env_mods)).await
}

#[tokio::test]
async fn test_xai_provider() -> Result<()> {
    test_provider("Xai", XAI_DEFAULT_MODEL, &["XAI_API_KEY"], None).await
}

#[tokio::test]
async fn test_claude_code_provider() -> Result<()> {
    if which::which("claude").is_err() {
        println!("'claude' CLI not found, skipping test");
        TEST_REPORT.record_skip("claude-code");
        return Ok(());
    }
    test_provider("claude-code", CLAUDE_CODE_DEFAULT_MODEL, &[], None).await
}

#[tokio::test]
async fn test_codex_provider() -> Result<()> {
    if which::which("codex").is_err() {
        println!("'codex' CLI not found, skipping test");
        TEST_REPORT.record_skip("codex");
        return Ok(());
    }
    test_provider("codex", CODEX_DEFAULT_MODEL, &[], None).await
}

#[ctor::dtor]
fn print_test_report() {
    TEST_REPORT.print_summary();
}
