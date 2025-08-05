use axum::extract::{Query, State};
use axum::response::Html;
use axum::routing::get;
use axum::Router;
use minijinja::render;
use rmcp::transport::auth::OAuthState;
use rmcp::transport::AuthorizationManager;
use serde::Deserialize;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::{oneshot, Mutex};

const CALLBACK_TEMPLATE: &str = include_str!("oauth_callback.html");

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

pub async fn oauth_flow(
    mcp_server_url: &String,
    name: &String,
) -> Result<AuthorizationManager, anyhow::Error> {
    let (code_sender, code_receiver) = oneshot::channel::<String>();
    let app_state = AppState {
        code_receiver: Arc::new(Mutex::new(Some(code_sender))),
    };

    let rendered = render!(CALLBACK_TEMPLATE, name => name);
    let handler = move |Query(params): Query<CallbackParams>, State(state): State<AppState>| {
        let rendered = rendered.clone();
        async move {
            if let Some(sender) = state.code_receiver.lock().await.take() {
                let _ = sender.send(params.code);
            }
            Html(rendered)
        }
    };
    let app = Router::new()
        .route("/oauth_callback", get(handler))
        .with_state(app_state);

    let addr = SocketAddr::from(([127, 0, 0, 1], 0));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let used_addr = listener.local_addr()?;
    tokio::spawn(async move {
        let result = axum::serve(listener, app).await;

        if let Err(e) = result {
            eprintln!("Callback server error: {}", e);
        }
    });

    let mut oauth_state = OAuthState::new(mcp_server_url, None).await?;
    let redirect_uri = format!("http://localhost:{}/oauth_callback", used_addr.port());
    oauth_state
        .start_authorization(&[], redirect_uri.as_str())
        .await?;

    let authorization_url = oauth_state.get_authorization_url().await?;
    if webbrowser::open(authorization_url.as_str()).is_err() {
        eprintln!("Open the following URL to authorize {}:", name);
        eprintln!("  {}", authorization_url);
    }

    let auth_code = code_receiver.await?;
    oauth_state.handle_callback(&auth_code).await?;

    let am = oauth_state
        .into_authorization_manager()
        .ok_or_else(|| anyhow::anyhow!("Failed to get authorization manager"))?;

    Ok(am)
}
