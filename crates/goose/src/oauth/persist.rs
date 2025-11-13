use async_trait::async_trait;
use oauth2::{basic::BasicTokenType, EmptyExtraTokenFields, StandardTokenResponse};
use rmcp::transport::auth::{OAuthTokenResponse, TokenStore};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

use crate::config::Config;

/// Credentials stored for an OAuth provider
#[derive(Debug, Clone, Serialize, Deserialize)]
struct SerializableCredentials {
    client_id: String,
    token_response: StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>,
}

/// Token store implementation that uses goose's Config secret storage
pub struct ConfigTokenStore {
    name: String,
}

impl ConfigTokenStore {
    pub fn new(name: String) -> Self {
        Self { name }
    }

    fn secret_key(&self) -> String {
        format!("oauth_creds_{}", self.name)
    }
}

#[async_trait]
impl TokenStore for ConfigTokenStore {
    async fn load(
        &self,
    ) -> Result<Option<OAuthTokenResponse>, Box<dyn std::error::Error + Send + Sync>> {
        let config = Config::global();
        let key = self.secret_key();

        match config.get_secret::<SerializableCredentials>(&key) {
            Ok(credentials) => Ok(Some(credentials.token_response)),
            Err(crate::config::ConfigError::NotFound(_)) => Ok(None),
            Err(e) => Err(Box::new(e)),
        }
    }

    async fn save(
        &self,
        token: &OAuthTokenResponse,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = Config::global();
        let key = self.secret_key();

        let credentials = SerializableCredentials {
            client_id: "goose".to_string(),
            token_response: token.clone(),
        };

        config.set_secret(&key, &credentials)?;
        Ok(())
    }

    async fn clear(&self) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = Config::global();
        let key = self.secret_key();
        config.delete_secret(&key)?;
        Ok(())
    }
}

/// Create a TokenStore for use with AuthorizationManager
pub fn create_token_store(name: &str) -> Arc<dyn TokenStore> {
    Arc::new(ConfigTokenStore::new(name.to_string()))
}
