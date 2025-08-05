use std::net::SocketAddr;
use std::sync::{Arc};
use axum::extract::{Query, State};
use axum::response::Html;
use axum::Router;
use axum::routing::get;
use rmcp::transport::auth::OAuthState;
use rmcp::transport::AuthorizationManager;
use serde::Deserialize;
use tokio::sync::{oneshot, Mutex};

const CALLBACK_HTML: &str = include_str!("oauth_callback.html");
const CALLBACK_PORT: u16 = 8020;

#[derive(Clone)]
struct AppState {
    code_receiver: Arc<Mutex<Option<oneshot::Sender<String>>>>,
}

#[derive(Debug, Deserialize)]
struct CallbackParams {
    code: String,
    #[allow(dead_code)]
    state: Option<String>,
}

async fn callback_handler(
    Query(params): Query<CallbackParams>,
    State(state): State<AppState>,
) -> Html<String> {
    eprintln!("Received callback with code: {}", params.code);

    // Send the code to the main thread
    if let Some(sender) = state.code_receiver.lock().await.take() {
        let _ = sender.send(params.code);
    }
    // Return success page
    Html(CALLBACK_HTML.to_string())
}

pub async fn oauth_flow(mcp_server_url: impl Into<String>) -> Result<AuthorizationManager, anyhow::Error> {
    let (code_sender, code_receiver) = oneshot::channel::<String>();
    let app_state = AppState {
        code_receiver: Arc::new(Mutex::new(Some(code_sender)))
    };

    let app = Router::new()
        .route("/oauth_callback", get(callback_handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], CALLBACK_PORT));

    tokio::spawn(async move {
        let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
        let result = axum::serve(listener, app).await;

        if let Err(e) = result {
            eprintln!("Callback server error: {}", e);
        }
    });

    let mut oauth_state = OAuthState::new(mcp_server_url.into(), None).await?;
    oauth_state.start_authorization(&[], "http://localhost:8020/oauth_callback").await?;

    eprintln!("Open the following URL:");
    eprintln!("  {}", oauth_state.get_authorization_url().await?);

    let auth_code = code_receiver.await?;
    oauth_state.handle_callback(&auth_code).await?;
    eprintln!("Authorization successful! Access token obtained.");

    let am = oauth_state
        .into_authorization_manager()
        .ok_or_else(|| anyhow::anyhow!("Failed to get authorization manager"))?;

    Ok(am)
}