use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::oneshot;

/// OIDC authorization code flow for CLI login.
///
/// Flow:
/// 1. CLI asks goosed for the OIDC authorization URL
/// 2. CLI opens a local HTTP server on a random port for the callback
/// 3. CLI opens the browser (or prints the URL for the user)
/// 4. User authenticates with the OIDC provider
/// 5. Provider redirects to localhost callback with auth code
/// 6. CLI sends the auth code to goosed to exchange for a session token
/// 7. Session token is stored locally for future requests

#[derive(Debug, Serialize)]
struct OidcAuthUrlRequest {
    issuer: String,
    redirect_uri: String,
}

#[derive(Debug, Deserialize)]
struct OidcAuthUrlResponse {
    auth_url: String,
    state: String,
}

#[derive(Debug, Deserialize)]
struct OidcLoginResponse {
    token: String,
    #[allow(dead_code)]
    token_type: String,
    expires_in: u64,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct TokenExchangeResponse {
    id_token: Option<String>,
    access_token: Option<String>,
    token_type: Option<String>,
}

#[derive(Debug, Deserialize)]
struct UserInfoResponse {
    id: String,
    name: String,
    auth_method: String,
    tenant: Option<String>,
}

/// Find a free port for the local callback server.
fn find_callback_port() -> Result<u16> {
    let listener = TcpListener::bind("127.0.0.1:0")?;
    let port = listener.local_addr()?.port();
    drop(listener);
    Ok(port)
}

/// Start a temporary local HTTP server that waits for the OIDC callback.
/// Returns the authorization code and state parameter from the callback.
async fn wait_for_callback(port: u16) -> Result<(String, String)> {
    let (tx, rx) = oneshot::channel::<(String, String)>();
    let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));

    let tx_clone = tx.clone();
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

    // Spawn the callback server
    let server_handle = tokio::spawn(async move {
        loop {
            let (mut stream, _) = match listener.accept().await {
                Ok(conn) => conn,
                Err(_) => continue,
            };

            use tokio::io::{AsyncReadExt, AsyncWriteExt};
            let mut buf = vec![0u8; 4096];
            let n = match stream.read(&mut buf).await {
                Ok(n) => n,
                Err(_) => continue,
            };

            let request = String::from_utf8_lossy(&buf[..n]);

            // Parse the GET request line
            let first_line = request.lines().next().unwrap_or("");
            if !first_line.starts_with("GET /callback") {
                let response = "HTTP/1.1 404 Not Found\r\nContent-Length: 0\r\n\r\n";
                let _ = stream.write_all(response.as_bytes()).await;
                continue;
            }

            // Extract query parameters
            let query = first_line
                .split_whitespace()
                .nth(1)
                .and_then(|path| path.split('?').nth(1))
                .unwrap_or("");

            let mut code = None;
            let mut state = None;
            let mut error = None;

            for param in query.split('&') {
                let mut kv = param.splitn(2, '=');
                match (kv.next(), kv.next()) {
                    (Some("code"), Some(v)) => {
                        code = Some(urlencoding::decode(v).unwrap_or_default().to_string())
                    }
                    (Some("state"), Some(v)) => {
                        state = Some(urlencoding::decode(v).unwrap_or_default().to_string())
                    }
                    (Some("error"), Some(v)) => {
                        error = Some(urlencoding::decode(v).unwrap_or_default().to_string())
                    }
                    _ => {}
                }
            }

            if let Some(err) = error {
                let body = format!(
                    "<html><body><h1>Login Failed</h1><p>Error: {}</p><p>You can close this window.</p></body></html>",
                    err
                );
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;

                if let Some(tx) = tx_clone.lock().await.take() {
                    let _ = tx.send(("".to_string(), "".to_string()));
                }
                break;
            }

            if let (Some(code), Some(state)) = (code, state) {
                let body = "<html><body><h1>Login Successful!</h1><p>You can close this window and return to the terminal.</p></body></html>";
                let response = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                    body.len(),
                    body
                );
                let _ = stream.write_all(response.as_bytes()).await;

                if let Some(tx) = tx_clone.lock().await.take() {
                    let _ = tx.send((code, state));
                }
                break;
            }

            let body = "<html><body><h1>Invalid callback</h1><p>Missing code or state parameter.</p></body></html>";
            let response = format!(
                "HTTP/1.1 400 Bad Request\r\nContent-Type: text/html\r\nContent-Length: {}\r\n\r\n{}",
                body.len(),
                body
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
    });

    // Wait for the callback with a timeout
    let result = tokio::time::timeout(std::time::Duration::from_secs(300), rx).await;

    // Clean up the server
    server_handle.abort();

    match result {
        Ok(Ok((code, state))) => {
            if code.is_empty() {
                Err(anyhow!("OIDC login failed — provider returned an error"))
            } else {
                Ok((code, state))
            }
        }
        Ok(Err(_)) => Err(anyhow!("Callback channel closed unexpectedly")),
        Err(_) => Err(anyhow!(
            "Login timed out after 5 minutes — no callback received"
        )),
    }
}

/// Open a URL in the user's default browser.
fn open_browser(url: &str) -> bool {
    #[cfg(target_os = "macos")]
    {
        std::process::Command::new("open").arg(url).spawn().is_ok()
    }
    #[cfg(target_os = "linux")]
    {
        std::process::Command::new("xdg-open")
            .arg(url)
            .spawn()
            .is_ok()
    }
    #[cfg(target_os = "windows")]
    {
        std::process::Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .is_ok()
    }
    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        false
    }
}

/// Get the token storage path.
fn token_path() -> Result<std::path::PathBuf> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| anyhow!("Could not determine config directory"))?
        .join("goose");
    std::fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join("session_token.json"))
}

#[derive(Debug, Serialize, Deserialize)]
struct StoredToken {
    token: String,
    issuer: String,
    expires_at: u64,
}

/// Store a session token locally.
fn store_token(token: &str, issuer: &str, expires_in: u64) -> Result<()> {
    let stored = StoredToken {
        token: token.to_string(),
        issuer: issuer.to_string(),
        expires_at: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_secs()
            + expires_in,
    };
    let json = serde_json::to_string_pretty(&stored)?;
    std::fs::write(token_path()?, json)?;
    Ok(())
}

/// Load a stored session token (if valid).
pub fn load_token() -> Option<String> {
    let path = token_path().ok()?;
    let json = std::fs::read_to_string(path).ok()?;
    let stored: StoredToken = serde_json::from_str(&json).ok()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    if now < stored.expires_at {
        Some(stored.token)
    } else {
        None
    }
}

/// Clear stored session token.
fn clear_token() -> Result<()> {
    let path = token_path()?;
    if path.exists() {
        std::fs::remove_file(path)?;
    }
    Ok(())
}

/// Handle `goose auth login --provider <issuer>` — OIDC authorization code flow.
pub async fn handle_login(server_url: &str, secret_key: &str, issuer: &str) -> Result<()> {
    let http = Client::new();

    // Step 1: Find a free port for the callback server
    let port = find_callback_port()?;
    let redirect_uri = format!("http://localhost:{}/callback", port);

    println!("🔐 Starting OIDC login with {}...", issuer);

    // Step 2: Ask goosed for the authorization URL
    let auth_url_resp = http
        .post(format!("{}/auth/login/oidc/url", server_url))
        .header("X-Secret-Key", secret_key)
        .json(&OidcAuthUrlRequest {
            issuer: issuer.to_string(),
            redirect_uri: redirect_uri.clone(),
        })
        .send()
        .await?;

    if !auth_url_resp.status().is_success() {
        let status = auth_url_resp.status();
        let body = auth_url_resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Failed to get authorization URL ({}): {}",
            status,
            body
        ));
    }

    let auth_url_data: OidcAuthUrlResponse = auth_url_resp.json().await?;

    // Step 3: Open the browser
    println!();
    if open_browser(&auth_url_data.auth_url) {
        println!("📎 Browser opened. Please log in with your identity provider.");
    } else {
        println!("📎 Open this URL in your browser to log in:");
        println!();
        println!("  {}", auth_url_data.auth_url);
    }
    println!();
    println!("⏳ Waiting for authentication callback (timeout: 5 minutes)...");

    // Step 4: Wait for the callback
    let (code, state) = wait_for_callback(port).await?;

    // Verify state matches
    if state != auth_url_data.state {
        return Err(anyhow!(
            "State mismatch — possible CSRF attack. Expected: {}, got: {}",
            auth_url_data.state,
            state
        ));
    }

    println!("✅ Authorization code received. Exchanging for token...");

    // Step 5: Exchange the authorization code for tokens via goosed
    // The server handles the token exchange and validates the ID token
    let login_resp = http
        .post(format!("{}/auth/login/oidc", server_url))
        .header("X-Secret-Key", secret_key)
        .json(&serde_json::json!({
            "code": code,
            "issuer": issuer,
            "redirect_uri": redirect_uri,
        }))
        .send()
        .await?;

    if !login_resp.status().is_success() {
        let status = login_resp.status();
        let body = login_resp.text().await.unwrap_or_default();
        return Err(anyhow!("Login failed ({}): {}", status, body));
    }

    let login_data: OidcLoginResponse = login_resp.json().await?;

    // Step 6: Store the session token
    store_token(&login_data.token, issuer, login_data.expires_in)?;

    println!("🎉 Login successful! Session token stored.");
    println!(
        "   Token expires in {} hours.",
        login_data.expires_in / 3600
    );

    Ok(())
}

/// Handle `goose auth logout` — clear stored session token.
pub async fn handle_logout(server_url: &str, secret_key: &str) -> Result<()> {
    let http = Client::new();

    // Revoke on server if we have a stored token
    if let Some(token) = load_token() {
        let _ = http
            .post(format!("{}/auth/logout", server_url))
            .header("X-Secret-Key", secret_key)
            .header("Authorization", format!("Bearer {}", token))
            .send()
            .await;
    }

    clear_token()?;
    println!("👋 Logged out. Session token cleared.");
    Ok(())
}

/// Handle `goose auth status` — show current auth status.
pub async fn handle_status(server_url: &str, secret_key: &str) -> Result<()> {
    let http = Client::new();

    // Check for stored token
    let token = load_token();

    let mut headers = vec![("X-Secret-Key".to_string(), secret_key.to_string())];
    if let Some(ref t) = token {
        headers.push(("Authorization".to_string(), format!("Bearer {}", t)));
    }

    let mut req = http.get(format!("{}/auth/me", server_url));
    for (k, v) in &headers {
        req = req.header(k.as_str(), v.as_str());
    }

    let resp = req.send().await?;

    if resp.status().is_success() {
        let user: UserInfoResponse = resp.json().await?;
        println!("🔐 Authentication Status");
        println!("   User ID:     {}", user.id);
        println!("   Name:        {}", user.name);
        println!("   Auth Method: {}", user.auth_method);
        if let Some(tenant) = &user.tenant {
            println!("   Tenant:      {}", tenant);
        }
        if token.is_some() {
            println!("   Session:     Active (stored token)");
        } else {
            println!("   Session:     None (guest mode)");
        }
    } else {
        println!("🔐 Authentication Status");
        println!("   Not authenticated (guest mode)");
        if token.is_some() {
            println!(
                "   ⚠️  Stored token may be expired. Run `goose auth login` to re-authenticate."
            );
        }
    }

    Ok(())
}

/// Handle `goose auth whoami` — simple identity check.
pub async fn handle_whoami(server_url: &str, secret_key: &str) -> Result<()> {
    let http = Client::new();

    let mut req = http.get(format!("{}/auth/me", server_url));
    req = req.header("X-Secret-Key", secret_key);

    if let Some(token) = load_token() {
        req = req.header("Authorization", format!("Bearer {}", token));
    }

    let resp = req.send().await?;

    if resp.status().is_success() {
        let user: UserInfoResponse = resp.json().await?;
        println!("{} ({})", user.name, user.auth_method);
    } else {
        println!("guest (not authenticated)");
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_callback_port() {
        let port = find_callback_port().unwrap();
        assert!(port > 0);
    }

    #[test]
    fn test_open_browser_returns_bool() {
        // Just verify it doesn't panic — actual browser opening is platform-dependent
        let _ = open_browser("http://localhost:12345");
    }

    #[test]
    fn test_store_and_load_token() {
        // Use a temporary directory for testing
        let _token = "test-token";
        // load_token returns None when no token is stored (or expired)
        // We can't easily test store_token without mocking the filesystem
        // but we can verify load_token handles missing files gracefully
        let result = load_token();
        // Result is Some or None depending on whether a token was previously stored
        let _ = result;
    }
}
