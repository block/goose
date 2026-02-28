mod api;
pub mod config;
pub mod error;
pub(crate) mod http;
pub(crate) mod streaming;
pub mod types;

pub use config::GooseClientConfig;
pub use error::{GooseClientError, Result};
pub use rmcp::model::Content;
pub use types::events::MessageEvent;
pub use types::requests::{
    ChatRequest, ConfirmToolActionRequest, ReadResourceRequest, StartAgentRequest,
    UpdateProviderRequest,
};
pub use types::responses::{
    CallToolResponse, ForkResponse, ImportAppResponse, ListAppsResponse, ReadResourceResponse,
    RestartAgentResponse, ResumeAgentResponse, SessionExtensionsResponse, SessionListResponse,
    UpdateUserRecipeValuesResponse,
};

pub use goose::goose_apps::GooseApp;
pub use goose::session::{SessionInsights, SystemInfo};

use http::HttpClient;

/// Async HTTP client for the goose agent server.
///
/// All methods require a running `goosed` instance. Authentication is via the
/// `X-Secret-Key` header, configured through [`GooseClientConfig`].
///
/// `GooseClient` is `Clone` — the underlying `reqwest::Client` uses an `Arc`
/// internally, so clones share the same connection pool.
#[derive(Clone)]
pub struct GooseClient {
    pub(crate) http: HttpClient,
}

impl GooseClient {
    pub fn new(config: GooseClientConfig) -> Result<Self> {
        Ok(Self {
            http: HttpClient::new(&config)?,
        })
    }
}
