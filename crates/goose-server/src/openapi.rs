use goose::agents::extension::Envs;
use goose::agents::extension::ToolInfo;
use goose::agents::ExtensionConfig;
use goose::config::permission::PermissionLevel;
use goose::config::ExtensionEntry;
use goose::permission::permission_confirmation::PrincipalType;
use goose::providers::base::{ConfigKey, ModelInfo, ProviderMetadata};
use goose::session::info::SessionInfo;
use goose::session::SessionMetadata;
use rmcp::model::{
    Annotations, Content, EmbeddedResource, ImageContent, RawEmbeddedResource, RawImageContent,
    RawTextContent, ResourceContents, Role, TextContent, Tool, ToolAnnotations,
};
use utoipa::{OpenApi, ToSchema};

use goose::conversation::message::{
    ContextLengthExceeded, FrontendToolRequest, Message, MessageContent, RedactedThinkingContent,
    SummarizationRequested, ThinkingContent, ToolConfirmationRequest, ToolRequest, ToolResponse,
};
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
    // For schemars 1.0+, we need to work with the public API
    // The schema is now a wrapper around a JSON Value that can be either an object or bool
    if let Some(true) = schema.as_bool() {
        return RefOr::T(Schema::Object(ObjectBuilder::new().build()));
    }

    if let Some(false) = schema.as_bool() {
        return RefOr::T(Schema::Object(ObjectBuilder::new().build()));
    }

    // For object schemas, we'll need to work with the JSON Value directly
    if let Some(obj) = schema.as_object() {
        return convert_json_object_to_utoipa(obj);
    }

    // Fallback
    RefOr::T(Schema::Object(ObjectBuilder::new().build()))
}

fn convert_json_object_to_utoipa(
    obj: &serde_json::Map<String, serde_json::Value>,
) -> RefOr<Schema> {
    use serde_json::Value;

    // Handle $ref
    if let Some(Value::String(reference)) = obj.get("$ref") {
        return RefOr::Ref(Ref::new(reference.clone()));
    }

    // Handle oneOf, allOf, anyOf
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

    // Handle type-based schemas
    match obj.get("type") {
        Some(Value::String(type_str)) => convert_typed_schema(type_str, obj),
        Some(Value::Array(types)) => {
            // Multiple types - use AnyOf
            let mut builder = AnyOfBuilder::new();
            for type_val in types {
                if let Value::String(type_str) = type_val {
                    builder = builder.item(convert_typed_schema(type_str, obj));
                }
            }
            RefOr::T(Schema::AnyOf(builder.build()))
        }
        None => RefOr::T(Schema::Object(ObjectBuilder::new().build())),
        _ => RefOr::T(Schema::Object(ObjectBuilder::new().build())), // Handle other value types
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

            // Add properties
            if let Some(Value::Object(properties)) = obj.get("properties") {
                for (name, prop_value) in properties {
                    if let Ok(prop_schema) = rmcp::schemars::Schema::try_from(prop_value.clone()) {
                        let prop = convert_schemars_to_utoipa(prop_schema);
                        object_builder = object_builder.property(name, prop);
                    }
                }
            }

            // Add required fields
            if let Some(Value::Array(required)) = obj.get("required") {
                for req in required {
                    if let Value::String(field_name) = req {
                        object_builder = object_builder.required(field_name);
                    }
                }
            }

            // Handle additional properties
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

            // Add items schema
            if let Some(items) = obj.get("items") {
                match items {
                    Value::Object(_) | Value::Bool(_) => {
                        if let Ok(item_schema) = rmcp::schemars::Schema::try_from(items.clone()) {
                            let item_schema = convert_schemars_to_utoipa(item_schema);
                            array_builder = array_builder.items(item_schema);
                        }
                    }
                    Value::Array(item_schemas) => {
                        // Multiple item types - use AnyOf
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

            // Add constraints
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
derive_utoipa!(RawEmbeddedResource as RawEmbeddedResourceSchema);
derive_utoipa!(Tool as ToolSchema);
derive_utoipa!(ToolAnnotations as ToolAnnotationsSchema);
derive_utoipa!(Annotations as AnnotationsSchema);
derive_utoipa!(ResourceContents as ResourceContentsSchema);

// Create a manual schema for the generic Annotated type
// We manually define this to avoid circular references from RawContent::Audio(AudioContent)
// where AudioContent = Annotated<RawAudioContent>
struct AnnotatedSchema {}

impl<'__s> ToSchema<'__s> for AnnotatedSchema {
    fn schema() -> (&'__s str, utoipa::openapi::RefOr<utoipa::openapi::Schema>) {
        // Create a oneOf schema with only the variants we actually use in the API
        // This avoids the circular reference from RawContent::Audio(AudioContent)
        let schema = Schema::OneOf(
            OneOfBuilder::new()
                .item(RefOr::Ref(Ref::new("#/components/schemas/RawTextContent")))
                .item(RefOr::Ref(Ref::new("#/components/schemas/RawImageContent")))
                .item(RefOr::Ref(Ref::new(
                    "#/components/schemas/RawEmbeddedResource",
                )))
                .build(),
        );
        ("Annotated", RefOr::T(schema))
    }

    fn aliases() -> Vec<(&'__s str, utoipa::openapi::schema::Schema)> {
        Vec::new()
    }
}

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
        RawTextContentSchema,
        RawImageContentSchema,
        RawEmbeddedResourceSchema,
        AnnotatedSchema,
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
