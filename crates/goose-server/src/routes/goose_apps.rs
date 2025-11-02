use crate::routes::errors::ErrorResponse;
use crate::state::AppState;
use axum::{
    extract::{Path, State},
    http::StatusCode,
    routing::{delete, get, post, put},
    Json, Router,
};
use goose::conversation::message::{Message, MessageContent};
use goose::goose_apps::{GooseApp, GooseAppsManager};
use goose::providers::create_with_named_model;
use include_dir::{include_dir, Dir};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use utoipa::ToSchema;

static GOOSE_APP_ASSETS: Dir =
    include_dir!("$CARGO_MANIFEST_DIR/../../ui/desktop/src/goose_apps/assets");

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
    pub js_implementation: String,
    pub screenshot: Vec<u8>,
    pub errors: String,
}

#[derive(Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct IterateAppResponse {
    pub js_implementation: Option<String>,
    pub message: String,
    pub done: bool,
}

#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct SuccessResponse {
    pub message: String,
}

#[utoipa::path(
    get,
    path = "/apps/list_apps",
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
async fn list_apps() -> Result<Json<AppListResponse>, ErrorResponse> {
    let manager = GooseAppsManager::new()?;

    let apps = manager.list_apps()?;

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

const CLOCK_PRD: &str = r#"
# Digital Clock Widget

## Overview
A simple clock widget that displays the current time and date.

## Core Functionality

### Time Display
- Shows current time updated every second
- Supports both 12-hour (with AM/PM) and 24-hour format
- Uses monospace font for consistent digit width and easy readability

### Date Display
- Shows full date including day of week, month, day, and year
- Displays below the time in a smaller, secondary style

### Settings
- User can toggle between 12-hour and 24-hour time format
- Format preference persists across sessions
- Default: 24-hour format

## Visual Design

### Layout
- Vertically stacked: time on top, date below
- Content centered within widget bounds
- Clear visual hierarchy (time prominent, date secondary)

### Typography
- Time: Large, bold, monospace
- Date: Medium size, lighter color
- Settings toggle: Small, subtle, changes on hover

### Interaction
- Clickable settings control to toggle time format
- Visual feedback on hover for interactive elements

## Default Dimensions
- Width: 300px
- Height: 180px
- Resizable by user

## Technical Requirements
- Updates automatically every second
- Minimal resource usage
- Properly cleans up when widget is closed
- Uses system locale for date formatting
"#;

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

    let app = if request.app.name == "" {
        let clock_js = GOOSE_APP_ASSETS
            .get_file("clock.js")
            .ok_or_else(|| ErrorResponse::internal("clock.js not found"))?
            .contents_utf8()
            .ok_or_else(|| ErrorResponse::internal("clock.js is not valid UTF-8"))?;
        GooseApp {
            name: "Clock".to_string(),
            description: Some("Example Clock app".to_string()),
            width: Some(300),
            height: Some(300),
            resizable: Some(false),
            js_implementation: clock_js.to_string(),
            prd: CLOCK_PRD.parse().unwrap(),
        }
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

    if !manager.app_exists(&name) {
        return Err(ErrorResponse::internal(format!("App '{}' not found", name)));
    }

    manager.update_app(&request.app)?;

    Ok(Json(SuccessResponse {
        message: format!("App '{}' updated successfully", name),
    }))
}

const ITERATE_APP_PROMPT: &str = r#"You're building a javascript widget according to spec.
The api you're building against looks like this:

```javascript
{goose_widget}
```

The current implementation of the widget looks like this:
````javascript
{js_implementation}
````

Here is the specification of what the user wants the widget to do:
{prd}

{errors}

You are also provided a screenshot. Compare the current implementation and the screenshot
with the specication/PRD. If you think it everything is good, i.e. the specification matches
the code and screenshot, you can just reply with:

DONE
MSG: <anything you want to tell the user>

If you think you we to adjust the javascript or think we need to start from scratch,
just return the code you want the widget to be going forward:

````javascript
// your code here
```
MSG: <the modifications you made>


Note: if you change the javascript, you will be called back with the next render, so you
don't have to get it right in one iteration. For complicated things it might be better
to do multiple turns.

"#;

fn iterate_app_prompt(iterate_on: &IterateAppRequest, goose_widget: &str) -> String {
    let errors = if iterate_on.errors.is_empty() {
        String::new()
    } else {
        format!(
            "\nthe current implementation throws js errors: {} - fix those too",
            iterate_on.errors
        )
    };
    ITERATE_APP_PROMPT
        .replace("{prd}", &iterate_on.prd)
        .replace("{js_implementation}", &iterate_on.js_implementation)
        .replace("{goose_widget}", goose_widget)
        .replace("{errors}", &errors)
}

fn extract_code_and_message(text: &str) -> (Option<String>, String) {
    let mut recording = false;
    let mut code_lines = Vec::new();
    let mut message = String::new();

    for line in text.lines() {
        if line.trim_start().starts_with("```") {
            recording = !recording;
        } else if recording {
            code_lines.push(line);
        } else if line.trim_start().starts_with("MSG:") {
            message = line
                .trim_start()
                .strip_prefix("MSG:")
                .unwrap()
                .trim()
                .to_string();
        }
    }

    let code = if code_lines.is_empty() {
        None
    } else {
        Some(code_lines.join("\n"))
    };

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
    let goose_widget = GOOSE_APP_ASSETS
        .get_file("goose_widget.js")
        .ok_or_else(|| ErrorResponse::internal("goose_widget.js not found"))?
        .contents_utf8()
        .ok_or_else(|| ErrorResponse::internal("goose_widget.js is not valid UTF-8"))?;

    let prompt = iterate_app_prompt(&request, goose_widget);

    let config = goose::config::Config::global();

    let provider_name: String = config.get_goose_provider()?;
    let model_name: String = config.get_goose_model()?;
    let provider = create_with_named_model(&provider_name, &model_name).await?;

    use base64::{engine::general_purpose::STANDARD as BASE64, Engine as _};
    let base64_image = BASE64.encode(&request.screenshot);

    let message_with_image = Message::user()
        .with_text(prompt)
        .with_image(base64_image, "image/png".to_string());

    let (response, _) = provider
        .complete(
            "You are a helpful coding assistant.",
            &[message_with_image],
            &[],
        )
        .await
        .map_err(|e| ErrorResponse::internal(format!("Provider error: {}", e)))?;

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
        js_implementation: code,
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

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/apps", post(create_app))
        .route("/apps/list_apps", get(list_apps))
        .route("/apps/iterate", post(iterate_app))
        .route("/apps/app/{name}", put(store_app))
        .route("/apps/app/{name}", delete(delete_app))
        .route("/apps/app/{name}", get(get_app))
        .with_state(state)
}
