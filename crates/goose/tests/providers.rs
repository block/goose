use anyhow::Result;
use dotenvy::dotenv;
use goose::agents::extension_manager::ExtensionManagerCapabilities;
use goose::agents::{ExtensionManager, GoosePlatform, PromptManager};
use goose::config::ExtensionConfig;
use goose::conversation::message::{Message, MessageContent};
use goose::providers::anthropic::ANTHROPIC_DEFAULT_MODEL;
use goose::providers::azure::AZURE_DEFAULT_MODEL;
use goose::providers::base::Provider;
use goose::providers::bedrock::BEDROCK_DEFAULT_MODEL;
use goose::providers::claude_code::CLAUDE_CODE_DEFAULT_MODEL;
use goose::providers::codex::CODEX_DEFAULT_MODEL;
use goose::providers::create_with_named_model;
use goose::providers::databricks::DATABRICKS_DEFAULT_MODEL;
use goose::providers::errors::ProviderError;
use goose::providers::google::GOOGLE_DEFAULT_MODEL;
use goose::providers::litellm::LITELLM_DEFAULT_MODEL;
use goose::providers::openai::OPEN_AI_DEFAULT_MODEL;
use goose::providers::sagemaker_tgi::SAGEMAKER_TGI_DEFAULT_MODEL;
use goose::providers::snowflake::SNOWFLAKE_DEFAULT_MODEL;
use goose::providers::xai::XAI_DEFAULT_MODEL;
use goose::session::SessionManager;
use goose_test_support::{ExpectedSessionId, McpFixture, FAKE_CODE, TEST_SESSION_ID};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio_util::sync::CancellationToken;

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
    extension_manager: Arc<ExtensionManager>,
    is_cli_provider: bool,
    model_switch_name: Option<String>,
}

impl ProviderTester {
    fn new(
        provider: Arc<dyn Provider>,
        name: String,
        extension_manager: Arc<ExtensionManager>,
        is_cli_provider: bool,
        model_switch_name: Option<String>,
    ) -> Self {
        Self {
            provider,
            name,
            extension_manager,
            is_cli_provider,
            model_switch_name,
        }
    }

    async fn tool_roundtrip(&self, prompt: &str, session_id: &str) -> Result<Message> {
        let tools = self
            .extension_manager
            .get_prefixed_tools(session_id, None)
            .await
            .expect("get_prefixed_tools failed");

        let info = self
            .extension_manager
            .get_extensions_info(std::path::Path::new("."))
            .await;
        let system = PromptManager::new()
            .builder()
            .with_extensions(info.into_iter())
            .build();

        let message = Message::user().with_text(prompt);
        let model_config = self.provider.get_model_config();
        let (response1, _) = self
            .provider
            .complete(
                &model_config,
                session_id,
                &system,
                std::slice::from_ref(&message),
                &tools,
            )
            .await?;

        // Agentic CLI providers (claude-code, codex) call tools internally and
        // return the final text result directly — no tool_request in the response.
        let tool_req = response1
            .content
            .iter()
            .filter_map(|c| c.as_tool_request())
            .next_back();

        let tool_req = match tool_req {
            Some(req) => req,
            None => return Ok(response1),
        };

        let params = tool_req
            .tool_call
            .as_ref()
            .expect("tool_call should be Ok")
            .clone();
        let result = self
            .extension_manager
            .dispatch_tool_call(session_id, params, None, CancellationToken::new())
            .await
            .expect("dispatch failed")
            .result
            .await
            .expect("tool call failed");
        let tool_response = Message::user().with_tool_response(&tool_req.id, Ok(result));

        let (response2, _) = self
            .provider
            .complete(
                &model_config,
                session_id,
                &system,
                &[message, response1, tool_response],
                &tools,
            )
            .await?;
        Ok(response2)
    }

    async fn test_basic_response(&self, session_id: &str) -> Result<()> {
        let message = Message::user().with_text("Just say hello!");
        let model_config = self.provider.get_model_config();

        let (response, _) = self
            .provider
            .complete(
                &model_config,
                session_id,
                "You are a helpful assistant.",
                &[message],
                &[],
            )
            .await?;

        assert!(
            !response.content.is_empty(),
            "Expected at least one content item in response"
        );

        assert!(
            response
                .content
                .iter()
                .any(|c| matches!(c, MessageContent::Text(_))),
            "Expected at least one text content item in response"
        );

        println!(
            "=== {}::basic_response === {}",
            self.name,
            response.as_concat_text()
        );
        Ok(())
    }

    async fn test_tool_usage(&self, session_id: &str) -> Result<()> {
        let response = self
            .tool_roundtrip(
                "Use the get_code tool and output only its result.",
                session_id,
            )
            .await?;
        let text = response.as_concat_text();
        assert!(
            text.contains(FAKE_CODE),
            "Expected lookup code '{}' in final response, got: {}",
            FAKE_CODE,
            text
        );
        println!("=== {}::tool_usage === {}", self.name, text);
        Ok(())
    }

    async fn test_context_length_exceeded_error(&self, session_id: &str) -> Result<()> {
        let large_message_content = if self.name.to_lowercase() == "google" {
            "hello ".repeat(1_300_000)
        } else {
            "hello ".repeat(300_000)
        };

        let messages = vec![Message::user().with_text(&large_message_content)];
        let model_config = self.provider.get_model_config();

        let result = self
            .provider
            .complete(
                &model_config,
                session_id,
                "You are a helpful assistant.",
                &messages,
                &[],
            )
            .await;

        println!("=== {}::context_length_exceeded_error ===", self.name);
        dbg!(&result);
        println!("===================");

        let name_lower = self.name.to_lowercase();
        if name_lower == "ollama" || name_lower == "openrouter" {
            // These providers handle context overflow internally: ollama and
            // openrouter truncate or have large windows.
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

    async fn test_image_content_support(&self, session_id: &str) -> Result<()> {
        let response = self
            .tool_roundtrip(
                "Use the get_image tool and describe what you see in its result.",
                session_id,
            )
            .await?;
        let text = response.as_concat_text().to_lowercase();
        assert!(
            text.contains("hello goose") || text.contains("test image"),
            "Expected response to describe the test image, got: {}",
            text
        );
        println!("=== {}::image_content === {}", self.name, text);
        Ok(())
    }

    async fn test_model_switch(&self, session_id: &str) -> Result<()> {
        let default = &self.provider.get_model_config().model_name;
        let alt = self
            .model_switch_name
            .as_deref()
            .expect("model_switch_name required for test_model_switch");
        let alt_config = goose::model::ModelConfig::new(alt)?.with_canonical_limits(&self.name);

        let message = Message::user().with_text("Just say hello!");
        let (response, _) = self
            .provider
            .complete(
                &alt_config,
                session_id,
                "You are a helpful assistant.",
                &[message],
                &[],
            )
            .await?;

        assert!(
            matches!(response.content.first(), Some(MessageContent::Text(_))),
            "Expected text response after model switch"
        );
        println!(
            "=== {}::model_switch ({} -> {}) === {}",
            self.name,
            default,
            alt,
            response.as_concat_text()
        );
        Ok(())
    }

    async fn test_model_listing(&self) -> Result<()> {
        let models = self.provider.fetch_supported_models().await?;

        println!("=== {}::model_listing ===", self.name);
        dbg!(&models);
        println!("===================");

        assert!(!models.is_empty(), "Expected non-empty model list");
        let model_name = &self.provider.get_model_config().model_name;
        // Model names may not match exactly: Ollama adds tags like "qwen3:latest",
        // and CLI providers like claude-code use aliases (e.g. "sonnet") that are
        // substrings of full model names (e.g. "claude-sonnet-4-5-20250929").
        assert!(
            models
                .iter()
                .any(|m| m == model_name || m.contains(model_name) || model_name.contains(m)),
            "Expected model '{}' in supported models",
            model_name
        );
        if let Some(alt) = &self.model_switch_name {
            assert!(
                models
                    .iter()
                    .any(|m| m == alt || m.contains(alt.as_str()) || alt.contains(m.as_str())),
                "Expected model_switch_name '{}' in supported models",
                alt
            );
        }
        Ok(())
    }

    fn session_id_for_test(&self, test_name: &str) -> String {
        if self.is_cli_provider {
            format!("test_{}", test_name)
        } else {
            TEST_SESSION_ID.to_string()
        }
    }

    async fn run_test_suite(&self) -> Result<()> {
        self.test_model_listing().await?;
        self.test_basic_response(&self.session_id_for_test("basic_response"))
            .await?;
        self.test_tool_usage(&self.session_id_for_test("tool_usage"))
            .await?;
        self.test_image_content_support(&self.session_id_for_test("image_content"))
            .await?;
        if self.model_switch_name.is_some() {
            self.test_model_switch(&self.session_id_for_test("model_switch"))
                .await?;
        }
        // claude-code responds unpredictably to oversized context:
        // sometimes "no", sometimes "Prompt is too long".
        if self.name != "claude-code" {
            self.test_context_length_exceeded_error(&self.session_id_for_test("context_length"))
                .await?;
        }
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
    model_switch_name: Option<&str>,
    required_vars: &[&str],
    env_modifications: Option<HashMap<&str, Option<String>>>,
    // CLI providers cannot propagate the agent-session-id header to MCP servers.
    is_cli_provider: bool,
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

    let provider_name = name.to_lowercase();
    let expected_session_id = if is_cli_provider {
        None
    } else {
        Some(ExpectedSessionId::default())
    };
    let mcp = McpFixture::new(expected_session_id.clone()).await;
    if let Some(ref id) = expected_session_id {
        id.set(TEST_SESSION_ID);
    }

    let mcp_extension =
        ExtensionConfig::streamable_http("mcp-fixture", &mcp.url, "MCP fixture", 30_u64);

    let provider = match create_with_named_model(
        &provider_name,
        model_name,
        vec![mcp_extension.clone()],
    )
    .await
    {
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

    let temp_dir = tempfile::tempdir()?;
    let shared_provider = Arc::new(tokio::sync::Mutex::new(Some(provider.clone())));
    let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
    let extension_manager = Arc::new(ExtensionManager::new(
        shared_provider,
        session_manager,
        GoosePlatform::GooseCli.to_string(),
        ExtensionManagerCapabilities { mcpui: false },
    ));
    extension_manager
        .add_extension(mcp_extension, None, None, None)
        .await
        .expect("failed to add extension");

    let tester = ProviderTester::new(
        provider,
        name.to_string(),
        extension_manager,
        is_cli_provider,
        model_switch_name.map(String::from),
    );
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
    test_provider(
        "openai",
        OPEN_AI_DEFAULT_MODEL,
        None,
        &["OPENAI_API_KEY"],
        None,
        false,
    )
    .await
}

#[tokio::test]
async fn test_azure_provider() -> Result<()> {
    test_provider(
        "Azure",
        AZURE_DEFAULT_MODEL,
        None,
        &[
            "AZURE_OPENAI_API_KEY",
            "AZURE_OPENAI_ENDPOINT",
            "AZURE_OPENAI_DEPLOYMENT_NAME",
        ],
        None,
        false,
    )
    .await
}

#[tokio::test]
async fn test_bedrock_provider_long_term_credentials() -> Result<()> {
    test_provider(
        "aws_bedrock",
        BEDROCK_DEFAULT_MODEL,
        None,
        &["AWS_ACCESS_KEY_ID", "AWS_SECRET_ACCESS_KEY"],
        None,
        false,
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
        None,
        &["AWS_PROFILE"],
        Some(env_mods),
        false,
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
        None,
        &["AWS_BEARER_TOKEN_BEDROCK", "AWS_REGION"],
        Some(env_mods),
        false,
    )
    .await
}

#[tokio::test]
async fn test_databricks_provider() -> Result<()> {
    test_provider(
        "Databricks",
        DATABRICKS_DEFAULT_MODEL,
        None,
        &["DATABRICKS_HOST", "DATABRICKS_TOKEN"],
        None,
        false,
    )
    .await
}

#[tokio::test]
async fn test_ollama_provider() -> Result<()> {
    // qwen3-vl supports text, tools, and vision (needed for image test)
    test_provider(
        "Ollama",
        "qwen3-vl",
        Some("qwen3"),
        &["OLLAMA_HOST"],
        None,
        false,
    )
    .await
}

#[tokio::test]
async fn test_anthropic_provider() -> Result<()> {
    test_provider(
        "Anthropic",
        ANTHROPIC_DEFAULT_MODEL,
        None,
        &["ANTHROPIC_API_KEY"],
        None,
        false,
    )
    .await
}

#[tokio::test]
async fn test_openrouter_provider() -> Result<()> {
    test_provider(
        "OpenRouter",
        OPEN_AI_DEFAULT_MODEL,
        None,
        &["OPENROUTER_API_KEY"],
        None,
        false,
    )
    .await
}

#[tokio::test]
async fn test_google_provider() -> Result<()> {
    test_provider(
        "Google",
        GOOGLE_DEFAULT_MODEL,
        None,
        &["GOOGLE_API_KEY"],
        None,
        false,
    )
    .await
}

#[tokio::test]
async fn test_snowflake_provider() -> Result<()> {
    test_provider(
        "Snowflake",
        SNOWFLAKE_DEFAULT_MODEL,
        None,
        &["SNOWFLAKE_HOST", "SNOWFLAKE_TOKEN"],
        None,
        false,
    )
    .await
}

#[tokio::test]
async fn test_sagemaker_tgi_provider() -> Result<()> {
    test_provider(
        "SageMakerTgi",
        SAGEMAKER_TGI_DEFAULT_MODEL,
        None,
        &["SAGEMAKER_ENDPOINT_NAME"],
        None,
        false,
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
        LITELLM_DEFAULT_MODEL,
        None,
        &[],
        Some(env_mods),
        false,
    )
    .await
}

#[tokio::test]
async fn test_xai_provider() -> Result<()> {
    test_provider(
        "Xai",
        XAI_DEFAULT_MODEL,
        None,
        &["XAI_API_KEY"],
        None,
        false,
    )
    .await
}

#[tokio::test]
async fn test_claude_code_provider() -> Result<()> {
    if which::which("claude").is_err() {
        println!("'claude' CLI not found, skipping test");
        TEST_REPORT.record_skip("claude-code");
        return Ok(());
    }
    test_provider(
        "claude-code",
        CLAUDE_CODE_DEFAULT_MODEL,
        Some("sonnet"),
        &[],
        None,
        true,
    )
    .await
}

#[tokio::test]
async fn test_codex_provider() -> Result<()> {
    if which::which("codex").is_err() {
        println!("'codex' CLI not found, skipping test");
        TEST_REPORT.record_skip("codex");
        return Ok(());
    }
    test_provider("codex", CODEX_DEFAULT_MODEL, None, &[], None, true).await
}

#[ctor::dtor]
fn print_test_report() {
    TEST_REPORT.print_summary();
}
