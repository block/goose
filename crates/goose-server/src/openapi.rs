use goose::agents::extension::Envs;
use goose::agents::extension::ToolInfo;
use goose::agents::ExtensionConfig;
use goose::config::permission::PermissionLevel;
use goose::config::ExtensionEntry;
use goose::message::{
    ContextLengthExceeded, FrontendToolRequest, Message, MessageContent, RedactedThinkingContent,
    ThinkingContent, ToolConfirmationRequest, ToolRequest, ToolResponse,
};
use goose::permission::permission_confirmation::PrincipalType;
use goose::providers::base::{ConfigKey, ModelInfo, ProviderMetadata};
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
        super::routes::context::manage_context, // Added this path
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
    ))
)]
pub struct ApiDoc;

#[allow(dead_code)] // Used by generate_schema binary
pub fn generate_schema() -> String {
    let api_doc = ApiDoc::openapi();
    serde_json::to_string_pretty(&api_doc).unwrap()
}
