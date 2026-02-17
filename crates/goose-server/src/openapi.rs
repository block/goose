use goose::agents::extension::Envs;
use goose::agents::extension::ToolInfo;
use goose::agents::ExtensionConfig;
use goose::config::permission::PermissionLevel;
use goose::config::ExtensionEntry;
use goose::conversation::Conversation;
use goose::dictation::download_manager::{DownloadProgress, DownloadStatus};
use goose::model::ModelConfig;
use goose::permission::permission_confirmation::{Permission, PrincipalType};
use goose::providers::base::{ConfigKey, ModelInfo, ProviderMetadata, ProviderType};
use goose::session::{Session, SessionInsights, SessionType, SystemInfo};
use rmcp::model::{
    Annotations, Content, EmbeddedResource, Icon, ImageContent, JsonObject, Prompt, PromptArgument,
    RawAudioContent, RawEmbeddedResource, RawImageContent, RawResource, RawTextContent,
    ResourceContents, Role, TaskSupport, TextContent, Tool, ToolAnnotations, ToolExecution,
};
use utoipa::{OpenApi, ToSchema};

use goose::config::declarative_providers::{
    DeclarativeProviderConfig, LoadedProvider, ProviderEngine,
};
use goose::conversation::message::{
    ActionRequired, ActionRequiredData, FrontendToolRequest, Message, MessageContent,
    MessageMetadata, RedactedThinkingContent, RoutingInfo, SystemNotificationContent,
    SystemNotificationType, ThinkingContent, TokenState, ToolConfirmationRequest, ToolRequest,
    ToolResponse,
};

use crate::routes::recipe_utils::RecipeManifest;
use crate::routes::reply::MessageEvent;
use utoipa::openapi::schema::{
    AdditionalProperties, AnyOfBuilder, ArrayBuilder, ObjectBuilder, OneOfBuilder, Schema,
    SchemaFormat, SchemaType,
};
use utoipa::openapi::{AllOfBuilder, Ref, RefOr};

macro_rules! derive_utoipa {
    ($inner_type:ident as $schema_name:ident) => {
        struct $schema_name {}

        impl<'__s> ToSchema<'__s> for $schema_name {
            fn schema() -> (&'__s str, utoipa::openapi::RefOr<utoipa::openapi::Schema>) {
                let settings = rmcp::schemars::generate::SchemaSettings::openapi3();
                let generator = settings.into_generator();
                let schema = generator.into_root_schema_for::<$inner_type>();
                let schema = convert_schemars_to_utoipa(schema);
                (stringify!($inner_type), schema)
            }

            fn aliases() -> Vec<(&'__s str, utoipa::openapi::schema::Schema)> {
                Vec::new()
            }
        }
    };
}

fn convert_schemars_to_utoipa(schema: rmcp::schemars::Schema) -> RefOr<Schema> {
    if let Some(true) = schema.as_bool() {
        return RefOr::T(Schema::Object(ObjectBuilder::new().build()));
    }

    if let Some(false) = schema.as_bool() {
        return RefOr::T(Schema::Object(ObjectBuilder::new().build()));
    }

    if let Some(obj) = schema.as_object() {
        return convert_json_object_to_utoipa(obj);
    }

    RefOr::T(Schema::Object(ObjectBuilder::new().build()))
}

fn convert_json_object_to_utoipa(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> RefOr<Schema> {
    use serde_json::Value;

    if let Some(Value::String(reference)) = obj.get("$ref") {
        return RefOr::Ref(Ref::new(reference.clone()));
    }

    if let Some(Value::Array(one_of)) = obj.get("oneOf") {
        let mut builder = OneOfBuilder::new();
        for item in one_of {
            if let Ok(schema) = rmcp::schemars::Schema::try_from(item.clone()) {
                builder = builder.item(convert_schemars_to_utoipa(schema));
            }
        }
        return RefOr::T(Schema::OneOf(builder.build()));
    }

    if let Some(Value::Array(all_of)) = obj.get("allOf") {
        let mut builder = AllOfBuilder::new();
        for item in all_of {
            if let Ok(schema) = rmcp::schemars::Schema::try_from(item.clone()) {
                builder = builder.item(convert_schemars_to_utoipa(schema));
            }
        }
        return RefOr::T(Schema::AllOf(builder.build()));
    }

    if let Some(Value::Array(any_of)) = obj.get("anyOf") {
        let mut builder = AnyOfBuilder::new();
        for item in any_of {
            if let Ok(schema) = rmcp::schemars::Schema::try_from(item.clone()) {
                builder = builder.item(convert_schemars_to_utoipa(schema));
            }
        }
        return RefOr::T(Schema::AnyOf(builder.build()));
    }

    match obj.get("type") {
        Some(Value::String(type_str)) => convert_typed_schema(type_str, obj),
        Some(Value::Array(types)) => {
            let mut builder = AnyOfBuilder::new();
            for type_val in types {
                if let Value::String(type_str) = type_val {
                    builder = builder.item(convert_typed_schema(type_str, obj));
                }
            }
            RefOr::T(Schema::AnyOf(builder.build()))
        }
        None => RefOr::T(Schema::Object(ObjectBuilder::new().build())),
        _ => RefOr::T(Schema::Object(ObjectBuilder::new().build())),
    }
}

fn convert_typed_schema(
    type_str: &str,
    obj: &serde_json::Map<String, serde_json::Value>,
) -> RefOr<Schema> {
    use serde_json::Value;

    match type_str {
        "object" => {
            let mut object_builder = ObjectBuilder::new();

            if let Some(Value::Object(properties)) = obj.get("properties") {
                for (name, prop_value) in properties {
                    if let Ok(prop_schema) = rmcp::schemars::Schema::try_from(prop_value.clone()) {
                        let prop = convert_schemars_to_utoipa(prop_schema);
                        object_builder = object_builder.property(name, prop);
                    }
                }
            }

            if let Some(Value::Array(required)) = obj.get("required") {
                for req in required {
                    if let Value::String(field_name) = req {
                        object_builder = object_builder.required(field_name);
                    }
                }
            }

            if let Some(additional) = obj.get("additionalProperties") {
                match additional {
                    Value::Bool(false) => {
                        object_builder = object_builder
                            .additional_properties(Some(AdditionalProperties::FreeForm(false)));
                    }
                    Value::Bool(true) => {
                        object_builder = object_builder
                            .additional_properties(Some(AdditionalProperties::FreeForm(true)));
                    }
                    _ => {
                        if let Ok(schema) = rmcp::schemars::Schema::try_from(additional.clone()) {
                            let schema = convert_schemars_to_utoipa(schema);
                            object_builder = object_builder
                                .additional_properties(Some(AdditionalProperties::RefOr(schema)));
                        }
                    }
                }
            }

            RefOr::T(Schema::Object(object_builder.build()))
        }
        "array" => {
            let mut array_builder = ArrayBuilder::new();

            if let Some(items) = obj.get("items") {
                match items {
                    Value::Object(_) | Value::Bool(_) => {
                        if let Ok(item_schema) = rmcp::schemars::Schema::try_from(items.clone()) {
                            let item_schema = convert_schemars_to_utoipa(item_schema);
                            array_builder = array_builder.items(item_schema);
                        }
                    }
                    Value::Array(item_schemas) => {
                        let mut any_of = AnyOfBuilder::new();
                        for item in item_schemas {
                            if let Ok(schema) = rmcp::schemars::Schema::try_from(item.clone()) {
                                any_of = any_of.item(convert_schemars_to_utoipa(schema));
                            }
                        }
                        let any_of_schema = RefOr::T(Schema::AnyOf(any_of.build()));
                        array_builder = array_builder.items(any_of_schema);
                    }
                    _ => {}
                }
            }

            if let Some(Value::Number(min_items)) = obj.get("minItems") {
                if let Some(min) = min_items.as_u64() {
                    array_builder = array_builder.min_items(Some(min as usize));
                }
            }
            if let Some(Value::Number(max_items)) = obj.get("maxItems") {
                if let Some(max) = max_items.as_u64() {
                    array_builder = array_builder.max_items(Some(max as usize));
                }
            }

            RefOr::T(Schema::Array(array_builder.build()))
        }
        "string" => {
            let mut object_builder = ObjectBuilder::new().schema_type(SchemaType::String);

            if let Some(Value::Number(min_length)) = obj.get("minLength") {
                if let Some(min) = min_length.as_u64() {
                    object_builder = object_builder.min_length(Some(min as usize));
                }
            }
            if let Some(Value::Number(max_length)) = obj.get("maxLength") {
                if let Some(max) = max_length.as_u64() {
                    object_builder = object_builder.max_length(Some(max as usize));
                }
            }
            if let Some(Value::String(pattern)) = obj.get("pattern") {
                object_builder = object_builder.pattern(Some(pattern.clone()));
            }
            if let Some(Value::String(format)) = obj.get("format") {
                object_builder = object_builder.format(Some(SchemaFormat::Custom(format.clone())));
            }

            RefOr::T(Schema::Object(object_builder.build()))
        }
        "number" => {
            let mut object_builder = ObjectBuilder::new().schema_type(SchemaType::Number);

            if let Some(Value::Number(minimum)) = obj.get("minimum") {
                if let Some(min) = minimum.as_f64() {
                    object_builder = object_builder.minimum(Some(min));
                }
            }
            if let Some(Value::Number(maximum)) = obj.get("maximum") {
                if let Some(max) = maximum.as_f64() {
                    object_builder = object_builder.maximum(Some(max));
                }
            }
            if let Some(Value::Number(exclusive_minimum)) = obj.get("exclusiveMinimum") {
                if let Some(min) = exclusive_minimum.as_f64() {
                    object_builder = object_builder.exclusive_minimum(Some(min));
                }
            }
            if let Some(Value::Number(exclusive_maximum)) = obj.get("exclusiveMaximum") {
                if let Some(max) = exclusive_maximum.as_f64() {
                    object_builder = object_builder.exclusive_maximum(Some(max));
                }
            }
            if let Some(Value::Number(multiple_of)) = obj.get("multipleOf") {
                if let Some(mult) = multiple_of.as_f64() {
                    object_builder = object_builder.multiple_of(Some(mult));
                }
            }

            RefOr::T(Schema::Object(object_builder.build()))
        }
        "integer" => {
            let mut object_builder = ObjectBuilder::new().schema_type(SchemaType::Integer);

            if let Some(Value::Number(minimum)) = obj.get("minimum") {
                if let Some(min) = minimum.as_f64() {
                    object_builder = object_builder.minimum(Some(min));
                }
            }
            if let Some(Value::Number(maximum)) = obj.get("maximum") {
                if let Some(max) = maximum.as_f64() {
                    object_builder = object_builder.maximum(Some(max));
                }
            }
            if let Some(Value::Number(exclusive_minimum)) = obj.get("exclusiveMinimum") {
                if let Some(min) = exclusive_minimum.as_f64() {
                    object_builder = object_builder.exclusive_minimum(Some(min));
                }
            }
            if let Some(Value::Number(exclusive_maximum)) = obj.get("exclusiveMaximum") {
                if let Some(max) = exclusive_maximum.as_f64() {
                    object_builder = object_builder.exclusive_maximum(Some(max));
                }
            }
            if let Some(Value::Number(multiple_of)) = obj.get("multipleOf") {
                if let Some(mult) = multiple_of.as_f64() {
                    object_builder = object_builder.multiple_of(Some(mult));
                }
            }

            RefOr::T(Schema::Object(object_builder.build()))
        }
        "boolean" => RefOr::T(Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::Boolean)
                .build(),
        )),
        "null" => RefOr::T(Schema::Object(
            ObjectBuilder::new().schema_type(SchemaType::String).build(),
        )),
        _ => RefOr::T(Schema::Object(ObjectBuilder::new().build())),
    }
}

derive_utoipa!(Role as RoleSchema);
derive_utoipa!(Content as ContentSchema);
derive_utoipa!(EmbeddedResource as EmbeddedResourceSchema);
derive_utoipa!(ImageContent as ImageContentSchema);
derive_utoipa!(TextContent as TextContentSchema);
derive_utoipa!(RawTextContent as RawTextContentSchema);
derive_utoipa!(RawImageContent as RawImageContentSchema);
derive_utoipa!(RawAudioContent as RawAudioContentSchema);
derive_utoipa!(RawEmbeddedResource as RawEmbeddedResourceSchema);
derive_utoipa!(RawResource as RawResourceSchema);
derive_utoipa!(Tool as ToolSchema);
derive_utoipa!(ToolAnnotations as ToolAnnotationsSchema);
derive_utoipa!(ToolExecution as ToolExecutionSchema);
derive_utoipa!(TaskSupport as TaskSupportSchema);
derive_utoipa!(Annotations as AnnotationsSchema);
derive_utoipa!(ResourceContents as ResourceContentsSchema);
derive_utoipa!(JsonObject as JsonObjectSchema);
derive_utoipa!(Icon as IconSchema);
derive_utoipa!(Prompt as PromptSchema);
derive_utoipa!(PromptArgument as PromptArgumentSchema);

#[derive(OpenApi)]
#[openapi(
    paths(
        super::routes::status::status,
        super::routes::status::system_info,
        super::routes::status::diagnostics,
        super::routes::mcp_ui_proxy::mcp_ui_proxy,
        super::routes::config_management::backup_config,
        super::routes::config_management::detect_provider,
        super::routes::config_management::recover_config,
        super::routes::config_management::validate_config,
        super::routes::config_management::init_config,
        super::routes::config_management::upsert_config,
        super::routes::config_management::remove_config,
        super::routes::config_management::read_config,
        super::routes::config_management::add_extension,
        super::routes::config_management::remove_extension,
        super::routes::config_management::get_extensions,
        super::routes::extension_routes::list_live_extensions,
        super::routes::extension_routes::get_live_extension,
        super::routes::extension_routes::disconnect_extension,
        super::routes::config_management::read_all_config,
        super::routes::config_management::providers,
        super::routes::config_management::get_provider_models,
        super::routes::config_management::get_slash_commands,
        super::routes::config_management::upsert_permissions,
        super::routes::config_management::create_custom_provider,
        super::routes::config_management::get_custom_provider,
        super::routes::config_management::update_custom_provider,
        super::routes::config_management::remove_custom_provider,
        super::routes::config_management::check_provider,
        super::routes::config_management::set_config_provider,
        super::routes::config_management::configure_provider_oauth,
        super::routes::config_management::get_pricing,
        super::routes::prompts::get_prompts,
        super::routes::prompts::get_prompt,
        super::routes::prompts::save_prompt,
        super::routes::prompts::reset_prompt,
        super::routes::agent::start_agent,
        super::routes::agent::resume_agent,
        super::routes::agent::stop_agent,
        super::routes::agent::restart_agent,
        super::routes::agent::update_working_dir,
        super::routes::agent::get_tools,
        super::routes::agent::list_extension_prompts,
        super::routes::agent::get_extension_prompt,
        super::routes::agent::read_resource,
        super::routes::agent::call_tool,
        super::routes::agent::list_apps,
        super::routes::agent::export_app,
        super::routes::agent::import_app,
        super::routes::agent::update_from_session,
        super::routes::agent::agent_add_extension,
        super::routes::agent::agent_remove_extension,
        super::routes::agent::update_agent_provider,
        super::routes::action_required::confirm_tool_action,
        super::routes::reply::reply,
        super::routes::session::list_sessions,
        super::routes::session::search_sessions,
        super::routes::session::get_session,
        super::routes::session::get_session_insights,
        super::routes::session::get_session_analytics,
        super::routes::analytics::list_datasets,
        super::routes::analytics::get_dataset,
        super::routes::analytics::create_dataset,
        super::routes::analytics::update_dataset,
        super::routes::analytics::delete_dataset,
        super::routes::analytics::run_eval,
        super::routes::analytics::list_runs,
        super::routes::analytics::get_run,
        super::routes::analytics::get_overview,
        super::routes::analytics::get_topics,
        super::routes::analytics::compare_runs,
        super::routes::analytics::get_tool_analytics,
        super::routes::analytics::get_agent_performance,
        super::routes::analytics::get_live_monitoring,
        super::routes::analytics::get_response_quality,
        super::routes::session::update_session_name,
        super::routes::session::delete_session,
        super::routes::session::export_session,
        super::routes::session::import_session,
        super::routes::session::update_session_user_recipe_values,
        super::routes::session::fork_session,
        super::routes::session::get_session_extensions,
        super::routes::session::clear_session,
        super::routes::session::add_message,
        super::routes::schedule::create_schedule,
        super::routes::schedule::list_schedules,
        super::routes::schedule::delete_schedule,
        super::routes::schedule::update_schedule,
        super::routes::schedule::run_now_handler,
        super::routes::schedule::pause_schedule,
        super::routes::schedule::unpause_schedule,
        super::routes::schedule::kill_running_job,
        super::routes::schedule::inspect_running_job,
        super::routes::schedule::sessions_handler,
        super::routes::recipe::create_recipe,
        super::routes::recipe::encode_recipe,
        super::routes::recipe::decode_recipe,
        super::routes::recipe::scan_recipe,
        super::routes::recipe::list_recipes,
        super::routes::recipe::delete_recipe,
        super::routes::recipe::schedule_recipe,
        super::routes::recipe::set_recipe_slash_command,
        super::routes::recipe::save_recipe,
        super::routes::recipe::parse_recipe,
        super::routes::recipe::recipe_to_yaml,
        super::routes::setup::start_openrouter_setup,
        super::routes::setup::start_tetrate_setup,
        super::routes::tunnel::start_tunnel,
        super::routes::tunnel::stop_tunnel,
        super::routes::tunnel::get_tunnel_status,
        super::routes::telemetry::send_telemetry_event,
        super::routes::dictation::transcribe_dictation,
        super::routes::dictation::get_dictation_config,
        super::routes::dictation::list_models,
        super::routes::dictation::download_model,
        super::routes::dictation::get_download_progress,
        super::routes::dictation::cancel_download,
        super::routes::dictation::delete_model,
        super::routes::agent_management::connect_agent,
        super::routes::agent_management::create_session,
        super::routes::agent_management::prompt_agent,
        super::routes::agent_management::set_mode,
        super::routes::agent_management::list_agents,
        super::routes::agent_management::disconnect_agent,
        super::routes::agent_management::list_builtin_agents,
        super::routes::agent_management::toggle_builtin_agent,
        super::routes::agent_management::bind_extension_to_agent,
        super::routes::agent_management::unbind_extension_from_agent,
        super::routes::agent_management::orchestrator_status,
        super::routes::agent_management::agent_catalog,
        // Observatory
        super::routes::observatory::get_dashboard,
        super::routes::observatory::get_active_agents,
        super::routes::observatory::get_health,
        // Auth Config
        super::routes::auth_config::list_oidc_providers,
        super::routes::auth_config::add_oidc_provider,
        super::routes::auth_config::remove_oidc_provider,
        super::routes::auth_config::auth_status,
        super::routes::user_auth::get_user_info,
        super::routes::user_auth::login,
        super::routes::user_auth::logout,
        super::routes::user_auth::refresh_token,
        super::routes::user_auth::oidc_auth_url,
        super::routes::user_auth::oidc_login,
        super::routes::user_auth::oidc_code_exchange,
        // ACP Discovery
        super::routes::acp_discovery::ping,
        super::routes::acp_discovery::list_agents,
        super::routes::acp_discovery::get_agent,
        super::routes::acp_discovery::get_acp_session,
        // ACP Runs
        super::routes::runs::create_run,
        super::routes::runs::get_run,
        super::routes::runs::resume_run,
        super::routes::runs::cancel_run,
        super::routes::runs::get_run_events,
        super::routes::runs::list_runs,
        // A2A Instance Management
        super::routes::a2a::list_instances,
        super::routes::a2a::spawn_instance,
        super::routes::a2a::get_instance,
        super::routes::a2a::cancel_instance,
        super::routes::a2a::get_instance_card,
        super::routes::a2a::get_instance_result,
        super::routes::a2a::stream_instance_events,
        super::routes::a2a::list_personas,
        // Pipeline Management (TODO: frontend agent WIP)
            super::routes::pipeline::list_pipelines_handler,
            super::routes::pipeline::get_pipeline_handler,
            super::routes::pipeline::save_pipeline_handler,
            super::routes::pipeline::update_pipeline_handler,
            super::routes::pipeline::delete_pipeline_handler,
            super::routes::pipeline::validate_pipeline_handler,
    ),
    components(schemas(
        super::routes::agent_management::OrchestratorStatus,
        super::routes::agent_management::OrchestratorAgentInfo,
        super::routes::config_management::UpsertConfigQuery,
        super::routes::config_management::ConfigKeyQuery,
        super::routes::config_management::DetectProviderRequest,
        super::routes::config_management::DetectProviderResponse,
        super::routes::config_management::ConfigResponse,
        super::routes::config_management::ProvidersResponse,
        super::routes::config_management::ProviderDetails,
        super::routes::config_management::SlashCommandsResponse,
        super::routes::config_management::SlashCommand,
        super::routes::config_management::CommandType,
        super::routes::config_management::ExtensionResponse,
        super::routes::config_management::ExtensionQuery,
        super::routes::config_management::ToolPermission,
        super::routes::config_management::UpsertPermissionsQuery,
        super::routes::config_management::UpdateCustomProviderRequest,
        super::routes::config_management::CheckProviderRequest,
        super::routes::config_management::SetProviderRequest,
        super::routes::config_management::PricingQuery,
        super::routes::config_management::PricingResponse,
        super::routes::config_management::PricingData,
        super::routes::prompts::PromptsListResponse,
        super::routes::prompts::PromptContentResponse,
        super::routes::prompts::SavePromptRequest,
        goose::prompt_template::Template,
        super::routes::action_required::ConfirmToolActionRequest,
        super::routes::reply::ChatRequest,
        super::routes::session::ImportSessionRequest,
        super::routes::session::SessionListResponse,
        super::routes::session::UpdateSessionNameRequest,
        super::routes::session::UpdateSessionUserRecipeValuesRequest,
        super::routes::session::UpdateSessionUserRecipeValuesResponse,
        super::routes::session::ForkRequest,
        super::routes::session::ForkResponse,
        super::routes::session::SessionExtensionsResponse,
        Message,
        MessageContent,
        MessageMetadata,
        RoutingInfo,
        TokenState,
        ContentSchema,
        EmbeddedResourceSchema,
        ImageContentSchema,
        AnnotationsSchema,
        TextContentSchema,
        RawTextContentSchema,
        RawImageContentSchema,
        RawAudioContentSchema,
        RawEmbeddedResourceSchema,
        RawResourceSchema,
        ToolResponse,
        ToolRequest,
        ToolConfirmationRequest,
        ActionRequired,
        ActionRequiredData,
        ThinkingContent,
        RedactedThinkingContent,
        FrontendToolRequest,
        ResourceContentsSchema,
        SystemNotificationType,
        SystemNotificationContent,
        MessageEvent,
        JsonObjectSchema,
        RoleSchema,
        ProviderMetadata,
        ProviderType,
        LoadedProvider,
        ProviderEngine,
        DeclarativeProviderConfig,
        ExtensionEntry,
        ExtensionConfig,
        ConfigKey,
        Envs,
        RecipeManifest,
        ToolSchema,
        ToolAnnotationsSchema,
        ToolExecutionSchema,
        TaskSupportSchema,
        PromptSchema,
        PromptArgumentSchema,
        ToolInfo,
        PermissionLevel,
        Permission,
        PrincipalType,
        ModelInfo,
        ModelConfig,
        Session,
        SessionInsights,
        goose::session::session_manager::SessionAnalytics,
        goose::session::session_manager::DailyActivity,
        goose::session::session_manager::ProviderUsage,
        goose::session::session_manager::DirectoryUsage,
        goose::session::eval_storage::EvalDataset,
        goose::session::eval_storage::EvalTestCase,
        goose::session::eval_storage::EvalDatasetSummary,
        goose::session::eval_storage::CreateDatasetRequest,
        goose::session::eval_storage::CreateTestCaseRequest,
        goose::session::eval_storage::EvalRunSummary,
        goose::session::eval_storage::EvalRunDetail,
        goose::session::eval_storage::AgentResult,
        goose::session::eval_storage::FailureDetail,
        goose::session::eval_storage::ConfusionMatrix,
        goose::session::eval_storage::EvalOverview,
        goose::session::eval_storage::AccuracyTrendPoint,
        goose::session::eval_storage::RegressionAlert,
        goose::session::eval_storage::RunEvalRequest,
        goose::session::eval_storage::TopicAnalytics,
        goose::session::eval_storage::TopicAgentDistribution,
        goose::session::eval_storage::RunComparison,
        goose::session::eval_storage::RunComparisonSide,
        goose::session::eval_storage::FixedCase,
        goose::session::eval_storage::AgentDelta,
        goose::session::eval_storage::CorrelationInsight,
        goose::session::tool_analytics::ToolAnalytics,
        goose::session::tool_analytics::ToolUsageStat,
        goose::session::tool_analytics::DailyToolActivity,
        goose::session::tool_analytics::ExtensionUsageStat,
        goose::session::tool_analytics::SessionToolSummary,
        goose::session::tool_analytics::AgentPerformanceMetrics,
        goose::session::tool_analytics::ProviderSessionStat,
        goose::session::tool_analytics::DurationStats,
        goose::session::tool_analytics::ActiveExtensionStat,
        goose::session::tool_analytics::VersionInfo,
        goose::session::tool_analytics::LiveMetrics,
        goose::session::tool_analytics::HotTool,
        goose::session::tool_analytics::RecentError,
        goose::session::tool_analytics::MinuteActivity,
        goose::session::tool_analytics::ResponseQualityMetrics,
        goose::session::tool_analytics::DailyQuality,
        goose::session::tool_analytics::ProviderQuality,
        SessionType,
        SystemInfo,
        Conversation,
        IconSchema,
        goose::session::ExtensionData,
        super::routes::schedule::CreateScheduleRequest,
        super::routes::schedule::UpdateScheduleRequest,
        super::routes::schedule::KillJobResponse,
        super::routes::schedule::InspectJobResponse,
        goose::scheduler::ScheduledJob,
        super::routes::schedule::RunNowResponse,
        super::routes::schedule::ListSchedulesResponse,
        super::routes::schedule::SessionsQuery,
        super::routes::schedule::SessionDisplayInfo,
        super::routes::recipe::CreateRecipeRequest,
        super::routes::recipe::AuthorRequest,
        super::routes::recipe::CreateRecipeResponse,
        super::routes::recipe::EncodeRecipeRequest,
        super::routes::recipe::EncodeRecipeResponse,
        super::routes::recipe::DecodeRecipeRequest,
        super::routes::recipe::DecodeRecipeResponse,
        super::routes::recipe::ScanRecipeRequest,
        super::routes::recipe::ScanRecipeResponse,
        super::routes::recipe::ListRecipeResponse,
        super::routes::recipe::ScheduleRecipeRequest,
        super::routes::recipe::SetSlashCommandRequest,
        super::routes::recipe::DeleteRecipeRequest,
        super::routes::recipe::SaveRecipeRequest,
        super::routes::recipe::SaveRecipeResponse,
        super::routes::errors::ErrorResponse,
        super::routes::recipe::ParseRecipeRequest,
        super::routes::recipe::ParseRecipeResponse,
        super::routes::recipe::RecipeToYamlRequest,
        super::routes::recipe::RecipeToYamlResponse,
        goose::recipe::Recipe,
        goose::recipe::Author,
        goose::recipe::Settings,
        goose::recipe::RecipeParameter,
        goose::recipe::RecipeParameterInputType,
        goose::recipe::RecipeParameterRequirement,
        goose::recipe::Response,
        goose::recipe::SubRecipe,
        goose::agents::types::RetryConfig,
        goose::agents::types::SuccessCheck,
        super::routes::agent::UpdateProviderRequest,
        super::routes::agent::GetToolsQuery,
        super::routes::agent::GetPromptsQuery,
        super::routes::agent::GetPromptRequest,
        super::routes::agent::GetPromptResponse,
        super::routes::agent::ReadResourceRequest,
        super::routes::agent::ReadResourceResponse,
        super::routes::agent::CallToolRequest,
        super::routes::agent::CallToolResponse,
        super::routes::agent::ListAppsRequest,
        super::routes::agent::ListAppsResponse,
        super::routes::agent::ImportAppRequest,
        super::routes::agent::ImportAppResponse,
        super::routes::agent::StartAgentRequest,
        super::routes::agent::ResumeAgentRequest,
        super::routes::agent::StopAgentRequest,
        super::routes::agent::RestartAgentRequest,
        super::routes::agent::UpdateWorkingDirRequest,
        super::routes::agent::UpdateFromSessionRequest,
        super::routes::agent::AddExtensionRequest,
        super::routes::agent::RemoveExtensionRequest,
        super::routes::agent::ResumeAgentResponse,
        super::routes::agent::RestartAgentResponse,
        goose::agents::ExtensionLoadResult,
        super::routes::setup::SetupResponse,
        super::tunnel::TunnelInfo,
        super::tunnel::TunnelState,
        super::routes::telemetry::TelemetryEventRequest,
        goose::goose_apps::GooseApp,
        goose::goose_apps::WindowProps,
        goose::goose_apps::McpAppResource,
        goose::goose_apps::CspMetadata,
        goose::goose_apps::PermissionsMetadata,
        goose::goose_apps::UiMetadata,
        goose::goose_apps::ResourceMetadata,
        super::routes::dictation::TranscribeRequest,
        super::routes::dictation::TranscribeResponse,
        goose::dictation::providers::DictationProvider,
        super::routes::dictation::DictationProviderStatus,
        super::routes::dictation::WhisperModelResponse,
        DownloadProgress,
        DownloadStatus,
        super::routes::agent_management::ConnectAgentRequest,
        super::routes::agent_management::ConnectAgentResponse,
        super::routes::agent_management::CreateSessionRequest,
        super::routes::agent_management::CreateSessionResponse,
        super::routes::agent_management::PromptAgentRequest,
        super::routes::agent_management::PromptAgentResponse,
        super::routes::agent_management::SetModeAgentRequest,
        super::routes::agent_management::AgentListResponse,
        super::routes::agent_management::BuiltinAgentInfo,
        super::routes::agent_management::BuiltinAgentMode,
        super::routes::agent_management::BuiltinAgentsResponse,
        super::routes::agent_management::ToggleAgentResponse,
        super::routes::agent_management::BindExtensionRequest,
        // Agent Catalog types
        super::routes::agent_management::CatalogAgent,
        super::routes::agent_management::CatalogAgentKind,
        super::routes::agent_management::CatalogAgentStatus,
        super::routes::agent_management::CatalogAgentMode,
        super::routes::agent_management::AgentCatalogResponse,
        // Observatory types
        super::routes::observatory::ObservatoryDashboard,
        super::routes::observatory::SystemHealth,
        super::routes::observatory::HealthStatus,
        super::routes::observatory::ActiveAgent,
        super::routes::observatory::ActiveAgentKind,
        super::routes::observatory::ActiveAgentStatus,
        super::routes::observatory::PerformanceSnapshot,
        super::routes::observatory::TopTool,
        // Auth Config types
        super::routes::auth_config::OidcProvidersResponse,
        super::routes::auth_config::OidcProviderInfo,
        super::routes::auth_config::AddOidcProviderRequest,
        super::routes::auth_config::RemoveOidcProviderRequest,
        super::routes::auth_config::AuthStatusResponse,
        // User Auth types
        super::routes::user_auth::UserInfoResponse,
        super::routes::user_auth::LoginRequest,
        super::routes::user_auth::LoginResponse,
        super::routes::user_auth::LogoutResponse,
        super::routes::user_auth::RefreshRequest,
        super::routes::user_auth::RefreshResponse,
        super::routes::user_auth::OidcLoginRequest,
        super::routes::user_auth::OidcLoginResponse,
        super::routes::user_auth::OidcAuthUrlRequest,
        super::routes::user_auth::OidcAuthUrlResponse,
        super::routes::user_auth::OidcCodeExchangeRequest,
        super::routes::user_auth::OidcCodeExchangeResponse,
        super::routes::extension_routes::LiveExtensionInfo,
        super::routes::extension_routes::LiveExtensionsResponse,
        // A2A Instance Management types
        super::routes::a2a::SpawnInstanceRequest,
        super::routes::a2a::InstanceResponse,
        super::routes::a2a::InstanceResultResponse,
        super::routes::a2a::PersonaSummary,
        // TODO: Pipeline Management types — frontend agent WIP, missing utoipa annotations
            super::routes::pipeline::SavePipelineRequest,
            super::routes::pipeline::SavePipelineResponse,
            super::routes::pipeline::ValidatePipelineResponse,
            goose::pipeline::Pipeline,
            goose::pipeline::PipelineNode,
            goose::pipeline::PipelineEdge,
            goose::pipeline::NodeKind,
            goose::pipeline::PipelineLayout,
            goose::pipeline::PipelineManifest,
            goose::pipeline::NodePosition,
            goose::pipeline::Viewport,
        // ACP types
        goose::acp_compat::AcpRun,
        goose::acp_compat::AcpRunStatus,
        goose::acp_compat::RunMode,
        goose::acp_compat::RunCreateRequest,
        goose::acp_compat::RunResumeRequest,
        goose::acp_compat::AcpMessage,
        goose::acp_compat::AcpMessagePart,
        goose::acp_compat::AcpRole,
        goose::acp_compat::AcpSession,
        goose::acp_compat::AcpError,
        goose::acp_compat::AwaitRequest,
        goose::acp_compat::AwaitResume,
        goose::acp_compat::AgentManifest,
        goose::acp_compat::AgentModeInfo,
        goose::acp_compat::AgentMetadata,
        goose::acp_compat::AgentStatus,
        goose::acp_compat::AgentDependency,
        goose::acp_compat::Person,
        goose::acp_compat::Link,
        super::routes::reply::PlanTask,
        goose::conversation::message::Message,
        super::routes::acp_discovery::AgentsListResponse,
    ))
)]
pub struct ApiDoc;

#[allow(dead_code)] // Used by generate_schema binary
pub fn generate_schema() -> String {
    let api_doc = ApiDoc::openapi();
    serde_json::to_string_pretty(&api_doc).unwrap()
}
