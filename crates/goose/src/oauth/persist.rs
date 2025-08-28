use oauth2::{basic::BasicTokenType, EmptyExtraTokenFields, StandardTokenResponse};
use reqwest::IntoUrl;
use rmcp::transport::{auth::OAuthState, AuthError};
use serde::{Deserialize, Serialize};

use crate::config::Config;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SerializableCredentials {
    pub client_id: String,
    pub token_response: Option<StandardTokenResponse<EmptyExtraTokenFields, BasicTokenType>>,
}

fn secret_key(name: &str) -> String {
    format!("oauth_creds_{name}")
}

pub async fn save_credentials(
    name: &str,
    oauth_state: &OAuthState,
) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::global();
    let (client_id, token_response) = oauth_state.get_credentials().await?;

    let credentials = SerializableCredentials {
        client_id,
        token_response,
    };

    let value = serde_json::to_value(&credentials)?;
    let key = secret_key(name);
    config.set_secret(&key, value)?;

    Ok(())
}

async fn load_credentials(
    name: &str,
) -> Result<SerializableCredentials, Box<dyn std::error::Error>> {
    let config = Config::global();
    let key = secret_key(name);
    let credentials: SerializableCredentials = config.get_secret(&key)?;

    Ok(credentials)
}

pub fn clear_credentials(name: &str) -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::global();

    Ok(config.delete_secret(&secret_key(name))?)
}

pub async fn load_cached_state<U: IntoUrl>(
    base_url: U,
    name: &str,
) -> Result<OAuthState, AuthError> {
    let credentials = load_credentials(name)
        .await
        .map_err(|e| AuthError::InternalError(format!("Failed to load credentials: {}", e)))?;

    if let Some(token_response) = credentials.token_response {
        let mut oauth_state = OAuthState::new(base_url, None).await?;
        oauth_state
            .set_credentials(&credentials.client_id, token_response)
            .await?;
        Ok(oauth_state)
    } else {
        Err(AuthError::InternalError(
            "No token response in cached credentials".to_string(),
        ))
    }
}
