use anyhow::Result;
use axum::{
    extract::Query,
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use include_dir::{include_dir, Dir};
use minijinja::{context, Environment};
use serde::Deserialize;
use std::net::SocketAddr;
use tokio::sync::oneshot;

static TEMPLATES_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/config/signup_tetrate/templates");

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: Option<String>,
    error: Option<String>,
}

/// Run the callback server on localhost:3000
pub async fn run_callback_server(
    code_tx: oneshot::Sender<String>,
    shutdown_rx: oneshot::Receiver<()>,
) -> Result<()> {
    let app = Router::new().route("/", get(handle_callback));
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    let listener = tokio::net::TcpListener::bind(addr).await?;
    let state = std::sync::Arc::new(tokio::sync::Mutex::new(Some(code_tx)));

    axum::serve(listener, app.with_state(state.clone()).into_make_service())
        .with_graceful_shutdown(async move {
            let _ = shutdown_rx.await;
        })
        .await?;

    Ok(())
}

async fn handle_callback(
    Query(params): Query<CallbackQuery>,
    state: axum::extract::State<
        std::sync::Arc<tokio::sync::Mutex<Option<oneshot::Sender<String>>>>,
    >,
) -> impl IntoResponse {
    if let Some(error) = params.error {
        let mut env = Environment::new();
        let template_content = TEMPLATES_DIR
            .get_file("error.html")
            .expect("error.html template not found")
            .contents_utf8()
            .expect("error.html is not valid UTF-8");

        env.add_template("error", template_content).unwrap();
        let tmpl = env.get_template("error").unwrap();
        let rendered = tmpl.render(context! { error => error }).unwrap();

        return (StatusCode::BAD_REQUEST, Html(rendered));
    }

    if let Some(code) = params.code {
        let mut tx_guard = state.lock().await;
        if let Some(tx) = tx_guard.take() {
            let _ = tx.send(code);
        }

        let success_html = TEMPLATES_DIR
            .get_file("success.html")
            .expect("success.html template not found")
            .contents_utf8()
            .expect("success.html is not valid UTF-8");

        return (StatusCode::OK, Html(success_html.to_string()));
    }

    let invalid_html = TEMPLATES_DIR
        .get_file("invalid.html")
        .expect("invalid.html template not found")
        .contents_utf8()
        .expect("invalid.html is not valid UTF-8");

    (StatusCode::BAD_REQUEST, Html(invalid_html.to_string()))
}
