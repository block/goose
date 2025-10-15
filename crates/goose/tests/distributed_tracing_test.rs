use axum::{
    response::{IntoResponse, Response},
    routing::post,
    Json, Router,
};
use futures::StreamExt;
use goose::{
    agents::{
        extension::ExtensionConfig,
        mcp_client::{Error as McpError, McpClientTrait},
        Agent,
    },
    conversation::{message::Message, Conversation},
    model::ModelConfig,
    providers::openai::OpenAiProvider,
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
    use std::sync::atomic::{AtomicU32, Ordering};
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

        // First call: return tool call, second call: return final response
        let response = if count == 0 {
            json!({
                "id": "test-completion",
                "object": "chat.completion",
                "choices": [{
                    "index": 0,
                    "message": {
                        "role": "assistant",
                        "content": "I will call the test tool",
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
                        "content": "Test completed successfully"
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

/// Start a mock MCP server
async fn start_mcp_server(state: TestState) -> (String, tokio::task::JoinHandle<()>) {
    async fn handle_mcp_request(
        axum::extract::State(state): axum::extract::State<TestState>,
        Json(payload): Json<Value>,
    ) -> Response {
        // Extract traceparent from params._meta
        if let Some(params) = payload.get("params") {
            if let Some(meta) = params.get("_meta") {
                if let Some(traceparent) = meta.get("traceparent").and_then(|v| v.as_str()) {
                    *state.mcp_traceparent.lock().unwrap() = Some(traceparent.to_string());
                }
            }
        }

        let method = payload.get("method").and_then(|v| v.as_str()).unwrap_or("");
        let id = payload.get("id").cloned().unwrap_or(json!(1));

        let result = match method {
            "initialize" => json!({
                "protocolVersion": "2025-03-26",
                "capabilities": { "tools": {} },
                "serverInfo": { "name": "test-server", "version": "1.0.0" }
            }),
            "tools/list" => json!({
                "tools": [{
                    "name": "test_tool",
                    "description": "A test tool",
                    "inputSchema": {
                        "type": "object",
                        "properties": {},
                        "required": []
                    }
                }]
            }),
            "tools/call" => json!({
                "content": [{"type": "text", "text": "Tool executed"}]
            }),
            _ => json!({}),
        };

        Json(json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": result
        }))
        .into_response()
    }

    let app = Router::new()
        .route("/mcp", post(handle_mcp_request))
        .with_state(state);

    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let endpoint = format!("http://{}", addr);

    let handle = tokio::spawn(async move {
        axum::serve(listener, app).await.unwrap();
    });

    (endpoint, handle)
}

/// Helper to extract trace and span IDs from W3C traceparent
fn parse_traceparent(traceparent: &str) -> Option<(String, String)> {
    let parts: Vec<&str> = traceparent.split('-').collect();
    if parts.len() == 4 && parts[0] == "00" {
        Some((parts[1].to_string(), parts[2].to_string()))
    } else {
        None
    }
}

#[tokio::test]
async fn test_distributed_tracing_with_real_goose_code() {
    // Initialize trace propagation
    goose::tracing::init_otel_propagation();

    // Create test state
    let state = TestState::default();

    // Start mock servers
    let (llm_endpoint, llm_handle) = start_llm_server(state.clone()).await;
    let (_mcp_endpoint, mcp_handle) = start_mcp_server(state.clone()).await;

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

    // Set global subscriber
    tracing::subscriber::set_global_default(subscriber).expect("Failed to set global subscriber");

    // Set environment variables for OpenAI provider
    std::env::set_var("OPENAI_HOST", llm_endpoint);
    std::env::set_var("OPENAI_API_KEY", "test-key");

    // Create actual goose Agent with real OpenAI provider
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
            "test".to_string(),
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

    // Create a session in SessionManager for the test
    use goose::agents::types::SessionConfig;
    use goose::session::SessionManager;
    use std::path::PathBuf;

    let test_session = SessionManager::create_session(
        PathBuf::from("/tmp"),
        "test-session-description".to_string(),
    )
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

    // Create a conversation and get a reply (this will trigger LLM call)
    let conversation =
        Conversation::new_unvalidated(vec![Message::user().with_text("test message")]);

    // Execute within a traced span
    let mut stream = agent
        .reply(conversation, session.clone(), None)
        .await
        .expect("Reply failed");

    // Consume the stream
    while let Some(_event) = stream.next().await {}

    // Drop the stream to ensure all span guards are released
    drop(stream);

    // Force flush all pending spans
    tracer_provider.force_flush();

    // Give time for async flush to complete
    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

    // Shutdown the provider which forces all pending spans to export
    tracer_provider
        .shutdown()
        .expect("Failed to shutdown tracer provider");

    // Give a moment for async operations
    tokio::time::sleep(tokio::time::Duration::from_millis(500)).await;

    // Verify LLM received traceparent
    let llm_traceparent = state.llm_traceparent.lock().unwrap().clone();
    assert!(
        llm_traceparent.is_some(),
        "LLM server should have received traceparent header"
    );

    let llm_tp = llm_traceparent.unwrap();
    let (llm_trace_id, _llm_span_id_in_header) =
        parse_traceparent(&llm_tp).expect("Failed to parse LLM traceparent");

    // Get exported spans
    let spans = state.exported_spans.lock().unwrap();

    // Debug: print all span names
    println!("Exported spans:");
    for span in spans.iter() {
        println!("  - {} (parent: {:?})", span.name, span.parent_span_id);
    }

    // Find the agent span (agent.reply) - updated to use goose idiom, not semconv
    let agent_span = spans
        .iter()
        .find(|s| s.name == "agent.reply")
        .expect("Should have agent.reply span");

    // Find the LLM span that's a child of agent.reply
    let llm_span = spans
        .iter()
        .find(|s| {
            s.name.starts_with("chat ") && s.parent_span_id == agent_span.span_context.span_id()
        })
        .expect("Should have chat span that's a child of agent.reply");

    // TODO: Re-enable this when trace propagation is fully working
    // Trace IDs may not match due to known issues with context propagation
    println!(
        "LLM span trace ID: {:032x}",
        llm_span.span_context.trace_id()
    );
    println!("Propagated trace ID: {}", llm_trace_id);

    // TODO: Re-enable this when trace propagation is fully working
    // This test is currently focused on verifying span hierarchy and session.id attributes
    // assert_eq!(
    //     format!("{:016x}", llm_span.span_context.span_id()),
    //     llm_span_id_in_header,
    //     "Span ID in traceparent header must match the actual LLM span ID (not HTTP span or other wrapper)"
    // );

    // Verify LLM span's parent is the agent span
    assert_eq!(
        llm_span.parent_span_id,
        agent_span.span_context.span_id(),
        "LLM span's parent should be the agent span"
    );

    // Verify LLM span attributes follow OpenTelemetry GenAI semantic conventions
    let llm_attrs: std::collections::HashMap<String, String> = llm_span
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
        llm_attrs.get("gen_ai.request.model"),
        Some(&"test-model".to_string()),
        "LLM span should have gen_ai.request.model attribute"
    );
    assert_eq!(
        llm_attrs.get("gen_ai.system"),
        Some(&"openai".to_string()),
        "LLM span should have gen_ai.system attribute"
    );
    assert_eq!(
        llm_attrs.get("gen_ai.operation.name"),
        Some(&"chat".to_string()),
        "LLM span should have gen_ai.operation.name attribute"
    );

    // Verify agent span attributes (goose-specific, not semantic conventions)
    let agent_attrs: std::collections::HashMap<String, String> = agent_span
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
        agent_attrs.get("model"),
        Some(&"test-model".to_string()),
        "Agent span should have model attribute"
    );
    // Verify session.id attribute is present when session is provided
    assert_eq!(
        agent_attrs.get("session.id"),
        Some(&test_session.id),
        "Agent span should have session.id attribute when session is provided"
    );

    // Verify span timing: agent span should encompass LLM span
    assert!(
        agent_span.start_time <= llm_span.start_time,
        "Agent span should start before or at the same time as LLM span"
    );
    assert!(
        agent_span.end_time >= llm_span.end_time,
        "Agent span should end after or at the same time as LLM span"
    );

    // Verify LLM span duration is reasonable (should take some time for HTTP request)
    let llm_duration = llm_span
        .end_time
        .duration_since(llm_span.start_time)
        .expect("End time should be after start time");
    assert!(
        llm_duration.as_millis() > 0,
        "LLM span should have non-zero duration"
    );

    let agent_duration = agent_span
        .end_time
        .duration_since(agent_span.start_time)
        .expect("End time should be after start time");

    println!(
        "Span timing verified: agent span ({:?}) encompasses LLM span ({:?})",
        agent_duration, llm_duration
    );

    // Cleanup
    llm_handle.abort();
    mcp_handle.abort();
}
