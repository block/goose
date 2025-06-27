use goose::agents::extension::Envs;
use goose::agents::extension::ToolInfo;
use goose::agents::ExtensionConfig;
use goose::config::permission::PermissionLevel;
use goose::config::ExtensionEntry;
use goose::message::{
    ContextLengthExceeded, FrontendToolRequest, Message, MessageContent, RedactedThinkingContent,
    SummarizationRequested, ThinkingContent, ToolConfirmationRequest, ToolRequest, ToolResponse,
};
use goose::permission::permission_confirmation::PrincipalType;
use goose::providers::base::{ConfigKey, ModelInfo, ProviderMetadata};
use goose::session::info::SessionInfo;
use goose::session::SessionMetadata;
use mcp_core::content::{Annotations, Content, EmbeddedResource, ImageContent, TextContent};
use mcp_core::handler::ToolResultSchema;
use mcp_core::resource::ResourceContents;
use mcp_core::role::Role;
use mcp_core::tool::{Tool, ToolAnnotations};
use utoipa::OpenApi;

#[allow(dead_code)] // Used by utoipa for OpenAPI generation
#[derive(OpenApi)]
#[openapi(
    paths(
        super::routes::config_management::backup_config,
        super::routes::config_management::recover_config,
        super::routes::config_management::validate_config,
        super::routes::config_management::init_config,
        super::routes::config_management::upsert_config,
        super::routes::config_management::remove_config,
        super::routes::config_management::read_config,
        super::routes::config_management::add_extension,
        super::routes::config_management::remove_extension,
        super::routes::config_management::get_extensions,
        super::routes::config_management::read_all_config,
        super::routes::config_management::providers,
        super::routes::config_management::upsert_permissions,
        super::routes::agent::get_tools,
        super::routes::reply::confirm_permission,
        super::routes::context::manage_context,
        super::routes::session::list_sessions,
        super::routes::session::get_session_history,
        super::routes::schedule::create_schedule,
        super::routes::schedule::list_schedules,
        super::routes::schedule::delete_schedule,
        super::routes::schedule::update_schedule,
        super::routes::schedule::run_now_handler,
        super::routes::schedule::pause_schedule,
        super::routes::schedule::unpause_schedule,
        super::routes::schedule::kill_running_job,
        super::routes::schedule::inspect_running_job,
        super::routes::schedule::sessions_handler
    ),
    components(schemas(
        super::routes::config_management::UpsertConfigQuery,
        super::routes::config_management::ConfigKeyQuery,
        super::routes::config_management::ConfigResponse,
        super::routes::config_management::ProvidersResponse,
        super::routes::config_management::ProviderDetails,
        super::routes::config_management::ExtensionResponse,
        super::routes::config_management::ExtensionQuery,
        super::routes::config_management::ToolPermission,
        super::routes::config_management::UpsertPermissionsQuery,
        super::routes::reply::PermissionConfirmationRequest,
        super::routes::context::ContextManageRequest,
        super::routes::context::ContextManageResponse,
        super::routes::session::SessionListResponse,
        super::routes::session::SessionHistoryResponse,
        Message,
        MessageContent,
        Content,
        EmbeddedResource,
        ImageContent,
        Annotations,
        TextContent,
        ToolResponse,
        ToolRequest,
        ToolResultSchema,
        ToolConfirmationRequest,
        ThinkingContent,
        RedactedThinkingContent,
        FrontendToolRequest,
        ResourceContents,
        ContextLengthExceeded,
        SummarizationRequested,
        Role,
        ProviderMetadata,
        ExtensionEntry,
        ExtensionConfig,
        ConfigKey,
        Envs,
        Tool,
        ToolAnnotations,
        ToolInfo,
        PermissionLevel,
        PrincipalType,
        ModelInfo,
        SessionInfo,
        SessionMetadata,
        super::routes::schedule::CreateScheduleRequest,
        super::routes::schedule::UpdateScheduleRequest,
        super::routes::schedule::KillJobResponse,
        super::routes::schedule::InspectJobResponse,
        goose::scheduler::ScheduledJob,
        super::routes::schedule::RunNowResponse,
        super::routes::schedule::ListSchedulesResponse,
        super::routes::schedule::SessionsQuery,
        super::routes::schedule::SessionDisplayInfo,
    ))
)]
pub struct ApiDoc;

#[allow(dead_code)] // Used by generate_schema binary
pub fn generate_schema() -> String {
    let api_doc = ApiDoc::openapi();
    serde_json::to_string_pretty(&api_doc).unwrap()
}
