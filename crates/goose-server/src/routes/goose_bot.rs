//! GooseBot HTTP routes - thin handlers over `goose::goose_bot`.

use std::{sync::Arc, time::Duration};

use axum::{
    extract::State,
    http::StatusCode,
    response::Json,
    routing::{get, post},
    Router,
};
use goose::config::signup_goose_bot::GooseBotInstallFlow;
use goose::config::Config;
use goose::goose_bot::{
    cached_installation_id, disconnect_install, extract_agent_id, fetch_analytics,
    fetch_oauth_client_id, fetch_repos, forward_routing_prefs, load_prefs, register_installation,
    replace_comment_reaction, report_analytics_event, resolve_install_credentials,
    run_comment_reply, run_review, save_prefs, AnalyticsEvent, GooseBotAnalytics,
    GooseBotCommentRequest, GooseBotDisconnectResponse, GooseBotPrefs, GooseBotPrefsRequest,
    GooseBotPrefsResponse, GooseBotReposResponse, GooseBotReviewRequest, GooseBotReviewResponse,
    GooseBotSetupResponse, GooseBotStatusResponse, InstallCredentials, RegisterInstallRequest,
    TunnelSnapshot, INSTALLATION_ID_CONFIG_KEY,
};
use tokio::time::sleep;

use crate::routes::errors::ErrorResponse;
use crate::state::AppState;

const TUNNEL_READY_ATTEMPTS: usize = 20;
const TUNNEL_READY_DELAY: Duration = Duration::from_millis(500);

async fn tunnel_snapshot(state: &AppState) -> TunnelSnapshot {
    let info = state.tunnel_manager.get_info().await;
    TunnelSnapshot {
        url: info.url,
        secret: info.secret,
    }
}

async fn wait_for_tunnel_ready(url: &str, secret: &str) -> Result<(), ErrorResponse> {
    let endpoint = format!("{}/goose-bot/status", url.trim_end_matches('/'));
    let client = reqwest::Client::new();
    let mut last_error = "tunnel status probe did not run".to_string();

    for attempt in 0..TUNNEL_READY_ATTEMPTS {
        match client
            .get(&endpoint)
            .header("X-Secret-Key", secret)
            .send()
            .await
        {
            Ok(res) if res.status().is_success() => return Ok(()),
            Ok(res) => {
                last_error = format!("tunnel status returned {}", res.status());
            }
            Err(e) => {
                last_error = format!("tunnel status probe failed: {e}");
            }
        }

        if attempt + 1 < TUNNEL_READY_ATTEMPTS {
            sleep(TUNNEL_READY_DELAY).await;
        }
    }

    Err(ErrorResponse::internal(format!(
        "tunnel did not become reachable: {last_error}"
    )))
}

fn app_not_installed_error(message: &str) -> bool {
    message.contains("not installed on any account")
}

#[utoipa::path(
    post,
    path = "/goose-bot/setup",
    responses(
        (status = 200, description = "Goose Bot connected", body = GooseBotSetupResponse),
        (status = 409, description = "GitHub App is not installed"),
        (status = 408, description = "Install timed out"),
        (status = 500, description = "Internal error"),
    ),
    tag = "goose_bot"
)]
#[axum::debug_handler]
async fn setup(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GooseBotSetupResponse>, ErrorResponse> {
    let oauth_client_id = fetch_oauth_client_id()
        .await
        .map_err(|e| ErrorResponse::internal(format!("oauth-config lookup failed: {e}")))?;

    let mut flow = GooseBotInstallFlow::new().with_oauth_client_id(oauth_client_id);
    let callback = flow
        .complete_flow()
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let tunnel_info = state.tunnel_manager.get_info().await;
    if tunnel_info.state != crate::tunnel::TunnelState::Running {
        state
            .tunnel_manager
            .start()
            .await
            .map_err(|e| ErrorResponse::internal(format!("tunnel start failed: {e}")))?;
    }
    let tunnel_info = state.tunnel_manager.get_info().await;
    let agent_id = extract_agent_id(&tunnel_info.url)
        .ok_or_else(|| ErrorResponse::internal("tunnel URL is missing the agent id".to_string()))?;
    let tunnel_secret = tunnel_info.secret.clone();

    wait_for_tunnel_ready(&tunnel_info.url, &tunnel_secret).await?;

    let installation_id = register_installation(RegisterInstallRequest {
        oauth_code: callback.oauth_code,
        agent_id,
        tunnel_secret: tunnel_secret.clone(),
        tunnel_url: tunnel_info.url.clone(),
    })
    .await
    .map_err(|e| {
        let message = e.to_string();
        if app_not_installed_error(&message) {
            flow.open_app_install_url();
            ErrorResponse {
                message: "Goose Bot GitHub App is not installed yet. Install it in the browser window that just opened, then retry Connect GitHub.".to_string(),
                status: StatusCode::CONFLICT,
            }
        } else {
            ErrorResponse::internal(message)
        }
    })?;

    let _ = Config::global().set_param(
        INSTALLATION_ID_CONFIG_KEY,
        serde_json::json!(installation_id),
    );
    let creds = InstallCredentials {
        installation_id,
        tunnel_secret,
    };
    let _ = forward_routing_prefs(&creds, &load_prefs().routing_subset()).await;

    Ok(Json(GooseBotSetupResponse { installation_id }))
}

#[utoipa::path(
    get,
    path = "/goose-bot/status",
    responses(
        (status = 200, description = "Cached GitHub App installation id", body = GooseBotStatusResponse),
    ),
    tag = "goose_bot"
)]
#[axum::debug_handler]
async fn get_status() -> Json<GooseBotStatusResponse> {
    Json(GooseBotStatusResponse {
        installation_id: cached_installation_id(Config::global()),
    })
}

#[utoipa::path(
    delete,
    path = "/goose-bot/setup",
    responses(
        (status = 200, description = "Local install cleared and switchboard registration removed", body = GooseBotDisconnectResponse),
        (status = 500, description = "Internal error"),
    ),
    tag = "goose_bot"
)]
#[axum::debug_handler]
async fn disconnect(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GooseBotDisconnectResponse>, ErrorResponse> {
    disconnect_install(tunnel_snapshot(&state).await)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(GooseBotDisconnectResponse { disconnected: true }))
}

#[utoipa::path(
    post,
    path = "/goose-bot/review",
    request_body = GooseBotReviewRequest,
    responses(
        (status = 200, description = "Review accepted, running in background", body = GooseBotReviewResponse),
        (status = 500, description = "Internal error"),
    ),
    tag = "goose_bot"
)]
#[axum::debug_handler]
async fn review(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GooseBotReviewRequest>,
) -> Result<Json<GooseBotReviewResponse>, ErrorResponse> {
    let pr_label = format!("{} #{}", req.repo, req.pr_number);
    let tunnel = tunnel_snapshot(&state).await;
    tokio::spawn(async move {
        let result = run_review(req.clone()).await;
        match &result {
            Ok(_) => report_analytics_event(tunnel, AnalyticsEvent::PrReviewed).await,
            Err(e) => tracing::error!("[goose_bot] review {} failed: {:#}", pr_label, e),
        }
        if let Some(id) = req.comment_id {
            let reaction = if result.is_ok() { "+1" } else { "confused" };
            let _ = replace_comment_reaction(&req.repo, id, reaction, &req.github_token).await;
        }
    });
    Ok(Json(GooseBotReviewResponse { accepted: true }))
}

#[utoipa::path(
    post,
    path = "/goose-bot/comment",
    request_body = GooseBotCommentRequest,
    responses(
        (status = 200, description = "Comment accepted, replying in background", body = GooseBotReviewResponse),
        (status = 500, description = "Internal error"),
    ),
    tag = "goose_bot"
)]
#[axum::debug_handler]
async fn comment(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GooseBotCommentRequest>,
) -> Result<Json<GooseBotReviewResponse>, ErrorResponse> {
    let pr_label = format!("{} #{}", req.repo, req.pr_number);
    let tunnel = tunnel_snapshot(&state).await;
    tokio::spawn(async move {
        let result = run_comment_reply(req.clone()).await;
        match &result {
            Ok(commit_pushed) => {
                if !req.is_pr {
                    report_analytics_event(tunnel.clone(), AnalyticsEvent::IssueHandled).await;
                }
                if *commit_pushed {
                    report_analytics_event(tunnel, AnalyticsEvent::CommitPushed).await;
                }
            }
            Err(e) => tracing::error!("[goose_bot] comment {} failed: {:#}", pr_label, e),
        }
        if let Some(id) = req.comment_id {
            let reaction = if result.is_ok() { "+1" } else { "confused" };
            let _ = replace_comment_reaction(&req.repo, id, reaction, &req.github_token).await;
        }
    });
    Ok(Json(GooseBotReviewResponse { accepted: true }))
}

#[utoipa::path(
    get,
    path = "/goose-bot/prefs",
    responses(
        (status = 200, description = "Current Goose Bot preferences", body = GooseBotPrefs),
        (status = 500, description = "Internal error"),
    ),
    tag = "goose_bot"
)]
#[axum::debug_handler]
async fn get_prefs() -> Result<Json<GooseBotPrefs>, ErrorResponse> {
    Ok(Json(load_prefs()))
}

#[utoipa::path(
    put,
    path = "/goose-bot/prefs",
    request_body = GooseBotPrefsRequest,
    responses(
        (status = 200, description = "Preferences saved", body = GooseBotPrefsResponse),
        (status = 400, description = "Validation error"),
        (status = 500, description = "Internal error"),
    ),
    tag = "goose_bot"
)]
#[axum::debug_handler]
async fn put_prefs(
    State(state): State<Arc<AppState>>,
    Json(req): Json<GooseBotPrefsRequest>,
) -> Result<Json<GooseBotPrefsResponse>, ErrorResponse> {
    req.prefs
        .validate()
        .map_err(|e| ErrorResponse::bad_request(e.to_string()))?;

    save_prefs(&req.prefs).map_err(|e| ErrorResponse::internal(e.to_string()))?;

    let (switchboard_synced, switchboard_error) =
        match resolve_install_credentials(tunnel_snapshot(&state).await).await {
            Ok(creds) => match forward_routing_prefs(&creds, &req.prefs.routing_subset()).await {
                Ok(()) => (true, None),
                Err(e) => (false, Some(e.to_string())),
            },
            Err(e) => (false, Some(e.to_string())),
        };

    Ok(Json(GooseBotPrefsResponse {
        prefs: req.prefs,
        switchboard_synced,
        switchboard_error,
    }))
}

#[utoipa::path(
    get,
    path = "/goose-bot/repos",
    responses(
        (status = 200, description = "Repos accessible to the installation", body = GooseBotReposResponse),
        (status = 412, description = "Setup not completed"),
        (status = 502, description = "Switchboard / GitHub error"),
    ),
    tag = "goose_bot"
)]
#[axum::debug_handler]
async fn get_repos(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GooseBotReposResponse>, ErrorResponse> {
    let creds = resolve_install_credentials(tunnel_snapshot(&state).await)
        .await
        .map_err(|e| ErrorResponse {
            message: e.to_string(),
            status: axum::http::StatusCode::PRECONDITION_FAILED,
        })?;

    let body = fetch_repos(&creds)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(body))
}

#[utoipa::path(
    get,
    path = "/goose-bot/analytics",
    responses(
        (status = 200, description = "Per-install analytics rollups", body = GooseBotAnalytics),
        (status = 412, description = "Setup not completed"),
    ),
    tag = "goose_bot"
)]
#[axum::debug_handler]
async fn get_analytics(
    State(state): State<Arc<AppState>>,
) -> Result<Json<GooseBotAnalytics>, ErrorResponse> {
    let creds = resolve_install_credentials(tunnel_snapshot(&state).await)
        .await
        .map_err(|e| ErrorResponse {
            message: e.to_string(),
            status: axum::http::StatusCode::PRECONDITION_FAILED,
        })?;
    let body = fetch_analytics(&creds)
        .await
        .map_err(|e| ErrorResponse::internal(e.to_string()))?;
    Ok(Json(body))
}

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/goose-bot/review", post(review))
        .route("/goose-bot/setup", post(setup).delete(disconnect))
        .route("/goose-bot/status", get(get_status))
        .route("/goose-bot/comment", post(comment))
        .route("/goose-bot/prefs", get(get_prefs).put(put_prefs))
        .route("/goose-bot/repos", get(get_repos))
        .route("/goose-bot/analytics", get(get_analytics))
        .with_state(state)
}
