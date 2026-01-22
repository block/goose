use assert_json_diff::{assert_json_matches_no_panic, CompareMode, Config};
use goose::session_context::SESSION_ID_HEADER;
use rmcp::model::{ClientNotification, ClientRequest, Meta, ServerResult};
use rmcp::service::{NotificationContext, RequestContext, ServiceRole};
use rmcp::transport::streamable_http_server::{
    session::local::LocalSessionManager, StreamableHttpServerConfig, StreamableHttpService,
};
use rmcp::{
    handler::server::router::tool::ToolRouter, model::*, tool, tool_handler, tool_router,
    ErrorData as McpError, RoleServer, ServerHandler, Service,
};
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};
use tokio::task::JoinHandle;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

pub const FAKE_CODE: &str = "test-uuid-12345-67890";

const NOT_YET_SET: &str = "session-id-not-yet-set";

#[derive(Clone)]
pub struct ExpectedSessionId {
    value: Arc<Mutex<String>>,
    errors: Arc<Mutex<Vec<String>>>,
}

impl Default for ExpectedSessionId {
    fn default() -> Self {
        Self {
            value: Arc::new(Mutex::new(NOT_YET_SET.to_string())),
            errors: Arc::new(Mutex::new(Vec::new())),
        }
    }
}

impl ExpectedSessionId {
    pub fn set(&self, id: &sacp::schema::SessionId) {
        *self.value.lock().unwrap() = id.0.to_string();
    }

    pub fn validate(&self, actual: Option<&str>) -> Result<(), String> {
        let expected = self.value.lock().unwrap();

        let err = match actual {
            Some(act) if act == *expected => None,
            _ => Some(format!(
                "{} mismatch: expected '{}', got {:?}",
                SESSION_ID_HEADER, expected, actual
            )),
        };
        match err {
            Some(e) => {
                self.errors.lock().unwrap().push(e.clone());
                Err(e)
            }
            None => Ok(()),
        }
    }

    /// Calling this ensures incidental requests that might error asynchronously, such as
    /// session rename have coherent session IDs.
    pub fn assert_no_errors(&self) {
        let e = self.errors.lock().unwrap();
        assert!(e.is_empty(), "Session ID validation errors: {:?}", *e);
    }
}

pub struct OpenAiFixture {
    pub server: MockServer,
}

impl OpenAiFixture {
    /// Mock OpenAI streaming endpoint. Exchanges are (pattern, response) pairs.
    /// On mismatch, returns 417 of the diff in OpenAI error format.
    pub async fn new(
        exchanges: Vec<(String, &'static str)>,
        expected_session_id: ExpectedSessionId,
    ) -> Self {
        let mock_server = MockServer::start().await;
        let queue: VecDeque<(String, &'static str)> = exchanges.into_iter().collect();
        let queue = Arc::new(Mutex::new(queue));

        Mock::given(method("POST"))
            .and(path("/v1/chat/completions"))
            .respond_with({
                let queue = queue.clone();
                let expected_session_id = expected_session_id.clone();
                move |req: &wiremock::Request| {
                    let body = String::from_utf8_lossy(&req.body);

                    let actual = req
                        .headers
                        .get(SESSION_ID_HEADER)
                        .and_then(|v| v.to_str().ok());
                    if let Err(e) = expected_session_id.validate(actual) {
                        return ResponseTemplate::new(417)
                            .insert_header("content-type", "application/json")
                            .set_body_json(serde_json::json!({"error": {"message": e}}));
                    }

                    // Session rename (async, unpredictable order) - canned response
                    if body.contains("Reply with only a description in four words or less") {
                        return ResponseTemplate::new(200)
                            .insert_header("content-type", "application/json")
                            .set_body_string(include_str!(
                                "./test_data/openai_session_description.json"
                            ));
                    }

                    let (expected_body, response) = {
                        let mut q = queue.lock().unwrap();
                        q.pop_front().unwrap_or_default()
                    };

                    if body.contains(&expected_body) && !expected_body.is_empty() {
                        return ResponseTemplate::new(200)
                            .insert_header("content-type", "text/event-stream")
                            .set_body_string(response);
                    }

                    // Coerce non-json to allow a uniform JSON diff error response.
                    let exp = serde_json::from_str(&expected_body)
                        .unwrap_or(serde_json::Value::String(expected_body.clone()));
                    let act = serde_json::from_str(&body)
                        .unwrap_or(serde_json::Value::String(body.to_string()));
                    let diff =
                        assert_json_matches_no_panic(&exp, &act, Config::new(CompareMode::Strict))
                            .unwrap_err();
                    ResponseTemplate::new(417)
                        .insert_header("content-type", "application/json")
                        .set_body_json(serde_json::json!({"error": {"message": diff}}))
                }
            })
            .mount(&mock_server)
            .await;

        Self {
            server: mock_server,
        }
    }
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

trait HasMeta {
    fn meta(&self) -> &Meta;
}

impl<R: ServiceRole> HasMeta for RequestContext<R> {
    fn meta(&self) -> &Meta {
        &self.meta
    }
}

impl<R: ServiceRole> HasMeta for NotificationContext<R> {
    fn meta(&self) -> &Meta {
        &self.meta
    }
}

pub struct ValidatingService<S> {
    inner: S,
    expected_session_id: ExpectedSessionId,
}

impl<S> ValidatingService<S> {
    pub fn new(inner: S, expected_session_id: ExpectedSessionId) -> Self {
        Self {
            inner,
            expected_session_id,
        }
    }

    fn validate<C: HasMeta>(&self, context: &C) -> Result<(), McpError> {
        let actual = context
            .meta()
            .0
            .get(SESSION_ID_HEADER)
            .and_then(|v| v.as_str());
        self.expected_session_id
            .validate(actual)
            .map_err(|e| McpError::new(ErrorCode::INVALID_REQUEST, e, None))
    }
}

impl<S: Service<RoleServer>> Service<RoleServer> for ValidatingService<S> {
    async fn handle_request(
        &self,
        request: ClientRequest,
        context: RequestContext<RoleServer>,
    ) -> Result<ServerResult, McpError> {
        if !matches!(request, ClientRequest::InitializeRequest(_)) {
            self.validate(&context)?;
        }
        self.inner.handle_request(request, context).await
    }

    async fn handle_notification(
        &self,
        notification: ClientNotification,
        context: NotificationContext<RoleServer>,
    ) -> Result<(), McpError> {
        if !matches!(notification, ClientNotification::InitializedNotification(_)) {
            self.validate(&context).ok();
        }
        self.inner.handle_notification(notification, context).await
    }

    fn get_info(&self) -> ServerInfo {
        self.inner.get_info()
    }
}

pub struct McpFixture {
    pub url: String,
    // Keep the server alive in tests; underscore avoids unused field warnings.
    _handle: JoinHandle<()>,
}

impl McpFixture {
    pub async fn new(expected_session_id: ExpectedSessionId) -> Self {
        let service = StreamableHttpService::new(
            {
                let expected_session_id = expected_session_id.clone();
                move || {
                    Ok(ValidatingService::new(
                        Lookup::new(),
                        expected_session_id.clone(),
                    ))
                }
            },
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

        Self {
            url,
            _handle: handle,
        }
    }
}
