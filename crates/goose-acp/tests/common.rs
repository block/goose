use assert_json_diff::{assert_json_matches_no_panic, CompareMode, Config};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{
    handler::server::router::tool::ToolRouter, model::*, tool, tool_handler, tool_router,
    ErrorData as McpError, ServerHandler,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const FAKE_CODE: &str = "test-uuid-12345-67890";

/// Mock OpenAI streaming endpoint. Exchanges are (pattern, response) pairs.
/// On mismatch, returns 417 of the diff in OpenAI error format.
pub async fn setup_mock_openai(exchanges: Vec<(String, &'static str)>) -> MockServer {
    let mock_server = MockServer::start().await;
    let queue: VecDeque<(String, &'static str)> = exchanges.into_iter().collect();
    let queue = Arc::new(Mutex::new(queue));

    Mock::given(method("POST"))
        .and(path("/v1/chat/completions"))
        .respond_with({
            let queue = queue.clone();
            move |req: &wiremock::Request| {
                let body = String::from_utf8_lossy(&req.body);

                // Special case session rename request which doesn't happen in a predictable order.
                if body.contains("Reply with only a description in four words or less") {
                    return ResponseTemplate::new(200)
                        .insert_header("content-type", "application/json")
                        .set_body_string(include_str!(
                            "./test_data/openai_session_description.json"
                        ));
                }

                let (expected, response) = {
                    let mut q = queue.lock().unwrap();
                    q.pop_front().unwrap_or_default()
                };

                if body.contains(&expected) && !expected.is_empty() {
                    return ResponseTemplate::new(200)
                        .insert_header("content-type", "text/event-stream")
                        .set_body_string(response);
                }

                // Coerce non-json to allow a uniform JSON diff error response.
                let exp = serde_json::from_str(&expected)
                    .unwrap_or(serde_json::Value::String(expected.clone()));
                let act = serde_json::from_str(&body)
                    .unwrap_or(serde_json::Value::String(body.to_string()));
                let diff =
                    assert_json_matches_no_panic(&exp, &act, Config::new(CompareMode::Strict))
                        .unwrap_err();
                ResponseTemplate::new(417)
                    .insert_header("content-type", "text/event-stream")
                    .set_body_json(serde_json::json!({"error": {"message": diff}}))
            }
        })
        .mount(&mock_server)
        .await;

    mock_server
}

#[derive(Clone)]
pub struct Lookup {
    tool_router: ToolRouter<Lookup>,
}

impl Default for Lookup {
    fn default() -> Self {
        Self::new()
    }
}

#[tool_router]
impl Lookup {
    pub fn new() -> Self {
        Self {
            tool_router: Self::tool_router(),
        }
    }

    #[tool(description = "Get the code")]
    fn get_code(&self) -> Result<CallToolResult, McpError> {
        Ok(CallToolResult::success(vec![Content::text(FAKE_CODE)]))
    }
}

#[tool_handler]
impl ServerHandler for Lookup {
    fn get_info(&self) -> ServerInfo {
        ServerInfo {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities::builder().enable_tools().build(),
            server_info: Implementation {
                name: "lookup".into(),
                version: "1.0.0".into(),
                ..Default::default()
            },
            instructions: Some("Lookup server with get_code tool.".into()),
        }
    }
}

pub async fn spawn_mcp_http_server() -> (String, JoinHandle<()>) {
    let service = StreamableHttpService::new(
        || Ok(Lookup::new()),
        LocalSessionManager::default().into(),
        StreamableHttpServerConfig::default(),
    );
    let router = axum::Router::new().nest_service("/mcp", service);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let url = format!("http://{addr}/mcp");

    let handle = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });

    (url, handle)
}
