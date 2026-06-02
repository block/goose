use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub(super) struct TokenData {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: Option<String>,
    pub expires_at: DateTime<Utc>,
    pub account_id: Option<String>,
}

#[derive(Debug, Clone)]
pub(super) struct TokenCache {
    pub cache_path: PathBuf,
}

pub(super) fn get_cache_path() -> PathBuf {
    Paths::in_config_dir("chatgpt_codex/tokens.json")
}

impl TokenCache {
    pub fn new() -> Self {
        let cache_path = get_cache_path();
        if let Some(parent) = cache_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        Self { cache_path }
    }

    pub fn load(&self) -> Option<TokenData> {
        if let Ok(contents) = std::fs::read_to_string(&self.cache_path) {
            serde_json::from_str(&contents).ok()
        } else {
            None
        }
    }

    pub fn save(&self, token_data: &TokenData) -> Result<()> {
        if let Some(parent) = self.cache_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string(token_data)?;
        std::fs::write(&self.cache_path, contents)?;
        Ok(())
    }

    pub fn clear(&self) {
        let _ = std::fs::remove_file(&self.cache_path);
    }
}

#[derive(Debug, Deserialize)]
pub(super) struct JwtClaims {
    pub chatgpt_account_id: Option<String>,
    #[serde(rename = "https://api.openai.com/auth")]
    pub auth_claims: Option<AuthClaims>,
    pub organizations: Option<Vec<OrgInfo>>,
}

#[derive(Debug, Deserialize)]
pub(super) struct AuthClaims {
    pub chatgpt_account_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub(super) struct OrgInfo {
    pub id: String,
}

#[derive(Debug, Deserialize)]
struct OidcConfiguration {
    jwks_uri: String,
}

pub(super) async fn fetch_jwks_for(issuer: &str) -> Result<JwkSet> {
    let client = reqwest::Client::new();
    let config_url = format!("{}/.well-known/openid-configuration", issuer);
    let config = client
        .get(config_url)
        .send()
        .await?
        .error_for_status()?
        .json::<OidcConfiguration>()
        .await?;

    let jwks = client
        .get(config.jwks_uri)
        .send()
        .await?
        .error_for_status()?
        .json::<JwkSet>()
        .await?;

    Ok(jwks)
}

pub(super) async fn get_jwks(state: &ChatGptCodexAuthState) -> Result<JwkSet> {
    let mut cache = state.jwks_cache.lock().await;
    if let Some(jwks) = cache.clone() {
        return Ok(jwks);
    }
    let jwks = fetch_jwks_for(ISSUER).await?;
    *cache = Some(jwks.clone());
    Ok(jwks)
}

pub(super) fn parse_jwt_claims_with_jwks(token: &str, jwks: &JwkSet) -> Result<JwtClaims> {
    let header = decode_header(token)?;
    let kid = header
        .kid
        .ok_or_else(|| anyhow!("JWT header missing kid"))?;
    let jwk = jwks
        .find(&kid)
        .ok_or_else(|| anyhow!("JWT signing key not found"))?;
    let decoding_key = DecodingKey::from_jwk(jwk)?;

    let mut validation = Validation::new(header.alg);
    validation.validate_aud = false;

    let token_data = decode::<JwtClaims>(token, &decoding_key, &validation)?;
    Ok(token_data.claims)
}

pub(super) fn parse_jwt_claims_unverified(token: &str) -> Option<JwtClaims> {
    let parts: Vec<&str> = token.split('.').collect();
    if parts.len() != 3 {
        return None;
    }
    let payload = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(parts[1])
        .ok()?;
    serde_json::from_slice(&payload).ok()
}

pub(super) async fn parse_jwt_claims(
    token: &str,
    state: &ChatGptCodexAuthState,
) -> Option<JwtClaims> {
    if let Ok(jwks) = get_jwks(state).await {
        if let Ok(claims) = parse_jwt_claims_with_jwks(token, &jwks) {
            return Some(claims);
        }
    }
    parse_jwt_claims_unverified(token)
}

pub(super) fn account_id_from_claims(claims: &JwtClaims) -> Option<String> {
    if let Some(id) = claims.chatgpt_account_id.as_ref() {
        return Some(id.clone());
    }
    if let Some(auth) = claims.auth_claims.as_ref() {
        if let Some(id) = auth.chatgpt_account_id.as_ref() {
            return Some(id.clone());
        }
    }
    if let Some(orgs) = claims.organizations.as_ref() {
        if let Some(org) = orgs.first() {
            return Some(org.id.clone());
        }
    }
    None
}

pub(super) async fn extract_account_id(
    token_data: &TokenData,
    state: &ChatGptCodexAuthState,
) -> Option<String> {
    if let Some(id_token) = token_data.id_token.as_deref() {
        if let Some(claims) = parse_jwt_claims(id_token, state).await {
            if let Some(account_id) = account_id_from_claims(&claims) {
                return Some(account_id);
            }
        }
    }

    parse_jwt_claims(&token_data.access_token, state)
        .await
        .and_then(|claims| account_id_from_claims(&claims))
}

pub(super) struct PkceChallenge {
    pub verifier: String,
    pub challenge: String,
}

pub(super) fn generate_pkce() -> PkceChallenge {
    let verifier = nanoid::nanoid!(43);
    let digest = sha2::Sha256::digest(verifier.as_bytes());
    let challenge = base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(digest);
    PkceChallenge {
        verifier,
        challenge,
    }
}

pub(super) fn generate_state() -> String {
    nanoid::nanoid!(32)
}

pub(super) fn build_authorize_url(
    redirect_uri: &str,
    pkce: &PkceChallenge,
    state: &str,
) -> Result<String> {
    let scopes = OAUTH_SCOPES.join(" ");
    let params = [
        ("response_type", "code"),
        ("client_id", CLIENT_ID),
        ("redirect_uri", redirect_uri),
        ("scope", &scopes),
        ("code_challenge", &pkce.challenge),
        ("code_challenge_method", "S256"),
        ("id_token_add_organizations", "true"),
        ("codex_cli_simplified_flow", "true"),
        ("state", state),
        ("originator", "goose"),
    ];
    let query = serde_urlencoded::to_string(params)?;
    Ok(format!("{}/oauth/authorize?{}", ISSUER, query))
}

#[derive(Debug, Deserialize)]
pub(super) struct TokenResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub id_token: Option<String>,
    pub expires_in: Option<i64>,
}

pub(super) async fn exchange_code_for_tokens_with_issuer(
    issuer: &str,
    code: &str,
    redirect_uri: &str,
    pkce: &PkceChallenge,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "authorization_code"),
        ("code", code),
        ("redirect_uri", redirect_uri),
        ("client_id", CLIENT_ID),
        ("code_verifier", &pkce.verifier),
    ];

    let resp = client
        .post(format!("{}/oauth/token", issuer))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Token exchange failed ({}): {}", status, text));
    }

    Ok(resp.json().await?)
}

pub(super) async fn refresh_access_token_with_issuer(
    issuer: &str,
    refresh_token: &str,
) -> Result<TokenResponse> {
    let client = reqwest::Client::new();
    let params = [
        ("grant_type", "refresh_token"),
        ("refresh_token", refresh_token),
        ("client_id", CLIENT_ID),
    ];

    let resp = client
        .post(format!("{}/oauth/token", issuer))
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;

    if !resp.status().is_success() {
        let status = resp.status();
        let text = resp.text().await.unwrap_or_default();
        return Err(anyhow!("Token refresh failed ({}): {}", status, text));
    }

    Ok(resp.json().await?)
}

const HTML_SUCCESS_TEMPLATE: &str = r#"<!doctype html>
<html>
  <head>
    <title>goose - ChatGPT Authorization Successful</title>
    <style>
      body {
        font-family: system-ui, -apple-system, sans-serif;
        display: flex;
        justify-content: center;
        align-items: center;
        height: 100vh;
        margin: 0;
        background: #131010;
        color: #f1ecec;
      }
      .container { text-align: center; padding: 2rem; }
      h1 { color: #f1ecec; margin-bottom: 1rem; }
      p { color: #b7b1b1; }
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Authorization Successful</h1>
      <p>You can close this window and return to goose.</p>
    </div>
    <script>const AUTO_CLOSE_TIMEOUT_MS = __AUTO_CLOSE_TIMEOUT_MS__; setTimeout(() => window.close(), AUTO_CLOSE_TIMEOUT_MS)</script>
  </body>
</html>"#;

fn html_success() -> String {
    HTML_SUCCESS_TEMPLATE.replace(
        "__AUTO_CLOSE_TIMEOUT_MS__",
        &HTML_AUTO_CLOSE_TIMEOUT_MS.to_string(),
    )
}

fn html_error(error: &str) -> String {
    let safe_error = v_htmlescape::escape(error).to_string();
    format!(
        r#"<!doctype html>
<html>
  <head>
    <title>goose - ChatGPT Authorization Failed</title>
    <style>
      body {{
        font-family: system-ui, -apple-system, sans-serif;
        display: flex;
        justify-content: center;
        align-items: center;
        height: 100vh;
        margin: 0;
        background: #131010;
        color: #f1ecec;
      }}
      .container {{ text-align: center; padding: 2rem; }}
      h1 {{ color: #fc533a; margin-bottom: 1rem; }}
      p {{ color: #b7b1b1; }}
      .error {{
        color: #ff917b;
        font-family: monospace;
        margin-top: 1rem;
        padding: 1rem;
        background: #3c140d;
        border-radius: 0.5rem;
      }}
    </style>
  </head>
  <body>
    <div class="container">
      <h1>Authorization Failed</h1>
      <p>An error occurred during authorization.</p>
      <div class="error">{}</div>
    </div>
  </body>
</html>"#,
        safe_error
    )
}

#[derive(Deserialize)]
struct CallbackParams {
    code: Option<String>,
    state: Option<String>,
    error: Option<String>,
    error_description: Option<String>,
}

pub(super) fn oauth_callback_router(
    expected_state: String,
    tx: Arc<TokioMutex<Option<oneshot::Sender<Result<String>>>>>,
) -> Router {
    Router::new().route(
        "/auth/callback",
        get(move |Query(params): Query<CallbackParams>| {
            let tx = tx.clone();
            let expected = expected_state.clone();
            async move {
                if let Some(error) = params.error {
                    let msg = params.error_description.unwrap_or(error);
                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send(Err(anyhow!("{}", msg)));
                    }
                    return Html(html_error(&msg));
                }

                let code = match params.code {
                    Some(c) => c,
                    None => {
                        let msg = "Missing authorization code";
                        if let Some(sender) = tx.lock().await.take() {
                            let _ = sender.send(Err(anyhow!("{}", msg)));
                        }
                        return Html(html_error(msg));
                    }
                };

                if params.state.as_deref() != Some(&expected) {
                    let msg = "Invalid state - potential CSRF attack";
                    if let Some(sender) = tx.lock().await.take() {
                        let _ = sender.send(Err(anyhow!("{}", msg)));
                    }
                    return Html(html_error(msg));
                }

                if let Some(sender) = tx.lock().await.take() {
                    let _ = sender.send(Ok(code));
                }
                Html(html_success())
            }
        }),
    )
}

pub(super) async fn spawn_oauth_server(app: Router) -> Result<tokio::task::JoinHandle<()>> {
    let addr = SocketAddr::from(([127, 0, 0, 1], OAUTH_PORT));
    let listener = tokio::net::TcpListener::bind(addr).await.map_err(|e| {
        if e.kind() == io::ErrorKind::AddrInUse {
            anyhow!(
                "OAuth callback server failed to bind to {}: port {} is already in use. \
                 Please stop the process using this port and try again.",
                addr,
                OAUTH_PORT
            )
        } else {
            anyhow!("OAuth callback server failed to bind to {}: {}", addr, e)
        }
    })?;
    Ok(tokio::spawn(async move {
        let server = axum::serve(listener, app);
        let _ = server.await;
    }))
}

pub(super) struct ServerHandleGuard(Option<tokio::task::JoinHandle<()>>);

impl ServerHandleGuard {
    pub fn new(handle: tokio::task::JoinHandle<()>) -> Self {
        Self(Some(handle))
    }

    pub fn abort(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
    }
}

impl Drop for ServerHandleGuard {
    fn drop(&mut self) {
        self.abort();
    }
}

pub(super) async fn wait_for_oauth_code(rx: oneshot::Receiver<Result<String>>) -> Result<String> {
    let code_result =
        tokio::time::timeout(std::time::Duration::from_secs(OAUTH_TIMEOUT_SECS), rx).await;
    code_result
        .map_err(|_| anyhow!("OAuth flow timed out"))??
        .map_err(|e| anyhow!("OAuth callback error: {}", e))
}

pub(super) async fn perform_oauth_flow(auth_state: &ChatGptCodexAuthState) -> Result<TokenData> {
    let _guard = auth_state.oauth_mutex.try_lock().map_err(|_| {
        anyhow!("Another OAuth flow is already in progress; please try again later")
    })?;

    let pkce = generate_pkce();
    let csrf_state = generate_state();
    let redirect_uri = format!("http://localhost:{}/auth/callback", OAUTH_PORT);
    let auth_url = build_authorize_url(&redirect_uri, &pkce, &csrf_state)?;

    let (tx, rx) = oneshot::channel::<Result<String>>();
    let tx = Arc::new(TokioMutex::new(Some(tx)));
    let app = oauth_callback_router(csrf_state.clone(), tx);
    let server_handle = spawn_oauth_server(app).await?;
    let mut server_guard = ServerHandleGuard::new(server_handle);

    if webbrowser::open(&auth_url).is_err() {
        tracing::info!("Please open this URL in your browser:\n{}", auth_url);
    }

    let code_result = wait_for_oauth_code(rx).await;
    server_guard.abort();
    let code = code_result?;

    let tokens = exchange_code_for_tokens_with_issuer(ISSUER, &code, &redirect_uri, &pkce).await?;

    let expires_at = Utc::now() + chrono::Duration::seconds(tokens.expires_in.unwrap_or(3600));

    let mut token_data = TokenData {
        access_token: tokens.access_token,
        refresh_token: tokens.refresh_token,
        id_token: tokens.id_token,
        expires_at,
        account_id: None,
    };

    token_data.account_id = extract_account_id(&token_data, auth_state).await;

    Ok(token_data)
}

#[derive(Debug)]
pub(super) struct ChatGptCodexAuthProvider {
    pub cache: TokenCache,
    pub state: Arc<ChatGptCodexAuthState>,
}

impl ChatGptCodexAuthProvider {
    pub fn new(state: Arc<ChatGptCodexAuthState>) -> Self {
        Self {
            cache: TokenCache::new(),
            state,
        }
    }

    pub fn clear_cached_tokens(&self) {
        self.cache.clear();
    }

    pub async fn get_valid_token(&self) -> Result<TokenData> {
        if let Some(mut token_data) = self.cache.load() {
            if token_data.expires_at > Utc::now() + chrono::Duration::seconds(60) {
                return Ok(token_data);
            }

            tracing::debug!("Token expired, attempting refresh");
            match refresh_access_token_with_issuer(ISSUER, &token_data.refresh_token).await {
                Ok(new_tokens) => {
                    token_data.access_token = new_tokens.access_token;
                    token_data.refresh_token = new_tokens.refresh_token;
                    if new_tokens.id_token.is_some() {
                        token_data.id_token = new_tokens.id_token;
                    }
                    token_data.expires_at = Utc::now()
                        + chrono::Duration::seconds(new_tokens.expires_in.unwrap_or(3600));
                    if token_data.account_id.is_none() {
                        token_data.account_id =
                            extract_account_id(&token_data, self.state.as_ref()).await;
                    }
                    self.cache.save(&token_data)?;
                    tracing::info!("Token refreshed successfully");
                    return Ok(token_data);
                }
                Err(e) => {
                    tracing::warn!("Token refresh failed, will re-authenticate: {}", e);
                    self.cache.clear();
                }
            }
        }

        tracing::info!("Starting OAuth flow for ChatGPT Codex");
        let token_data = perform_oauth_flow(self.state.as_ref()).await?;
        self.cache.save(&token_data)?;
        Ok(token_data)
    }
}

#[async_trait]
impl AuthProvider for ChatGptCodexAuthProvider {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        let token_data = self.get_valid_token().await?;
        Ok((
            "Authorization".to_string(),
            format!("Bearer {}", token_data.access_token),
        ))
    }
}
