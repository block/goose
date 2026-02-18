use anyhow::{anyhow, Result};
use goose::config::Config;
use goose::oidc::OidcProviderPreset;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::net::TcpListener;
use std::sync::Arc;
use tokio::sync::oneshot;

/// Keyring secret key for the CLI session token.
const SESSION_TOKEN_SECRET_KEY: &str = "goose_session_token";

/// OIDC authorization code flow for CLI login.
///
/// Supports named provider presets (google, azure, github, gitlab, aws, auth0, okta)
/// or raw issuer URLs for custom OIDC providers.
///
/// Flow:
/// 1. Resolve provider name to issuer URL (or use raw URL)
/// 2. CLI asks goosed for the OIDC authorization URL
/// 3. CLI opens a local HTTP server on a random port for the callback
/// 4. CLI opens the browser (or prints the URL for the user)
/// 5. User authenticates with the OIDC provider
/// 6. Provider redirects to localhost callback with auth code
/// 7. CLI sends the auth code to goosed to exchange for a session token
/// 8. Session token is stored locally for future requests

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
    #[serde(default)]
    refresh_token: Option<String>,
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

/// Start a local HTTP server that waits for the OIDC callback.
/// Returns (authorization_code, state) when the callback is received.
async fn wait_for_callback(port: u16) -> Result<(String, String)> {
    let listener = tokio::net::TcpListener::bind(format!("127.0.0.1:{}", port)).await?;

    let (tx, rx) = oneshot::channel::<(String, String)>();
    let tx = Arc::new(tokio::sync::Mutex::new(Some(tx)));

    let server_handle = tokio::spawn(async move {
        use tokio::io::AsyncWriteExt;

        loop {
            let Ok((mut stream, _)) = listener.accept().await else {
                break;
            };

            let tx_clone = tx.clone();

            let mut buf = vec![0u8; 4096];
            let n = tokio::io::AsyncReadExt::read(&mut stream, &mut buf)
                .await
                .unwrap_or(0);
            let request = String::from_utf8_lossy(&buf[..n]);

            let query = request
                .lines()
                .next()
                .and_then(|line| line.split_whitespace().nth(1))
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

#[derive(Serialize, Deserialize)]
struct StoredToken {
    token: String,
    issuer: String,
    expires_at: u64,
    #[serde(default)]
    refresh_token: Option<String>,
    #[serde(default)]
    server_url: Option<String>,
    #[serde(default)]
    secret_key: Option<String>,
}

/// Legacy token path for migration from plaintext JSON to keyring.
fn legacy_token_path() -> Option<std::path::PathBuf> {
    dirs::config_dir().map(|d| d.join("goose").join("session_token.json"))
}

/// Migrate from legacy `session_token.json` to keyring if the file exists.
fn migrate_legacy_token() {
    let Some(path) = legacy_token_path() else {
        return;
    };
    if !path.exists() {
        return;
    }
    let Ok(json) = std::fs::read_to_string(&path) else {
        return;
    };
    let Ok(stored) = serde_json::from_str::<StoredToken>(&json) else {
        // Corrupt file — remove it
        let _ = std::fs::remove_file(&path);
        return;
    };

    let config = Config::global();
    if config.set_secret(SESSION_TOKEN_SECRET_KEY, &stored).is_ok() {
        let _ = std::fs::remove_file(&path);
        tracing::info!("migrated session token from session_token.json to keyring");
    }
}

fn store_token(
    token: &str,
    issuer: &str,
    expires_in: u64,
    refresh_token: Option<&str>,
    server_url: Option<&str>,
    secret_key: Option<&str>,
) -> Result<()> {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_secs();

    let stored = StoredToken {
        token: token.to_string(),
        issuer: issuer.to_string(),
        expires_at: now + expires_in,
        refresh_token: refresh_token.map(|s| s.to_string()),
        server_url: server_url.map(|s| s.to_string()),
        secret_key: secret_key.map(|s| s.to_string()),
    };

    Config::global()
        .set_secret(SESSION_TOKEN_SECRET_KEY, &stored)
        .map_err(|e| anyhow!("failed to store session token: {}", e))
}

/// Load the stored token from keyring, deserialize it.
fn load_stored_token() -> Option<StoredToken> {
    // One-time migration from legacy plaintext file
    migrate_legacy_token();

    let config = Config::global();
    config
        .get_secret::<StoredToken>(SESSION_TOKEN_SECRET_KEY)
        .ok()
}

/// Load the session token, auto-refreshing via OIDC if expired but a refresh_token is available.
pub fn load_token() -> Option<String> {
    let stored = load_stored_token()?;

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .ok()?
        .as_secs();

    if now < stored.expires_at {
        return Some(stored.token);
    }

    // Token expired — try auto-refresh if we have a refresh_token
    if let (Some(refresh_token), Some(server_url), Some(secret_key)) = (
        &stored.refresh_token,
        &stored.server_url,
        &stored.secret_key,
    ) {
        if let Some(new_token) =
            try_refresh_token(server_url, secret_key, &stored.issuer, refresh_token)
        {
            return Some(new_token);
        }
    }

    None
}

/// Attempt to refresh the session token via the server's OIDC refresh endpoint.
fn try_refresh_token(
    server_url: &str,
    secret_key: &str,
    issuer: &str,
    refresh_token: &str,
) -> Option<String> {
    let client = reqwest::blocking::Client::new();
    let resp = client
        .post(format!("{}/auth/refresh/oidc", server_url))
        .header("X-Secret-Key", secret_key)
        .json(&serde_json::json!({
            "issuer": issuer,
            "refresh_token": refresh_token,
        }))
        .send()
        .ok()?;

    if !resp.status().is_success() {
        return None;
    }

    #[derive(Deserialize)]
    struct RefreshResp {
        token: String,
        expires_in: u64,
        #[serde(default)]
        refresh_token: Option<String>,
    }

    let data: RefreshResp = resp.json().ok()?;

    // Update stored token with the new one (and rotated refresh_token if present)
    let new_refresh = data.refresh_token.as_deref().or(Some(refresh_token));
    store_token(
        &data.token,
        issuer,
        data.expires_in,
        new_refresh,
        Some(server_url),
        Some(secret_key),
    )
    .ok()?;

    Some(data.token)
}

fn clear_token() -> Result<()> {
    Config::global()
        .delete_secret(SESSION_TOKEN_SECRET_KEY)
        .map_err(|e| anyhow!("failed to clear session token: {}", e))?;

    // Also clean up legacy file if it still exists
    if let Some(path) = legacy_token_path() {
        if path.exists() {
            let _ = std::fs::remove_file(path);
        }
    }

    Ok(())
}

/// Resolve a provider string to an OIDC issuer URL.
/// Accepts:
/// - Preset names: "google", "azure", "github", "gitlab", "aws", "auth0", "okta"
/// - Raw issuer URLs: "https://accounts.google.com"
fn resolve_provider(provider: &str, tenant: Option<&str>) -> (String, Option<OidcProviderPreset>) {
    if let Some(preset) = OidcProviderPreset::from_name(provider) {
        // Extract issuer from discovery URL by stripping /.well-known/openid-configuration
        let discovery = preset.discovery_url(tenant);
        let issuer = discovery
            .strip_suffix("/.well-known/openid-configuration")
            .unwrap_or(&discovery)
            .to_string();
        (issuer, Some(preset))
    } else if provider.starts_with("http://") || provider.starts_with("https://") {
        (provider.to_string(), None)
    } else {
        (format!("https://{}", provider), None)
    }
}

/// Handle `goose auth login --provider <name>` — OIDC authorization code flow.
pub async fn handle_login(
    server_url: &str,
    secret_key: &str,
    provider: &str,
    tenant: Option<&str>,
    client_id: Option<&str>,
) -> Result<()> {
    let http = Client::new();
    let (issuer, preset) = resolve_provider(provider, tenant);

    let display_name = preset
        .as_ref()
        .map(|p| p.to_string())
        .unwrap_or_else(|| issuer.clone());

    println!("🔐 Starting login with {}...", display_name);

    // Check if GitHub (uses OAuth2, not OIDC code flow)
    if let Some(ref preset) = preset {
        if !preset.supports_oidc_code_flow() {
            return handle_github_oauth2_login(server_url, secret_key, &http, preset, client_id)
                .await;
        }
    }

    // Step 1: Find a free port for the callback server
    let port = find_callback_port()?;
    let redirect_uri = format!("http://localhost:{}/callback", port);

    // Step 2: Ask goosed for the authorization URL
    let auth_url_resp = http
        .post(format!("{}/auth/login/oidc/url", server_url))
        .header("X-Secret-Key", secret_key)
        .json(&OidcAuthUrlRequest {
            issuer: issuer.clone(),
            redirect_uri: redirect_uri.clone(),
        })
        .send()
        .await?;

    if !auth_url_resp.status().is_success() {
        let status = auth_url_resp.status();
        let body = auth_url_resp.text().await.unwrap_or_default();
        return Err(anyhow!(
            "Failed to get authorization URL ({}): {}.\n\
             Hint: Make sure the OIDC provider is configured on the server.\n\
             Run: goose auth providers",
            status,
            body
        ));
    }

    let auth_url_data: OidcAuthUrlResponse = auth_url_resp.json().await?;

    // Step 3: Open the browser
    println!();
    if open_browser(&auth_url_data.auth_url) {
        println!("📎 Browser opened. Please log in with {}.", display_name);
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
    let login_resp = http
        .post(format!("{}/auth/login/oidc/code", server_url))
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

    // Step 6: Store the session token (with refresh_token if available)
    store_token(
        &login_data.token,
        &issuer,
        login_data.expires_in,
        login_data.refresh_token.as_deref(),
        Some(server_url),
        Some(secret_key),
    )?;

    println!("🎉 Login successful! Session token stored.");
    println!(
        "   Token expires in {} hours.",
        login_data.expires_in / 3600
    );
    if login_data.refresh_token.is_some() {
        println!("   Auto-refresh enabled (refresh token stored).");
    }

    Ok(())
}

/// GitHub uses OAuth2 (not OIDC) — handle it with the device flow or web flow.
async fn handle_github_oauth2_login(
    server_url: &str,
    secret_key: &str,
    http: &Client,
    preset: &OidcProviderPreset,
    client_id: Option<&str>,
) -> Result<()> {
    let client_id = client_id.ok_or_else(|| {
        anyhow!(
            "GitHub login requires a --client-id.\n\
             Create an OAuth App at https://github.com/settings/developers\n\
             and provide the client ID."
        )
    })?;

    let port = find_callback_port()?;
    let redirect_uri = format!("http://localhost:{}/callback", port);

    let authorize_url = preset.oauth2_authorize_url().unwrap();
    let state = uuid::Uuid::new_v4().to_string();

    let auth_url = format!(
        "{}?client_id={}&redirect_uri={}&scope={}&state={}",
        authorize_url,
        urlencoding::encode(client_id),
        urlencoding::encode(&redirect_uri),
        urlencoding::encode("read:user user:email"),
        urlencoding::encode(&state),
    );

    println!();
    if open_browser(&auth_url) {
        println!("📎 Browser opened. Please authorize the GitHub OAuth App.");
    } else {
        println!("📎 Open this URL in your browser to authorize:");
        println!();
        println!("  {}", auth_url);
    }
    println!();
    println!("⏳ Waiting for GitHub callback (timeout: 5 minutes)...");

    let (code, returned_state) = wait_for_callback(port).await?;

    if returned_state != state {
        return Err(anyhow!("State mismatch — possible CSRF attack."));
    }

    println!("✅ Authorization code received. Exchanging for token...");

    // Exchange code with GitHub's token endpoint
    let token_url = preset.oauth2_token_url().unwrap();
    let token_resp = http
        .post(token_url)
        .header("Accept", "application/json")
        .form(&[
            ("client_id", client_id),
            ("code", &code),
            ("redirect_uri", &redirect_uri),
        ])
        .send()
        .await?;

    if !token_resp.status().is_success() {
        let body = token_resp.text().await.unwrap_or_default();
        return Err(anyhow!("GitHub token exchange failed: {}", body));
    }

    #[derive(Deserialize)]
    struct GitHubTokenResponse {
        access_token: String,
    }

    let gh_token: GitHubTokenResponse = token_resp.json().await?;

    // Use the GitHub access token to get user info, then create a session
    let user_resp = http
        .get("https://api.github.com/user")
        .header("Authorization", format!("Bearer {}", gh_token.access_token))
        .header("User-Agent", "goose-cli")
        .send()
        .await?;

    if !user_resp.status().is_success() {
        return Err(anyhow!("Failed to fetch GitHub user info"));
    }

    #[derive(Deserialize)]
    struct GitHubUser {
        login: String,
        id: u64,
    }

    let gh_user: GitHubUser = user_resp.json().await?;

    // Create a session via goosed's login endpoint using the GitHub identity
    let login_resp = http
        .post(format!("{}/auth/login", server_url))
        .header("X-Secret-Key", secret_key)
        .json(&serde_json::json!({
            "api_key": format!("github:{}", gh_user.id),
            "display_name": gh_user.login,
        }))
        .send()
        .await?;

    if !login_resp.status().is_success() {
        let body = login_resp.text().await.unwrap_or_default();
        return Err(anyhow!("Login failed: {}", body));
    }

    let login_data: OidcLoginResponse = login_resp.json().await?;
    store_token(
        &login_data.token,
        "https://github.com",
        login_data.expires_in,
        None, // GitHub OAuth2 doesn't provide OIDC refresh tokens
        Some(server_url),
        Some(secret_key),
    )?;

    println!("🎉 Logged in as {} (GitHub)", gh_user.login);
    println!(
        "   Token expires in {} hours.",
        login_data.expires_in / 3600
    );

    Ok(())
}

/// Handle `goose auth logout` — clear stored session token.
pub async fn handle_logout(server_url: &str, secret_key: &str) -> Result<()> {
    let http = Client::new();

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

    let token = load_token();

    let mut req = http.get(format!("{}/auth/me", server_url));
    req = req.header("X-Secret-Key", secret_key);
    if let Some(ref t) = token {
        req = req.header("Authorization", format!("Bearer {}", t));
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

/// Handle `goose auth providers` — list available OIDC provider presets.
pub async fn handle_providers() -> Result<()> {
    println!("🔐 Supported Identity Providers\n");
    println!("  {:<12} {:<35} Notes", "Name", "Provider");
    println!("  {}", "─".repeat(75));

    for preset in OidcProviderPreset::all() {
        let notes = match preset {
            OidcProviderPreset::Google => "Standard OIDC",
            OidcProviderPreset::Azure => "Use --tenant <tenant-id> for single-tenant",
            OidcProviderPreset::GitHub => "OAuth2 (requires --client-id)",
            OidcProviderPreset::GitLab => "Use --tenant <host> for self-hosted",
            OidcProviderPreset::Aws => "Use --tenant <pool-id> (e.g., us-west-2_abc123)",
            OidcProviderPreset::Auth0 => "Use --tenant <domain> (e.g., myapp.auth0.com)",
            OidcProviderPreset::Okta => "Use --tenant <domain> (e.g., dev-123.okta.com)",
        };

        let name = format!("{:?}", preset).to_lowercase();
        println!("  {:<12} {:<35} {}", name, preset.to_string(), notes,);
    }

    println!();
    println!("Usage:");
    println!("  goose auth login --provider google");
    println!("  goose auth login --provider azure --tenant <tenant-id>");
    println!("  goose auth login --provider github --client-id <id>");
    println!("  goose auth login --provider gitlab");
    println!("  goose auth login --provider aws --tenant us-west-2_abc123");
    println!("  goose auth login --provider https://custom-oidc.example.com");

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
        let _ = open_browser("http://localhost:12345");
    }

    #[test]
    fn test_resolve_provider_preset() {
        let (issuer, preset) = resolve_provider("google", None);
        assert!(issuer.contains("accounts.google.com"));
        assert!(preset.is_some());
        assert_eq!(preset.unwrap(), OidcProviderPreset::Google);
    }

    #[test]
    fn test_resolve_provider_azure_with_tenant() {
        let (issuer, preset) = resolve_provider("azure", Some("my-tenant-id"));
        assert!(issuer.contains("my-tenant-id"));
        assert!(issuer.contains("login.microsoftonline.com"));
        assert_eq!(preset.unwrap(), OidcProviderPreset::Azure);
    }

    #[test]
    fn test_resolve_provider_raw_url() {
        let (issuer, preset) = resolve_provider("https://custom-oidc.example.com", None);
        assert_eq!(issuer, "https://custom-oidc.example.com");
        assert!(preset.is_none());
    }

    #[test]
    fn test_resolve_provider_bare_domain() {
        let (issuer, preset) = resolve_provider("custom-oidc.example.com", None);
        assert_eq!(issuer, "https://custom-oidc.example.com");
        assert!(preset.is_none());
    }

    #[test]
    fn test_resolve_provider_github() {
        let (_, preset) = resolve_provider("github", None);
        let preset = preset.unwrap();
        assert!(!preset.supports_oidc_code_flow());
        assert!(preset.oauth2_authorize_url().is_some());
    }

    #[test]
    fn test_resolve_provider_aliases() {
        assert!(resolve_provider("gh", None).1.is_some());
        assert!(resolve_provider("gl", None).1.is_some());
        assert!(resolve_provider("microsoft", None).1.is_some());
        assert!(resolve_provider("cognito", None).1.is_some());
    }

    #[test]
    fn test_load_token_graceful_on_missing() {
        let result = load_token();
        let _ = result; // Should not panic
    }
}
