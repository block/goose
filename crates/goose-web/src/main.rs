use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::Stdio;

use axum::{
    body::Body,
    extract::State,
    http::{header, Request, Response, StatusCode},
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use clap::Parser;
use include_dir::{include_dir, Dir};
use tokio::process::{Child, Command};
use tower_http::cors::{Any, CorsLayer};

/// Embedded web UI static files, built by `pnpm build:web` in ui/desktop/.
/// At compile time this directory must exist; use an empty dir for dev builds.
static WEB_ASSETS: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/../../ui/desktop/dist-web");

#[derive(Parser)]
#[command(author, version, about = "Goose Web UI — serves the browser frontend and proxies API calls to goosed")]
struct Cli {
    /// Port to listen on for the web UI
    #[arg(long, default_value = "3000", env = "GOOSE_WEB_PORT")]
    port: u16,

    /// Connect to an existing goosed instance instead of spawning one
    #[arg(long, env = "GOOSE_WEB_GOOSED_URL")]
    goosed_url: Option<String>,

    /// Secret key for goosed authentication (generated automatically if spawning)
    #[arg(long, env = "GOOSE_SERVER__SECRET_KEY")]
    secret_key: Option<String>,

    /// Path to goosed binary (auto-detected if not specified)
    #[arg(long, env = "GOOSED_BINARY")]
    goosed_binary: Option<PathBuf>,
}

#[derive(Clone)]
struct AppState {
    goosed_base_url: String,
    secret_key: String,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "goose_web=info".into()),
        )
        .init();

    let cli = Cli::parse();

    let (goosed_base_url, secret_key, _child): (String, String, Option<Child>) =
        if let Some(url) = cli.goosed_url {
            let secret = cli
                .secret_key
                .expect("--secret-key is required when using --goosed-url");
            (url, secret, None)
        } else {
            let secret = cli
                .secret_key
                .unwrap_or_else(|| hex::encode(rand::random::<[u8; 32]>()));
            let (url, child) = spawn_goosed(&cli.goosed_binary, &secret).await?;
            (url, secret, Some(child))
        };

    let state = AppState {
        goosed_base_url,
        secret_key,
    };

    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    let app = Router::new()
        .route("/", get(serve_index))
        .fallback(serve_static_or_proxy)
        .layer(cors)
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], cli.port));
    tracing::info!("Goose Web UI listening on http://{}", addr);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;

    if let Some(mut child) = _child {
        tracing::info!("Stopping goosed child process");
        let _ = child.kill().await;
    }

    Ok(())
}

/// Serve index.html with injected meta tags for API config
async fn serve_index(State(_state): State<AppState>) -> impl IntoResponse {
    let index_html = WEB_ASSETS
        .get_file("index.html")
        .map(|f| f.contents_utf8().unwrap_or(""))
        .unwrap_or("<html><body>dist-web not found — run pnpm build:web first</body></html>");

    // Inject goosed connection info as meta tags before </head>.
    // API is proxied through same origin, so base is empty.
    let injected = index_html.replace(
        "</head>",
        "<meta name=\"goose:api-base\" content=\"\">\n\
         <meta name=\"goose:secret-key\" content=\"\">\n\
         </head>",
    );

    Html(injected)
}

/// Try to serve a static file; if not found, proxy to goosed API
async fn serve_static_or_proxy(
    State(state): State<AppState>,
    req: Request<Body>,
) -> impl IntoResponse {
    let path = req.uri().path().trim_start_matches('/');

    // Try static file first
    if let Some(file) = WEB_ASSETS.get_file(path) {
        let mime = mime_guess::from_path(path)
            .first_or_octet_stream()
            .to_string();
        return Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, mime)
            .header(
                header::CACHE_CONTROL,
                "public, max-age=31536000, immutable",
            )
            .body(Body::from(file.contents().to_vec()))
            .unwrap()
            .into_response();
    }

    // Not a static file — proxy to goosed
    proxy_to_goosed(state, req).await.into_response()
}

/// Forward a request to the goosed backend, injecting the secret key
async fn proxy_to_goosed(state: AppState, req: Request<Body>) -> impl IntoResponse {
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true) // goosed uses self-signed TLS
        .build()
        .unwrap();

    let method = req.method().clone();
    let uri = format!("{}{}", state.goosed_base_url, req.uri());
    let headers = req.headers().clone();

    let body_bytes = match axum::body::to_bytes(req.into_body(), 50 * 1024 * 1024).await {
        Ok(b) => b,
        Err(_) => {
            return Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .body(Body::from("Failed to read request body"))
                .unwrap()
                .into_response();
        }
    };

    let mut proxy_req = client.request(method, &uri);
    for (key, value) in headers.iter() {
        if key != header::HOST {
            proxy_req = proxy_req.header(key, value);
        }
    }
    proxy_req = proxy_req.header("X-Secret-Key", &state.secret_key);
    proxy_req = proxy_req.body(body_bytes);

    match proxy_req.send().await {
        Ok(resp) => {
            let resp: reqwest::Response = resp;
            let status = StatusCode::from_u16(resp.status().as_u16())
                .unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let resp_headers = resp.headers().clone();
            let body = resp.bytes().await.unwrap_or_default();

            let mut builder = Response::builder().status(status);
            for (key, value) in resp_headers.iter() {
                builder = builder.header(key, value);
            }
            builder.body(Body::from(body)).unwrap().into_response()
        }
        Err(e) => {
            tracing::error!("Proxy error: {}", e);
            Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from(format!("Failed to reach goosed: {}", e)))
                .unwrap()
                .into_response()
        }
    }
}

/// Find and spawn goosed, returning (base_url, child_process)
async fn spawn_goosed(
    binary_path: &Option<PathBuf>,
    secret_key: &str,
) -> anyhow::Result<(String, Child)> {
    let goosed_path = if let Some(p) = binary_path {
        p.clone()
    } else {
        find_goosed_binary()?
    };

    // Find a free port for goosed
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await?;
    let port = listener.local_addr()?.port();
    drop(listener);

    tracing::info!(
        "Spawning goosed from {} on port {}",
        goosed_path.display(),
        port
    );

    let child = Command::new(&goosed_path)
        .arg("agent")
        .env("GOOSE_PORT", port.to_string())
        .env("GOOSE_SERVER__SECRET_KEY", secret_key)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()?;

    let base_url = format!("https://127.0.0.1:{}", port);

    // Wait for goosed to become healthy
    let client = reqwest::Client::builder()
        .danger_accept_invalid_certs(true)
        .build()?;

    let status_url = format!("{}/status", base_url);
    for attempt in 1..=100 {
        tokio::time::sleep(std::time::Duration::from_millis(100)).await;
        let result: Result<reqwest::Response, _> = client.get(&status_url).send().await;
        if result.is_ok() {
            tracing::info!("goosed is healthy after {} attempts", attempt);
            return Ok((base_url, child));
        }
    }

    anyhow::bail!("goosed did not become healthy within 10 seconds");
}

fn find_goosed_binary() -> anyhow::Result<PathBuf> {
    // Check GOOSED_BINARY env
    if let Ok(p) = std::env::var("GOOSED_BINARY") {
        let path = PathBuf::from(p);
        if path.is_file() {
            return Ok(path);
        }
    }

    // Check common build paths relative to workspace root
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(|p| p.parent())
        .map(|p| p.to_path_buf())
        .unwrap_or_default();

    let binary_name = if cfg!(windows) {
        "goosed.exe"
    } else {
        "goosed"
    };

    for subdir in &["target/release", "target/debug"] {
        let candidate = workspace_root.join(subdir).join(binary_name);
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    // Try PATH
    if let Ok(path) = which::which("goosed") {
        return Ok(path);
    }

    anyhow::bail!(
        "Could not find goosed binary. Build it with `cargo build -p goose-server` \
         or set GOOSED_BINARY env var."
    );
}

#[cfg(unix)]
async fn shutdown_signal() {
    use tokio::signal::unix::{signal, SignalKind};

    let mut sigint = signal(SignalKind::interrupt()).expect("failed to install SIGINT handler");
    let mut sigterm = signal(SignalKind::terminate()).expect("failed to install SIGTERM handler");

    tokio::select! {
        _ = sigint.recv() => {},
        _ = sigterm.recv() => {},
    }
}

#[cfg(not(unix))]
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
