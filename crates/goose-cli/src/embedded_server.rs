use anyhow::Result;
use std::sync::Arc;

use crate::goosed_client::GoosedClient;

pub struct EmbeddedGoosed {
    pub client: GoosedClient,
    pub state: Arc<goose_server::AppState>,
}

pub async fn start_embedded_goosed() -> Result<GoosedClient> {
    let embedded = start_embedded_goosed_with_state().await?;
    Ok(embedded.client)
}

pub async fn start_embedded_goosed_with_state() -> Result<EmbeddedGoosed> {
    let state = goose_server::AppState::new().await?;
    let secret_key = generate_secret_key();

    let router = goose_server::routes::configure(state.clone(), secret_key.clone());

    let app = router.layer(
        tower_http::cors::CorsLayer::new()
            .allow_origin(tower_http::cors::Any)
            .allow_methods(tower_http::cors::Any)
            .allow_headers(tower_http::cors::Any),
    );

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();

    let app = apply_auth(app, secret_key.clone());

    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app.into_make_service()).await {
            tracing::error!("Embedded goosed server error: {}", e);
        }
    });

    let base_url = format!("http://127.0.0.1:{}", port);
    tracing::info!("Embedded goosed server started on {}", base_url);

    let client = GoosedClient::new(base_url, secret_key)?;
    Ok(EmbeddedGoosed { client, state })
}

fn generate_secret_key() -> String {
    use rand::Rng;
    let mut rng = rand::thread_rng();
    let bytes: Vec<u8> = (0..32).map(|_| rng.gen()).collect();
    hex::encode(bytes)
}

/// Apply the same secret-key auth middleware that standalone goosed uses
fn apply_auth(router: axum::Router, secret_key: String) -> axum::Router {
    use axum::extract::Request;
    use axum::middleware::{self, Next};
    use axum::response::{IntoResponse, Response};
    use http::StatusCode;

    let auth_middleware = move |req: Request, next: Next| {
        let key = secret_key.clone();
        async move {
            // Skip auth for health endpoint
            if req.uri().path() == "/status" {
                return Ok::<Response, StatusCode>(next.run(req).await);
            }

            if let Some(header) = req.headers().get("X-Secret-Key") {
                if let Ok(val) = header.to_str() {
                    if val == key {
                        return Ok(next.run(req).await);
                    }
                }
            }

            Ok(StatusCode::UNAUTHORIZED.into_response())
        }
    };

    router.layer(middleware::from_fn(auth_middleware))
}
