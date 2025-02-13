use utoipa::OpenApi;

#[allow(dead_code)] // Used by utoipa for OpenAPI generation
#[derive(OpenApi)]
#[openapi(
    paths(
        super::routes::config_management::upsert_config,
        super::routes::config_management::remove_config,
        super::routes::config_management::read_config,
        super::routes::config_management::add_extension,
        super::routes::config_management::remove_extension,
        super::routes::config_management::read_all_config
    ),
    components(schemas(
        super::routes::config_management::UpsertConfigQuery,
        super::routes::config_management::ConfigKeyQuery,
        super::routes::config_management::ExtensionQuery,
        super::routes::config_management::ConfigResponse
    ))
)]
pub struct ApiDoc;

#[allow(dead_code)] // Used by generate_schema binary
pub fn generate_schema() -> String {
    let api_doc = ApiDoc::openapi();
    serde_json::to_string_pretty(&api_doc).unwrap()
}
