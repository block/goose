use super::TunnelInfo;
use anyhow::{Context, Result};
use axum::{
    body::Body,
    extract::Request,
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::task::JoinHandle;
use tracing::{error, info};

const LOCAL_TUNNEL_PORT: u16 = 3033;

fn secure_compare(a: &str, b: &str) -> bool {
    let mut hasher_a = DefaultHasher::new();
    a.hash(&mut hasher_a);
    let hash_a = hasher_a.finish();

    let mut hasher_b = DefaultHasher::new();
    b.hash(&mut hasher_b);
    let hash_b = hasher_b.finish();

    hash_a == hash_b
}

#[derive(Clone)]
struct ProxyState {
    server_port: u16,
    tunnel_secret: String,
    server_secret: String,
}

async fn proxy_handler(
    axum::extract::State(state): axum::extract::State<ProxyState>,
    headers: HeaderMap,
    req: Request,
) -> Response {
    let incoming_secret = headers
        .get("x-secret-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !secure_compare(incoming_secret, &state.tunnel_secret) {
        error!("Invalid tunnel secret in request");
        return (StatusCode::UNAUTHORIZED, "Invalid tunnel secret").into_response();
    }

    let method = req.method().clone();
    let uri = req.uri();
    let path = uri.path();
    let query = uri.query().unwrap_or("");

    let target_url = if query.is_empty() {
        format!("http://127.0.0.1:{}{}", state.server_port, path)
    } else {
        format!("http://127.0.0.1:{}{}?{}", state.server_port, path, query)
    };

    let client = reqwest::Client::new();
    let mut request_builder = client.request(method.clone(), &target_url);

    for (key, value) in headers.iter() {
        let key_str = key.as_str();
        if key_str.eq_ignore_ascii_case("x-secret-key")
            || key_str.eq_ignore_ascii_case("host")
            || key_str.eq_ignore_ascii_case("content-length")
        {
            continue;
        }
        request_builder = request_builder.header(key, value);
    }

    request_builder = request_builder.header("X-Secret-Key", state.server_secret.clone());

    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return (StatusCode::BAD_REQUEST, "Failed to read request body").into_response();
        }
    };

    if !body_bytes.is_empty() {
        request_builder = request_builder.body(body_bytes.to_vec());
    }

    match request_builder.send().await {
        Ok(response) => {
            let status = response.status();
            let response_headers = response.headers().clone();

            match response.bytes().await {
                Ok(body) => {
                    let mut builder = Response::builder().status(status);

                    for (key, value) in response_headers.iter() {
                        if !key.as_str().eq_ignore_ascii_case("content-length") {
                            builder = builder.header(key, value);
                        }
                    }

                    match builder.body(Body::from(body)) {
                        Ok(resp) => resp,
                        Err(e) => {
                            error!("Failed to build response: {}", e);
                            (
                                StatusCode::INTERNAL_SERVER_ERROR,
                                "Failed to build response",
                            )
                                .into_response()
                        }
                    }
                }
                Err(e) => {
                    error!("Failed to read response body: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        "Failed to read response body",
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            error!("Failed to proxy request: {}", e);
            (
                StatusCode::BAD_GATEWAY,
                format!("Failed to proxy request: {}", e),
            )
                .into_response()
        }
    }
}

pub async fn start(
    server_port: u16,
    tunnel_secret: String,
    server_secret: String,
    handle: Arc<RwLock<Option<JoinHandle<()>>>>,
) -> Result<TunnelInfo> {
    let addr = SocketAddr::from(([127, 0, 0, 1], LOCAL_TUNNEL_PORT));

    let state = ProxyState {
        server_port,
        tunnel_secret: tunnel_secret.clone(),
        server_secret,
    };

    let app = Router::new().fallback(any(proxy_handler)).with_state(state);

    let listener = tokio::net::TcpListener::bind(addr)
        .await
        .context(format!("Failed to bind to {}", addr))?;

    info!(
        "✓ Local tunnel listening on http://127.0.0.1:{}",
        LOCAL_TUNNEL_PORT
    );
    info!("✓ Proxying to: http://127.0.0.1:{}", server_port);

    let server_handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("Local tunnel server error: {}", e);
        }
    });

    *handle.write().await = Some(server_handle);

    Ok(TunnelInfo {
        state: super::TunnelState::Running,
        url: Some(format!("http://localhost:{}", LOCAL_TUNNEL_PORT)),
        hostname: Some("localhost".to_string()),
        secret: Some(tunnel_secret),
    })
}

pub async fn stop(handle: Arc<RwLock<Option<JoinHandle<()>>>>) {
    if let Some(h) = handle.write().await.take() {
        h.abort();
        info!("Local tunnel stopped");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires port 3033 to be available
    async fn test_local_tunnel_starts_and_stops() {
        let handle = Arc::new(RwLock::new(None));
        let tunnel_secret = "test-tunnel-secret".to_string();
        let server_secret = "test-server-secret".to_string();

        let test_server = tokio::spawn(async {
            let app = axum::Router::new()
                .route("/test", axum::routing::get(|| async { "test response" }));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            port
        });

        let server_port = test_server.await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let result = start(
            server_port,
            tunnel_secret.clone(),
            server_secret.clone(),
            handle.clone(),
        )
        .await;

        if let Err(ref e) = result {
            eprintln!("Start tunnel failed: {}", e);
        }
        assert!(result.is_ok(), "Failed to start tunnel: {:?}", result);
        let tunnel_info = result.unwrap();
        assert_eq!(tunnel_info.state, super::super::TunnelState::Running);
        assert_eq!(tunnel_info.url, Some("http://localhost:3033".to_string()));
        assert_eq!(tunnel_info.secret, Some(tunnel_secret.clone()));

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let response = client
            .get("http://127.0.0.1:3033/test")
            .header("x-secret-key", &tunnel_secret)
            .send()
            .await;

        assert!(response.is_ok());
        let text = response.unwrap().text().await.unwrap();
        assert_eq!(text, "test response");

        stop(handle.clone()).await;

        let response_after_stop = client
            .get("http://127.0.0.1:3033/test")
            .header("x-secret-key", &tunnel_secret)
            .send()
            .await;

        assert!(response_after_stop.is_err());
    }

    #[tokio::test]
    #[ignore] // Requires port 3033 to be available
    async fn test_local_tunnel_rejects_invalid_secret() {
        let handle = Arc::new(RwLock::new(None));
        let tunnel_secret = "correct-secret".to_string();
        let server_secret = "server-secret".to_string();

        let test_server = tokio::spawn(async {
            let app = axum::Router::new()
                .route("/test", axum::routing::get(|| async { "test response" }));
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let port = listener.local_addr().unwrap().port();
            tokio::spawn(async move {
                axum::serve(listener, app).await.unwrap();
            });
            port
        });

        let server_port = test_server.await.unwrap();
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let _ = start(
            server_port,
            tunnel_secret.clone(),
            server_secret.clone(),
            handle.clone(),
        )
        .await;

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let client = reqwest::Client::new();
        let response = client
            .get("http://127.0.0.1:3033/test")
            .header("x-secret-key", "wrong-secret")
            .send()
            .await;

        assert!(response.is_ok());
        let status = response.unwrap().status();
        assert_eq!(status, StatusCode::UNAUTHORIZED);

        stop(handle).await;
    }
}
