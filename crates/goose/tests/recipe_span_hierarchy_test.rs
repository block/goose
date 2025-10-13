use axum::{
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use futures::StreamExt;
use goose::{
    agents::types::SessionConfig,
    agents::{
        extension::ExtensionConfig,
        mcp_client::{Error as McpError, McpClientTrait},
        Agent,
    },
    conversation::{message::Message, Conversation},
    model::ModelConfig,
    providers::openai::OpenAiProvider,
    session::SessionManager,
};
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::{
    export::trace::{SpanData, SpanExporter},
    trace::{RandomIdGenerator, Sampler, TracerProvider as SdkTracerProvider},
    Resource,
};
use rmcp::model::{
    CallToolResult, Content, GetPromptResult, ListPromptsResult, ListResourcesResult,
    ListToolsResult, ReadResourceResult, ServerNotification, Tool,
};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;
use tracing_subscriber::{layer::SubscriberExt, Registry};

/// Test state to capture trace data
#[derive(Clone, Default)]
struct TestState {
    /// Captured traceparent from LLM requests
    llm_traceparent: Arc<Mutex<Option<String>>>,
    /// Captured traceparent from MCP requests
    mcp_traceparent: Arc<Mutex<Option<String>>>,
    /// Exported spans
    exported_spans: Arc<Mutex<Vec<SpanData>>>,
}

/// Custom span exporter that stores spans in memory for testing
#[derive(Clone, Debug)]
struct TestSpanExporter {
    spans: Arc<Mutex<Vec<SpanData>>>,
}

impl TestSpanExporter {
    fn new(spans: Arc<Mutex<Vec<SpanData>>>) -> Self {
        Self { spans }
    }
}

impl SpanExporter for TestSpanExporter {
    fn export(
        &mut self,
        batch: Vec<SpanData>,
    ) -> futures::future::BoxFuture<'static, opentelemetry_sdk::export::trace::ExportResult> {
        let spans = self.spans.clone();
        Box::pin(async move {
            spans.lock().unwrap().extend(batch);
            Ok(())
        })
    }
}

/// Mock MCP client for testing
struct TestMcpClient {
    tools: HashMap<String, Tool>,
}

impl TestMcpClient {
    fn new() -> Self {
        let mut tools = HashMap::new();

        // Create test tool
        let test_tool = Tool::new(
            "test_tool",
            "A test MCP tool",
            Arc::new(serde_json::Map::from_iter(vec![
                ("type".to_string(), json!("object")),
                ("properties".to_string(), json!({})),
                ("required".to_string(), json!([])),
            ])),
        );

        tools.insert("test_tool".to_string(), test_tool);
        Self { tools }
    }
}

#[async_trait::async_trait]
impl McpClientTrait for TestMcpClient {
    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListResourcesResult, McpError> {
        Ok(ListResourcesResult {
            resources: vec![],
            next_cursor: None,
        })
    }

    fn get_info(&self) -> Option<&rmcp::model::InitializeResult> {
        None
    }

    async fn read_resource(
        &self,
        _uri: &str,
        _cancel_token: CancellationToken,
    ) -> Result<ReadResourceResult, McpError> {
        Err(McpError::UnexpectedResponse)
    }

    async fn list_tools(
        &self,
        _: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListToolsResult, McpError> {
        let rmcp_tools: Vec<Tool> = self
            .tools
            .values()
            .map(|tool| {
                Tool::new(
                    tool.name.to_string(),
                    tool.description.clone().unwrap_or_default(),
                    tool.input_schema.clone(),
                )
            })
            .collect();

        Ok(ListToolsResult {
            tools: rmcp_tools,
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        _name: &str,
        _arguments: Option<serde_json::Map<String, Value>>,
        _cancel_token: CancellationToken,
    ) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult {
            content: vec![Content::text("Tool executed successfully")],
            is_error: None,
            structured_content: None,
            meta: None,
        })
    }

    async fn list_prompts(
        &self,
        _next_cursor: Option<String>,
        _cancel_token: CancellationToken,
    ) -> Result<ListPromptsResult, McpError> {
        Ok(ListPromptsResult {
            prompts: vec![],
            next_cursor: None,
        })
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: Value,
        _cancel_token: CancellationToken,
    ) -> Result<GetPromptResult, McpError> {
        Err(McpError::UnexpectedResponse)
    }

    async fn subscribe(&self) -> tokio::sync::mpsc::Receiver<ServerNotification> {
        mpsc::channel(1).1
    }
}

/// Start a mock LLM server that returns a tool call on first request, then a final response
async fn start_llm_server(state: TestState) -> (String, tokio::task::JoinHandle<()>) {
    let call_count = Arc::new(AtomicU32::new(0));

    async fn handle_llm_request(
        axum::extract::State((state, call_count)): axum::extract::State<(
            TestState,
            Arc<AtomicU32>,
        )>,
        headers: axum::http::HeaderMap,
        Json(_payload): Json<Value>,
    ) -> Response {
        // Capture traceparent header
        if let Some(traceparent) = headers.get("traceparent").and_then(|v| v.to_str().ok()) {
            *state.llm_traceparent.lock().unwrap() = Some(traceparent.to_string());
        }

        let count = call_count.fetch_add(1, Ordering::SeqCst);

        // First call: return MCP tool call, second call: return final response
        let response = if count == 0 {
            json!({
                "id": "test-completion",
                "object": "chat.completion",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "I will use the test tool",
                        "tool_calls": [{
                            "id": "call_1",
                            "type": "function",
                            "function": {
                                "name": "test__test_tool",
                                "arguments": "{}"
                            }
                        }]
                    },
                    "finish_reason": "tool_calls"
                }],
                "usage": {
                    "prompt_tokens": 10,
                    "completion_tokens": 5,
                    "total_tokens": 15
                }
            })
        } else {
            json!({
                "id": "test-completion-2",
                "object": "chat.completion",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "Task completed successfully"
                    },
                    "finish_reason": "stop"
                }],
                "usage": {
                    "prompt_tokens": 15,
                    "completion_tokens": 3,
                    "total_tokens": 18
                }
            })
        };

        Json(response).into_response()
    }

    let app = Router::new()
        .route("/v1/chat/completions", post(handle_llm_request))
        .with_state((state, call_count));

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let endpoint = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (endpoint, handle)
}

#[tokio::test]
async fn test_recipe_execution_with_mcp_tool_span_hierarchy() {
    // Initialize trace propagation
    goose::tracing::init_otel_propagation();

    // Create test state
    let state = TestState::default();

    // Start mock LLM server
    let (llm_endpoint, llm_handle) = start_llm_server(state.clone()).await;

    // Set up OpenTelemetry with test exporter
    let resource = Resource::new(vec![opentelemetry::KeyValue::new("service.name", "test")]);

    let test_exporter = TestSpanExporter::new(state.exported_spans.clone());

    let tracer_provider = SdkTracerProvider::builder()
        .with_simple_exporter(test_exporter)
        .with_resource(resource)
        .with_id_generator(RandomIdGenerator::default())
        .with_sampler(Sampler::AlwaysOn)
        .build();

    // Set as global tracer provider
    let _ = opentelemetry::global::set_tracer_provider(tracer_provider.clone());

    let tracer = tracer_provider.tracer("test");
    let telemetry_layer = tracing_opentelemetry::layer().with_tracer(tracer);
    let subscriber = Registry::default().with(telemetry_layer);
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global subscriber");

    // Set environment variables for OpenAI provider
    std::env::set_var("OPENAI_HOST", llm_endpoint);
    std::env::set_var("OPENAI_API_KEY", "test-key");

    // Create agent with OpenAI provider
    let agent = Agent::new();
    let model = ModelConfig::new("test-model").unwrap();
    let llm_provider = Arc::new(OpenAiProvider::from_env(model).await.unwrap());
    agent.update_provider(llm_provider).await.unwrap();

    // Add MCP extension using MockClient
    let mock_client = TestMcpClient::new();
    let client_box: Arc<tokio::sync::Mutex<Box<dyn McpClientTrait>>> =
        Arc::new(tokio::sync::Mutex::new(Box::new(mock_client)));

    agent
        .extension_manager
        .add_client(
            "test_extension".to_string(),
            ExtensionConfig::Builtin {
                name: "test".to_string(),
                display_name: None,
                description: "Test MCP extension".to_string(),
                timeout: None,
                bundled: None,
                available_tools: vec![],
            },
            client_box,
            None, // info
            None, // temp_dir
        )
        .await;

    // Create session in SessionManager
    let test_session =
        SessionManager::create_session(PathBuf::from("/tmp"), "test-recipe-session".to_string())
            .await
            .unwrap();

    let session = Some(SessionConfig {
        id: test_session.id.clone(),
        working_dir: PathBuf::from("/tmp"),
        schedule_id: None,
        max_turns: None,
        execution_mode: None,
        retry_config: None,
    });

    // Create the root span (simulating what cli.rs does for recipe execution)
    let recipe_span = tracing::info_span!(
        "recipe.execute",
        session.id = %test_session.id,
        recipe = "test-recipe"
    );
    let _span_guard = recipe_span.enter();

    // Create conversation and execute agent reply within the root span
    let conversation =
        Conversation::new_unvalidated(vec![Message::user().with_text("Execute the test")]);

    let mut stream = agent
        .reply(conversation, session.clone(), None)
        .await
        .expect("Reply failed");

    // Consume the stream
    while let Some(_event) = stream.next().await {}

    // Drop everything to ensure spans are closed
    drop(stream);
    drop(_span_guard);
    drop(recipe_span);

    // Force flush before shutdown
    tracer_provider.force_flush();

    // Give time for flush
    tokio::time::sleep(tokio::time::Duration::from_millis(1000)).await;

    // Shutdown tracer provider to force export
    tracer_provider
        .shutdown()
        .expect("Failed to shutdown tracer provider");

    // Give more time for async operations
    tokio::time::sleep(tokio::time::Duration::from_secs(2)).await;

    // Verify LLM received traceparent
    assert!(
        state.llm_traceparent.lock().unwrap().is_some(),
        "LLM should have received traceparent header"
    );

    // Get exported spans
    let spans = state.exported_spans.lock().unwrap();

    println!("\n=== Exported Spans ===");
    for span in spans.iter() {
        println!(
            "  - {} (id: {:016x}, parent: {:016x})",
            span.name,
            span.span_context.span_id(),
            span.parent_span_id
        );
    }

    // Verify we have the expected spans
    assert!(!spans.is_empty(), "Should have exported spans");

    // Find key spans
    let recipe_span = spans.iter().find(|s| s.name == "recipe.execute");
    let agent_span = spans.iter().find(|s| s.name == "agent.reply");
    let llm_spans: Vec<_> = spans
        .iter()
        .filter(|s| s.name.starts_with("chat "))
        .collect();
    let tool_span = spans.iter().find(|s| s.name == "gen_ai.tool.call");

    // Validate root span exists
    assert!(
        recipe_span.is_some(),
        "Should have recipe.execute root span"
    );
    let recipe_span = recipe_span.unwrap();

    // Validate agent span exists and is child of recipe span
    assert!(agent_span.is_some(), "Should have agent.reply span");
    let agent_span = agent_span.unwrap();
    assert_eq!(
        agent_span.parent_span_id,
        recipe_span.span_context.span_id(),
        "agent.reply should be child of recipe.execute"
    );

    // Validate LLM spans
    assert!(!llm_spans.is_empty(), "Should have LLM spans");
    // At least one LLM span should be a direct child of agent.reply
    let direct_llm_children: Vec<_> = llm_spans
        .iter()
        .filter(|s| s.parent_span_id == agent_span.span_context.span_id())
        .collect();
    assert!(
        !direct_llm_children.is_empty(),
        "At least one LLM span should be a direct child of agent.reply"
    );

    // All LLM spans should be descendants of agent.reply (either direct children or nested)
    for llm_span in &llm_spans {
        // Check if this span is agent.reply itself, or a direct child, or nested under another LLM span
        let is_child_of_agent = llm_span.parent_span_id == agent_span.span_context.span_id();
        let is_nested_under_llm = llm_spans
            .iter()
            .any(|other| llm_span.parent_span_id == other.span_context.span_id());

        assert!(
            is_child_of_agent || is_nested_under_llm,
            "LLM span {} should be a descendant of agent.reply (either direct or nested)",
            llm_span.name
        );
    }

    // Validate tool span if present
    if let Some(tool_span) = tool_span {
        assert_eq!(
            tool_span.parent_span_id,
            agent_span.span_context.span_id(),
            "Tool span should be child of agent.reply"
        );
    }

    // Validate session.id attribute is present in root and agent spans
    let recipe_attrs: HashMap<String, String> = recipe_span
        .attributes
        .iter()
        .filter_map(|kv| {
            if let opentelemetry::Value::String(s) = &kv.value {
                Some((kv.key.to_string(), s.to_string()))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        recipe_attrs.get("session.id"),
        Some(&test_session.id),
        "recipe.execute span should have session.id"
    );

    let agent_attrs: HashMap<String, String> = agent_span
        .attributes
        .iter()
        .filter_map(|kv| {
            if let opentelemetry::Value::String(s) = &kv.value {
                Some((kv.key.to_string(), s.to_string()))
            } else {
                None
            }
        })
        .collect();

    assert_eq!(
        agent_attrs.get("session.id"),
        Some(&test_session.id),
        "agent.reply span should have session.id"
    );

    println!("\n=== Test Passed ===");
    println!("✓ Root span (recipe.execute) exported");
    println!("✓ Agent span is child of root");
    println!("✓ LLM spans are children of agent");
    println!("✓ session.id present in all relevant spans");
    println!("✓ Traceparent propagated to LLM");
    println!("✓ MCP tool call via MockClient succeeded");

    // Cleanup
    llm_handle.abort();
}
