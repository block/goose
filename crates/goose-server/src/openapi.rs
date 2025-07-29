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
use rmcp::model::{
    Annotations, Content, EmbeddedResource, ImageContent, ResourceContents, Role, TextContent,
    Tool, ToolAnnotations,
};
use utoipa::{OpenApi, ToSchema};

use rmcp::schemars::schema::{InstanceType, SchemaObject, SingleOrVec};
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
                let settings = rmcp::schemars::gen::SchemaSettings::openapi3();
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

fn convert_schemars_to_utoipa(schema: rmcp::schemars::schema::RootSchema) -> RefOr<Schema> {
    convert_schema_object(&rmcp::schemars::schema::Schema::Object(
        schema.schema.clone(),
    ))
}

fn convert_schema_object(schema: &rmcp::schemars::schema::Schema) -> RefOr<Schema> {
    match schema {
        rmcp::schemars::schema::Schema::Object(schema_object) => {
            convert_schema_object_inner(schema_object)
        }
        rmcp::schemars::schema::Schema::Bool(true) => {
            RefOr::T(Schema::Object(ObjectBuilder::new().build()))
        }
        rmcp::schemars::schema::Schema::Bool(false) => {
            RefOr::T(Schema::Object(ObjectBuilder::new().build()))
        }
    }
}

fn convert_schema_object_inner(schema: &SchemaObject) -> RefOr<Schema> {
    // Handle references first
    if let Some(reference) = &schema.reference {
        return RefOr::Ref(Ref::new(reference.clone()));
    }

    // Handle subschemas (oneOf, allOf, anyOf)
    if let Some(subschemas) = &schema.subschemas {
        if let Some(one_of) = &subschemas.one_of {
            let schemas: Vec<RefOr<Schema>> = one_of.iter().map(convert_schema_object).collect();
            let mut builder = OneOfBuilder::new();
            for schema in schemas {
                builder = builder.item(schema);
            }
            return RefOr::T(Schema::OneOf(builder.build()));
        }
        if let Some(all_of) = &subschemas.all_of {
            let schemas: Vec<RefOr<Schema>> = all_of.iter().map(convert_schema_object).collect();
            let mut all_of = AllOfBuilder::new();
            for schema in schemas {
                all_of = all_of.item(schema);
            }
            return RefOr::T(Schema::AllOf(all_of.build()));
        }
        if let Some(any_of) = &subschemas.any_of {
            let schemas: Vec<RefOr<Schema>> = any_of.iter().map(convert_schema_object).collect();
            let mut any_of = AnyOfBuilder::new();
            for schema in schemas {
                any_of = any_of.item(schema);
            }
            return RefOr::T(Schema::AnyOf(any_of.build()));
        }
    }

    // Handle based on instance type
    match &schema.instance_type {
        Some(SingleOrVec::Single(instance_type)) => {
            convert_single_instance_type(instance_type, schema)
        }
        Some(SingleOrVec::Vec(instance_types)) => {
            // Multiple types - use AnyOf
            let schemas: Vec<RefOr<Schema>> = instance_types
                .iter()
                .map(|instance_type| convert_single_instance_type(instance_type, schema))
                .collect();
            let mut any_of = AnyOfBuilder::new();
            for schema in schemas {
                any_of = any_of.item(schema);
            }
            RefOr::T(Schema::AnyOf(any_of.build()))
        }
        None => {
            // No type specified - create a generic schema
            RefOr::T(Schema::Object(ObjectBuilder::new().build()))
        }
    }
}

fn convert_single_instance_type(
    instance_type: &InstanceType,
    schema: &SchemaObject,
) -> RefOr<Schema> {
    match instance_type {
        InstanceType::Object => {
            let mut object_builder = ObjectBuilder::new();

            if let Some(object_validation) = &schema.object {
                // Add properties
                for (name, prop_schema) in &object_validation.properties {
                    let prop = convert_schema_object(prop_schema);
                    object_builder = object_builder.property(name, prop);
                }

                // Add required fields
                for required_field in &object_validation.required {
                    object_builder = object_builder.required(required_field);
                }

                // Handle additional properties
                if let Some(additional) = &object_validation.additional_properties {
                    match &**additional {
                        rmcp::schemars::schema::Schema::Bool(false) => {
                            object_builder = object_builder
                                .additional_properties(Some(AdditionalProperties::FreeForm(false)));
                        }
                        rmcp::schemars::schema::Schema::Bool(true) => {
                            object_builder = object_builder
                                .additional_properties(Some(AdditionalProperties::FreeForm(true)));
                        }
                        rmcp::schemars::schema::Schema::Object(obj) => {
                            let schema = convert_schema_object(
                                &rmcp::schemars::schema::Schema::Object(obj.clone()),
                            );
                            object_builder = object_builder
                                .additional_properties(Some(AdditionalProperties::RefOr(schema)));
                        }
                    }
                }
            }

            RefOr::T(Schema::Object(object_builder.build()))
        }
        InstanceType::Array => {
            let mut array_builder = ArrayBuilder::new();

            if let Some(array_validation) = &schema.array {
                // Add items schema
                if let Some(items) = &array_validation.items {
                    match items {
                        rmcp::schemars::schema::SingleOrVec::Single(item_schema) => {
                            let item_schema = convert_schema_object(item_schema);
                            array_builder = array_builder.items(item_schema);
                        }
                        rmcp::schemars::schema::SingleOrVec::Vec(item_schemas) => {
                            // Multiple item types - use AnyOf
                            let schemas: Vec<RefOr<Schema>> =
                                item_schemas.iter().map(convert_schema_object).collect();
                            let mut any_of = AnyOfBuilder::new();
                            for schema in schemas {
                                any_of = any_of.item(schema);
                            }
                            let any_of_schema = RefOr::T(Schema::AnyOf(any_of.build()));
                            array_builder = array_builder.items(any_of_schema);
                        }
                    }
                }

                // Add constraints
                if let Some(min_items) = array_validation.min_items {
                    array_builder = array_builder.min_items(Some(min_items as usize));
                }
                if let Some(max_items) = array_validation.max_items {
                    array_builder = array_builder.max_items(Some(max_items as usize));
                }
            }

            RefOr::T(Schema::Array(array_builder.build()))
        }
        InstanceType::String => {
            let mut object_builder = ObjectBuilder::new().schema_type(SchemaType::String);

            if let Some(string_validation) = &schema.string {
                if let Some(min_length) = string_validation.min_length {
                    object_builder = object_builder.min_length(Some(min_length as usize));
                }
                if let Some(max_length) = string_validation.max_length {
                    object_builder = object_builder.max_length(Some(max_length as usize));
                }
                if let Some(pattern) = &string_validation.pattern {
                    object_builder = object_builder.pattern(Some(pattern.clone()));
                }
            }

            if let Some(format) = &schema.format {
                object_builder = object_builder.format(Some(SchemaFormat::Custom(format.clone())));
            }

            RefOr::T(Schema::Object(object_builder.build()))
        }
        InstanceType::Number => {
            let mut object_builder = ObjectBuilder::new().schema_type(SchemaType::Number);

            if let Some(number_validation) = &schema.number {
                if let Some(minimum) = number_validation.minimum {
                    object_builder = object_builder.minimum(Some(minimum));
                }
                if let Some(maximum) = number_validation.maximum {
                    object_builder = object_builder.maximum(Some(maximum));
                }
                if let Some(exclusive_minimum) = number_validation.exclusive_minimum {
                    object_builder = object_builder.exclusive_minimum(Some(exclusive_minimum));
                }
                if let Some(exclusive_maximum) = number_validation.exclusive_maximum {
                    object_builder = object_builder.exclusive_maximum(Some(exclusive_maximum));
                }
                if let Some(multiple_of) = number_validation.multiple_of {
                    object_builder = object_builder.multiple_of(Some(multiple_of));
                }
            }

            RefOr::T(Schema::Object(object_builder.build()))
        }
        InstanceType::Integer => {
            let mut object_builder = ObjectBuilder::new().schema_type(SchemaType::Integer);

            if let Some(number_validation) = &schema.number {
                if let Some(minimum) = number_validation.minimum {
                    object_builder = object_builder.minimum(Some(minimum));
                }
                if let Some(maximum) = number_validation.maximum {
                    object_builder = object_builder.maximum(Some(maximum));
                }
                if let Some(exclusive_minimum) = number_validation.exclusive_minimum {
                    object_builder = object_builder.exclusive_minimum(Some(exclusive_minimum));
                }
                if let Some(exclusive_maximum) = number_validation.exclusive_maximum {
                    object_builder = object_builder.exclusive_maximum(Some(exclusive_maximum));
                }
                if let Some(multiple_of) = number_validation.multiple_of {
                    object_builder = object_builder.multiple_of(Some(multiple_of));
                }
            }

            RefOr::T(Schema::Object(object_builder.build()))
        }
        InstanceType::Boolean => RefOr::T(Schema::Object(
            ObjectBuilder::new()
                .schema_type(SchemaType::Boolean)
                .build(),
        )),
        InstanceType::Null => RefOr::T(Schema::Object(
            ObjectBuilder::new().schema_type(SchemaType::String).build(),
        )),
    }
}

derive_utoipa!(Role as RoleSchema);
derive_utoipa!(Content as ContentSchema);
derive_utoipa!(EmbeddedResource as EmbeddedResourceSchema);
derive_utoipa!(ImageContent as ImageContentSchema);
derive_utoipa!(TextContent as TextContentSchema);
derive_utoipa!(Tool as ToolSchema);
derive_utoipa!(ToolAnnotations as ToolAnnotationsSchema);
derive_utoipa!(Annotations as AnnotationsSchema);
derive_utoipa!(ResourceContents as ResourceContentsSchema);

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
        super::routes::agent::add_sub_recipes,
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
        super::routes::schedule::sessions_handler,
        super::routes::recipe::create_recipe,
        super::routes::recipe::encode_recipe,
        super::routes::recipe::decode_recipe
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
        ContentSchema,
        EmbeddedResourceSchema,
        ImageContentSchema,
        AnnotationsSchema,
        TextContentSchema,
        ToolResponse,
        ToolRequest,
        ToolConfirmationRequest,
        ThinkingContent,
        RedactedThinkingContent,
        FrontendToolRequest,
        ResourceContentsSchema,
        ContextLengthExceeded,
        SummarizationRequested,
        RoleSchema,
        ProviderMetadata,
        ExtensionEntry,
        ExtensionConfig,
        ConfigKey,
        Envs,
        ToolSchema,
        ToolAnnotationsSchema,
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
        super::routes::recipe::CreateRecipeRequest,
        super::routes::recipe::AuthorRequest,
        super::routes::recipe::CreateRecipeResponse,
        super::routes::recipe::EncodeRecipeRequest,
        super::routes::recipe::EncodeRecipeResponse,
        super::routes::recipe::DecodeRecipeRequest,
        super::routes::recipe::DecodeRecipeResponse,
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
        super::routes::agent::AddSubRecipesRequest,
        super::routes::agent::AddSubRecipesResponse,
    ))
)]
pub struct ApiDoc;

#[allow(dead_code)] // Used by generate_schema binary
pub fn generate_schema() -> String {
    let api_doc = ApiDoc::openapi();
    serde_json::to_string_pretty(&api_doc).unwrap()
}
