use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DatabricksAuth {
    Token(String),
    OAuth {
        host: String,
        client_id: String,
        redirect_url: String,
        scopes: Vec<String>,
    },
}

impl DatabricksAuth {
    pub fn oauth(host: String) -> Self {
        Self::OAuth {
            host,
            client_id: DEFAULT_CLIENT_ID.to_string(),
            redirect_url: DEFAULT_REDIRECT_URL.to_string(),
            scopes: DEFAULT_SCOPES.iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn token(token: String) -> Self {
        Self::Token(token)
    }
}

pub(super) struct DatabricksAuthProvider {
    pub auth: DatabricksAuth,
    pub token_cache: Arc<Mutex<Option<String>>>,
}

#[async_trait]
impl AuthProvider for DatabricksAuthProvider {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        let token = match &self.auth {
            DatabricksAuth::Token(original) => {
                let cached = self.token_cache.lock().unwrap().clone();
                match cached {
                    Some(t) => t,
                    None => {
                        // Cache was cleared by refresh_credentials(); re-read
                        // from config which may have a sidecar-rotated token.
                        // Fall back to the constructor-provided token if config
                        // lookup fails (e.g. from_params usage).
                        let fresh = crate::config::Config::global()
                            .get_secret::<String>("DATABRICKS_TOKEN")
                            .unwrap_or_else(|_| original.clone());
                        *self.token_cache.lock().unwrap() = Some(fresh.clone());
                        fresh
                    }
                }
            }
            DatabricksAuth::OAuth {
                host,
                client_id,
                redirect_url,
                scopes,
            } => oauth::get_oauth_token_async(host, client_id, redirect_url, scopes).await?,
        };
        Ok(("Authorization".to_string(), format!("Bearer {}", token)))
    }
}
