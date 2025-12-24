use super::TunnelInfo;
use anyhow::Result;
use axum::{
    body::Body,
    extract::State,
    http::{header, Method, Request, StatusCode},
    response::{IntoResponse, Response},
    routing::any,
    Router,
};
use std::sync::Arc;
use tokio::net::TcpListener;
use tracing::{error, info};

const CGNAT_PORT: u16 = 9700;

#[derive(Clone)]
struct ProxyState {
    target_port: u16,
    target_secret: String,
    tunnel_secret: String,
}

fn secure_compare(a: &str, b: &str) -> bool {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};

    let mut hasher_a = DefaultHasher::new();
    a.hash(&mut hasher_a);
    let hash_a = hasher_a.finish();

    let mut hasher_b = DefaultHasher::new();
    b.hash(&mut hasher_b);
    let hash_b = hasher_b.finish();

    hash_a == hash_b
}

async fn proxy_handler(State(state): State<Arc<ProxyState>>, req: Request<Body>) -> Response {
    let incoming_secret = req
        .headers()
        .get("x-secret-key")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");

    if !secure_compare(incoming_secret, &state.tunnel_secret) {
        return (StatusCode::UNAUTHORIZED, "Invalid tunnel secret").into_response();
    }

    let client = reqwest::Client::new();
    let url = format!("http://127.0.0.1:{}{}", state.target_port, req.uri().path());

    let method = req.method().clone();
    let headers = req.headers().clone();

    let body_bytes = match axum::body::to_bytes(req.into_body(), usize::MAX).await {
        Ok(bytes) => bytes,
        Err(e) => {
            error!("Failed to read request body: {}", e);
            return (StatusCode::BAD_REQUEST, "Failed to read body").into_response();
        }
    };

    let mut request_builder = match method {
        Method::GET => client.get(&url),
        Method::POST => client.post(&url),
        Method::PUT => client.put(&url),
        Method::DELETE => client.delete(&url),
        Method::PATCH => client.patch(&url),
        _ => client.get(&url),
    };

    for (key, value) in headers.iter() {
        if key == header::HOST || key.as_str().eq_ignore_ascii_case("x-secret-key") {
            continue;
        }
        if let Ok(v) = value.to_str() {
            request_builder = request_builder.header(key.as_str(), v);
        }
    }

    request_builder = request_builder.header("X-Secret-Key", &state.target_secret);

    if method != Method::GET && method != Method::HEAD && !body_bytes.is_empty() {
        request_builder = request_builder.body(body_bytes.to_vec());
    }

    let response = match request_builder.send().await {
        Ok(resp) => resp,
        Err(e) => {
            error!("Proxy request failed: {}", e);
            return (StatusCode::BAD_GATEWAY, format!("Proxy error: {}", e)).into_response();
        }
    };

    let status = response.status();
    let resp_headers = response.headers().clone();
    let body = match response.bytes().await {
        Ok(b) => b,
        Err(e) => {
            error!("Failed to read response body: {}", e);
            return (StatusCode::BAD_GATEWAY, "Failed to read response").into_response();
        }
    };

    let mut response_builder = Response::builder().status(status);

    for (key, value) in resp_headers.iter() {
        if let Ok(name) = header::HeaderName::try_from(key.as_str()) {
            response_builder = response_builder.header(name, value);
        }
    }

    response_builder
        .body(Body::from(body.to_vec()))
        .unwrap_or_else(|_| {
            (StatusCode::INTERNAL_SERVER_ERROR, "Response build error").into_response()
        })
}

pub async fn start(
    target_port: u16,
    tunnel_secret: String,
    target_secret: String,
    cgnat_ip: String,
) -> Result<(TunnelInfo, tokio::task::JoinHandle<()>)> {
    let state = Arc::new(ProxyState {
        target_port,
        target_secret,
        tunnel_secret: tunnel_secret.clone(),
    });

    let app = Router::new()
        .route("/{*path}", any(proxy_handler))
        .route("/", any(proxy_handler))
        .with_state(state);

    let addr = format!("{}:{}", cgnat_ip, CGNAT_PORT);
    let listener = TcpListener::bind(&addr).await?;

    info!("CGNAT proxy listening on {}", addr);
    info!("Proxying to http://127.0.0.1:{}", target_port);

    let handle = tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, app).await {
            error!("CGNAT proxy server error: {}", e);
        }
    });

    let info = TunnelInfo {
        state: super::TunnelState::Running,
        url: format!("http://{}:{}", cgnat_ip, CGNAT_PORT),
        hostname: cgnat_ip,
        secret: tunnel_secret,
    };

    Ok((info, handle))
}
