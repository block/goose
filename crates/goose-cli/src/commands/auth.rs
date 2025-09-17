use anyhow::{anyhow, Result};
use axum::{extract::Query, routing::get, Router};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine};
use goose::config::Config;
use serde::Deserialize;
use serde_json::Value;
use std::net::SocketAddr;
use std::process::Command;
use std::time::Duration;
use tokio::sync::oneshot;
use tokio::time::timeout;
use url::Url;

const DEFAULT_SCOPES: &str = "read:user user:email";

#[derive(Debug, Deserialize)]
struct CallbackQuery {
    code: String,
    state: String,
}

// Generate a random URL-safe string suitable for state/code_verifier in PKCE (plain in this minimal impl)
fn random_url_safe(len: usize) -> String {
    use rand::RngCore;
    let mut bytes = vec![0u8; len];
    rand::thread_rng().fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

pub async fn ensure_authenticated() -> Result<()> {
    // Allow bypass in strictly controlled environments if needed
    if std::env::var("GOOSE_AUTH_BYPASS").unwrap_or_default() == "1" {
        return Ok(());
    }

    let config = Config::global();
    match config.get_secret::<String>("GITHUB_ACCESS_TOKEN") {
        Ok(token) if !token.trim().is_empty() => Ok(()),
        _ => {
            // Auto-trigger login
            login().await
        }
    }
}

pub async fn login() -> Result<()> {
    let client_id = std::env::var("GOOSE_GITHUB_CLIENT_ID")
        .map_err(|_| anyhow!("GOOSE_GITHUB_CLIENT_ID is required for GitHub OAuth"))?;
    let redirect_url = std::env::var("GOOSE_AUTH_REDIRECT_URL")
        .map_err(|_| anyhow!("GOOSE_AUTH_REDIRECT_URL must be set to a stable HTTPS callback URL"))?;

    let scopes = std::env::var("GOOSE_GITHUB_SCOPES").unwrap_or_else(|_| DEFAULT_SCOPES.to_string());

    // PKCE (plain for minimal impl; upgrade to S256 later)
    let state = random_url_safe(24);
    let code_verifier = random_url_safe(48);
    let code_challenge = code_verifier.clone();
    let code_challenge_method = "plain"; // minimal; S256 recommended later

    let mut auth_url = Url::parse("https://github.com/login/oauth/authorize")?;
    {
        let mut qp = auth_url.query_pairs_mut();
        qp.append_pair("client_id", &client_id);
        qp.append_pair("redirect_uri", &redirect_url);
        qp.append_pair("scope", &scopes);
        qp.append_pair("state", &state);
        qp.append_pair("code_challenge", &code_challenge);
        qp.append_pair("code_challenge_method", code_challenge_method);
    }

    // Start ephemeral callback server if redirect_url points to our local listener
    // Expect users to route Cloudflare Tunnel hostname to this listener address.
    let listen_addr = std::env::var("GOOSE_AUTH_LISTEN_ADDR").unwrap_or_else(|_| "0.0.0.0:8080".to_string());
    let listen_addr: SocketAddr = listen_addr.parse()?;

    // Channel to receive code
    let (tx, rx) = oneshot::channel::<(String, String)>();
    let expected_state = state.clone();

    // Build a tiny router for /oauth_callback
    let app = {
        let tx = std::sync::Arc::new(tokio::sync::Mutex::new(Some(tx)));
        Router::new().route(
            "/oauth_callback",
            get(move |Query(q): Query<CallbackQuery>| {
                let tx = tx.clone();
                let expected_state = expected_state.clone();
                async move {
                    let body = if q.state == expected_state {
                        if let Some(sender) = tx.lock().await.take() {
                            let _ = sender.send((q.code.clone(), q.state.clone()));
                        }
                        "<html><body><h3>Authentication succeeded. You can close this window.</h3></body></html>"
                    } else {
                        "<html><body><h3>Invalid state parameter.</h3></body></html>"
                    };
                    axum::response::Html(body)
                }
            }),
        )
    };

    // Start server with shutdown when we get the code or timeout
    let listener = tokio::net::TcpListener::bind(listen_addr).await?;

    // Open browser
    let _ = webbrowser::open(auth_url.as_str());

    // Start server as a background task and wait for callback (up to 60s)
    let server_task = tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    let result = timeout(Duration::from_secs(60), rx).await;

    // Stop server
    server_task.abort();

    let (code, returned_state) = match result {
        Ok(Ok(pair)) => pair,
        Ok(Err(_)) => return Err(anyhow!("Authentication failed to capture code")),
        Err(_) => return Err(anyhow!("Authentication timed out after 60s")),
    };
    if returned_state != state {
        return Err(anyhow!("State mismatch in OAuth callback"));
    }

    // Exchange code for token using curl to avoid adding new HTTP client deps
    let mut form: Vec<(&str, &str)> = Vec::new();
    form.push(("client_id", &client_id));
    form.push(("redirect_uri", &redirect_url));
    form.push(("grant_type", "authorization_code"));
    form.push(("code", &code));
    form.push(("code_verifier", &code_verifier));

    let mut args: Vec<String> = vec![
        "-s".into(),
        "-X".into(),
        "POST".into(),
        "-H".into(),
        "Accept: application/json".into(),
        "https://github.com/login/oauth/access_token".into(),
    ];
    for (k, v) in form.iter() {
        args.push("--data-urlencode".into());
        args.push(format!("{}={}", k, v));
    }

    let output = Command::new("curl").args(&args).output();
    let output = match output {
        Ok(o) if o.status.success() => String::from_utf8_lossy(&o.stdout).to_string(),
        Ok(o) => {
            let stderr = String::from_utf8_lossy(&o.stderr);
            return Err(anyhow!("Token exchange failed: {}", stderr));
        }
        Err(e) => return Err(anyhow!("Failed to run curl: {}", e)),
    };

    let json: Value = serde_json::from_str(&output)
        .map_err(|e| anyhow!("Failed to parse token response as JSON: {}", e))?;
    let access_token = json
        .get("access_token")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow!("No access_token in token response"))?;

    // Optional expiry
    let expires_in = json.get("expires_in").and_then(|v| v.as_u64());

    // Store token
    let config = Config::global();
    config.set_secret("GITHUB_ACCESS_TOKEN", Value::String(access_token.to_string()))?;

    if let Some(secs) = expires_in {
        let expire_at = chrono::Utc::now() + chrono::Duration::seconds(secs as i64);
        config.set_secret(
            "GITHUB_EXPIRES_AT",
            Value::String(expire_at.to_rfc3339()),
        )?;
    }

    println!("Login successful.");
    Ok(())
}

pub async fn status() -> Result<()> {
    let config = Config::global();
    match config.get_secret::<String>("GITHUB_ACCESS_TOKEN") {
        Ok(_) => {
            println!("Authenticated with GitHub (token present)");
        }
        Err(_) => {
            println!("Not authenticated. Run: goose auth login");
        }
    }
    Ok(())
}

pub async fn logout() -> Result<()> {
    let config = Config::global();
    let _ = config.delete_secret("GITHUB_ACCESS_TOKEN");
    let _ = config.delete_secret("GITHUB_EXPIRES_AT");
    println!("Logged out (local credentials removed)");
    Ok(())
}
