pub use goose::agents::extension::ToolInfo;
pub use goose_server::routes::agent::{
    AddExtensionRequest, RemoveExtensionRequest, ResumeAgentRequest, StartAgentRequest,
    UpdateProviderRequest,
};
pub use goose_server::routes::config_management::{
    ConfigResponse, ExtensionQuery, ExtensionResponse, ProviderDetails, UpsertConfigQuery,
};
pub use goose_server::routes::reply::ChatRequest;
pub use goose_server::routes::session::SessionListResponse;
