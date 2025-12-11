use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::extract::Query;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use goose::agents::ExtensionManager;
use goose::conversation::message::{Message, MessageContent};
use goose::goose_apps::{GooseApp, GooseAppsManager};
use goose::providers::create_with_named_model;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Instant;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use utoipa::ToSchema;

#[derive(Deserialize, utoipa::IntoParams, ToSchema)]
pub struct ListAppsRequest {
    session_id: Option<String>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppListResponse {
    pub apps: Vec<GooseApp>,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct AppResponse {
    pub app: GooseApp,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateAppRequest {
    pub app: GooseApp,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateAppRequest {
    pub app: GooseApp,
}

#[derive(Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IterateAppRequest {
    pub prd: String,
    pub html: String,
    pub screenshot_base64: Option<String>,
    pub errors: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IterateAppResponse {
    pub html: Option<String>,
    pub message: String,
    pub done: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub message: String,
}

fn format_resource_name(name: String) -> String {
    name.replace('_', " ")
        .split_whitespace()
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                None => String::new(),
                Some(first) => first.to_uppercase().chain(chars).collect(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

async fn list_mcp_apps(
    extension_manager: &ExtensionManager,
) -> Result<Vec<GooseApp>, ErrorResponse> {
    let mut apps = Vec::new();

    let ui_resources = extension_manager
        .get_ui_resources()
        .await
        .map_err(|err| ErrorResponse {
            message: format!("Failed to create session: {}", err),
            status: StatusCode::INTERNAL_SERVER_ERROR,
        })?;

    for (extension_name, resource) in ui_resources {
        match extension_manager
            .read_ui_resource(&resource.uri, &extension_name, CancellationToken::default())
            .await
        {
            Ok(html) => {
                apps.push(GooseApp {
                    name: format_resource_name(resource.name.clone()),
                    description: resource.description.clone(),
                    html,
                    width: None,
                    height: None,
                    resizable: Some(true),
                    prd: String::new(),
                    mcp_server: Some(extension_name),
                });
            }
            Err(e) => {
                warn!(
                    "Failed to read resource {} from {}: {}",
                    resource.uri, extension_name, e
                );
            }
        }
    }

    Ok(apps)
}

#[utoipa::path(
    get,
    path = "/apps/list_apps",
    params(
        ListAppsRequest
    ),
    responses(
        (status = 200, description = "List of installed apps retrieved successfully", body = AppListResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn list_apps(
    State(state): State<Arc<AppState>>,
    Query(params): Query<ListAppsRequest>,
) -> Result<Json<AppListResponse>, ErrorResponse> {
    let manager = GooseAppsManager::new()?;
    let mut apps = manager.list_apps()?;
    if let Some(session_id) = params.session_id {
        if let Ok(agent) = state.get_agent_for_route(session_id).await {
            let mcp_apps = list_mcp_apps(&agent.extension_manager).await?;
            apps.extend(mcp_apps);
        }
    }
    Ok(Json(AppListResponse { apps }))
}

#[utoipa::path(
    get,
    path = "/apps/app/{name}",
    responses(
        (status = 200, description = "App retrieved successfully", body = AppResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 404, description = "App not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("name" = String, Path, description = "Name of the app")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn get_app(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<AppResponse>, ErrorResponse> {
    let manager = GooseAppsManager::new()?;
    let app = manager.get_app(&name)?;

    match app {
        Some(app) => Ok(Json(AppResponse { app })),
        None => Err(ErrorResponse::internal("Unknown App")),
    }
}

#[utoipa::path(
    post,
    path = "/apps",
    request_body = CreateAppRequest,
    responses(
        (status = 201, description = "App created successfully", body = SuccessResponse),
        (status = 400, description = "Bad request - Invalid app data", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 409, description = "Conflict - App already exists", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn create_app(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<CreateAppRequest>,
) -> Result<(StatusCode, Json<SuccessResponse>), ErrorResponse> {
    let manager = GooseAppsManager::new()?;

    let app = if request.app.name.is_empty() {
        manager.get_clock().map_err(|err| {
            error!("Failed to create session: {}", err);
            ErrorResponse {
                message: format!("Failed to create session: {}", err),
                status: StatusCode::BAD_REQUEST,
            }
        })?
    } else {
        request.app
    };

    if manager.app_exists(&app.name) {
        return Err(ErrorResponse::internal(format!(
            "App '{}' already exists",
            app.name
        )));
    }

    manager.update_app(&app)?;

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse {
            message: format!("App '{}' created successfully", app.name),
        }),
    ))
}

#[utoipa::path(
    put,
    path = "/apps/app/{name}",
    request_body = UpdateAppRequest,
    responses(
        (status = 200, description = "App updated successfully", body = SuccessResponse),
        (status = 400, description = "Bad request - Invalid app data", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 404, description = "App not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("name" = String, Path, description = "Name of the app to update")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn store_app(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
    Json(request): Json<UpdateAppRequest>,
) -> Result<Json<SuccessResponse>, ErrorResponse> {
    let manager = GooseAppsManager::new()?;
    manager.update_app(&request.app)?;

    Ok(Json(SuccessResponse {
        message: format!("App '{}' updated successfully", name),
    }))
}

const ITERATE_APP_PROMPT: &str = r#"You are building a widget for a desktop in pure HTML

The entire file is one HTML file and needs to render inside the dimensions of
{width} pixels wide and {height} pixels high. You cannot access any resources outside
the html, so inline all js and css.

{html}

Here is the specification of what the user wants the app to do:
{prd}

{errors}
{screenshot_instruction}

Reply with:

description: <describe 

"#;

fn iterate_app_prompt(iterate_on: &IterateAppRequest) -> String {
    let errors = if iterate_on.errors.is_empty() {
        String::new()
    } else {
        format!(
            "\n\nThe current implementation throws errors: {}\nFix these errors.",
            iterate_on.errors
        )
    };

    let (screenshot_instruction, screenshot_note) = if iterate_on.screenshot_base64.is_some() {
        (

            "You are also provided a screenshot. Compare the current implementation and the screenshot with the specification/PRD.",
            "Note: if you change the HTML, you will be called back with the next render, so you don't have to get it right in one iteration. For complicated things, use multiple turns.\n\nIn the message, describe exactly what you see on the screenshot, then explain the changes you made or need to make."
        )
    } else {
        ("", "")
    };

    let html = if iterate_on.html.is_empty() {
        String::new()
    } else {
        format!("The current implementation looks like this:\n````html\n{}\n```", iterate_on.html)
    };

    ITERATE_APP_PROMPT
        .replace("{prd}", &iterate_on.prd)
        .replace("{html}", &html)
        .replace("{errors}", &errors)
        .replace("{screenshot_instruction}", screenshot_instruction)
        .replace("{screenshot_note}", screenshot_note)
        .replace("{width}", &iterate_on.width.to_string())
        .replace("{height}", &iterate_on.height.to_string())
}

fn extract_code_and_message(text: &str) -> (Option<String>, String) {
    let mut recording = false;
    let mut code_lines = Vec::new();
    let mut message_lines = Vec::new();

    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            recording = !recording;
        } else if recording {
            code_lines.push(line);
        } else if line.trim_start().starts_with("MSG:") {
            if let Some(msg_content) = line.trim_start().strip_prefix("MSG:") {
                message_lines.push(msg_content.trim());
            }
        }
    }

    let code = if code_lines.is_empty() {
        None
    } else {
        Some(code_lines.join("\n"))
    };

    let message = message_lines.join(" ");

    (code, message)
}

#[utoipa::path(
    post,
    path = "/apps/iterate",
    request_body = IterateAppRequest,
    responses(
        (status = 200, description = "App iterated successfully", body = IterateAppResponse),
        (status = 400, description = "Bad request - Invalid app data", body = ErrorResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn iterate_app(
    State(_state): State<Arc<AppState>>,
    Json(request): Json<IterateAppRequest>,
) -> Result<Json<IterateAppResponse>, ErrorResponse> {
    let prompt = iterate_app_prompt(&request);

    let config = goose::config::Config::global();
    let provider_name: String = config.get_goose_provider()?;
    let model_name: String = config.get_goose_model()?;
    let provider = create_with_named_model(&provider_name, &model_name).await?;

    let message = if let Some(ref screenshot) = request.screenshot_base64 {
        Message::user()
            .with_text(prompt)
            .with_image(screenshot, "image/png".to_string())
    } else {
        Message::user().with_text(prompt)
    };

    let completion_start = Instant::now();
    let (response, _) = provider
        .complete("You are a helpful coding assistant.", &[message], &[])
        .await
        .map_err(|e| ErrorResponse::internal(format!("Provider error: {}", e)))?;
    info!("Provider completion: {:?}", completion_start.elapsed());

    let text_content = response
        .content
        .iter()
        .find_map(|c| {
            if let MessageContent::Text(text) = c {
                Some(text.text.as_str())
            } else {
                None
            }
        })
        .unwrap_or("");

    let (code, message) = extract_code_and_message(text_content);

    Ok(Json(IterateAppResponse {
        done: code.is_none(),
        html: code,
        message,
    }))
}

#[utoipa::path(
    delete,
    path = "/apps/app/{name}",
    responses(
        (status = 200, description = "App deleted successfully", body = SuccessResponse),
        (status = 401, description = "Unauthorized - Invalid or missing API key", body = ErrorResponse),
        (status = 404, description = "App not found", body = ErrorResponse),
        (status = 500, description = "Internal server error", body = ErrorResponse),
    ),
    params(
        ("name" = String, Path, description = "Name of the app to delete")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn delete_app(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<Json<SuccessResponse>, ErrorResponse> {
    let manager = GooseAppsManager::new()?;
    manager.delete_app(&name)?;

    Ok(Json(SuccessResponse {
        message: format!("App '{}' deleted successfully", name),
    }))
}

#[utoipa::path(
    get,
    path = "/apps/export/{name}",
    responses(
        (status = 200, description = "App HTML exported successfully"),
        (status = 404, description = "App not found", body = ErrorResponse),
    ),
    params(
        ("name" = String, Path, description = "Name of the app to export")
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn export_app(
    State(_state): State<Arc<AppState>>,
    Path(name): Path<String>,
) -> Result<String, ErrorResponse> {
    let manager = GooseAppsManager::new()?;
    let app = manager.get_app(&name)?;

    match app {
        Some(app) => app
            .to_file_content()
            .map_err(|e| ErrorResponse::internal(format!("Failed to generate HTML: {}", e))),
        None => Err(ErrorResponse::internal("App not found")),
    }
}

#[utoipa::path(
    post,
    path = "/apps/import",
    request_body = String,
    responses(
        (status = 201, description = "App imported successfully", body = SuccessResponse),
        (status = 400, description = "Bad request - Invalid HTML", body = ErrorResponse),
    ),
    security(
        ("api_key" = [])
    ),
    tag = "App Management"
)]
async fn import_app(
    State(_state): State<Arc<AppState>>,
    body: String,
) -> Result<(StatusCode, Json<SuccessResponse>), ErrorResponse> {
    let manager = GooseAppsManager::new()?;

    let mut app = GooseApp::from_html(&body)
        .map_err(|e| ErrorResponse::internal(format!("Invalid Goose App HTML: {}", e)))?;

    let original_name = app.name.clone();
    let mut counter = 1;
    while manager.app_exists(&app.name) {
        app.name = format!("{}_{}", original_name, counter);
        counter += 1;
    }

    manager.update_app(&app)?;

    Ok((
        StatusCode::CREATED,
        Json(SuccessResponse {
            message: format!("App '{}' imported successfully", app.name),
        }),
    ))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/apps", post(create_app))
        .route("/apps/list_apps", get(list_apps))
        .route("/apps/iterate", post(iterate_app))
        .route("/apps/app/{name}", put(store_app))
        .route("/apps/app/{name}", delete(delete_app))
        .route("/apps/app/{name}", get(get_app))
        .route("/apps/import", post(import_app))
        .route("/apps/export/{name}", get(export_app))
        .with_state(state)
}
