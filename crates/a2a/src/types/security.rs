//! Security scheme types mapped from a2a.proto.
//!
//! SecurityScheme is represented as a tagged enum. Individual scheme variants
//! are fully typed. SecurityRequirement uses HashMap<String, Vec<String>>.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Security scheme (proto `SecurityScheme` oneof).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum SecurityScheme {
    #[serde(rename = "apiKey")]
    ApiKey(ApiKeySecurityScheme),
    #[serde(rename = "http")]
    Http(HttpAuthSecurityScheme),
    #[serde(rename = "oauth2")]
    OAuth2(OAuth2SecurityScheme),
    #[serde(rename = "openIdConnect")]
    OpenIdConnect(OpenIdConnectSecurityScheme),
    #[serde(rename = "mutualTls")]
    MutualTls(MutualTlsSecurityScheme),
}

/// API key authentication (proto `APIKeySecurityScheme`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiKeySecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub location: String,
    pub name: String,
}

/// HTTP authentication (proto `HTTPAuthSecurityScheme`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HttpAuthSecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub scheme: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bearer_format: Option<String>,
}

/// OAuth 2.0 authentication (proto `OAuth2SecurityScheme`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OAuth2SecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub flows: OAuthFlows,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub oauth2_metadata_url: Option<String>,
}

/// OpenID Connect authentication (proto `OpenIdConnectSecurityScheme`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenIdConnectSecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub open_id_connect_url: String,
}

/// Mutual TLS authentication (proto `MutualTlsSecurityScheme`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MutualTlsSecurityScheme {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// OAuth 2.0 flows (proto `OAuthFlows` oneof).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum OAuthFlows {
    AuthorizationCode(AuthorizationCodeOAuthFlow),
    ClientCredentials(ClientCredentialsOAuthFlow),
    DeviceCode(DeviceCodeOAuthFlow),
}

/// Authorization Code OAuth flow (proto `AuthorizationCodeOAuthFlow`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthorizationCodeOAuthFlow {
    pub authorization_url: String,
    pub token_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    pub scopes: HashMap<String, String>,
    #[serde(default)]
    pub pkce_required: bool,
}

/// Client Credentials OAuth flow (proto `ClientCredentialsOAuthFlow`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientCredentialsOAuthFlow {
    pub token_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub refresh_url: Option<String>,
    pub scopes: HashMap<String, String>,
}

/// Device Code OAuth flow (proto `DeviceCodeOAuthFlow`).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceCodeOAuthFlow {
    pub device_authorization_url: String,
    pub token_url: String,
    pub scopes: HashMap<String, String>,
}

/// Security requirement: maps scheme name to required scopes (proto `SecurityRequirement`).
pub type SecurityRequirement = HashMap<String, Vec<String>>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_http_auth_scheme_serde() {
        let scheme = SecurityScheme::Http(HttpAuthSecurityScheme {
            description: None,
            scheme: "Bearer".to_string(),
            bearer_format: Some("JWT".to_string()),
        });
        let json = serde_json::to_value(&scheme).unwrap();
        assert_eq!(json["type"], "http");
        assert_eq!(json["scheme"], "Bearer");
        assert_eq!(json["bearerFormat"], "JWT");
    }

    #[test]
    fn test_api_key_scheme_serde() {
        let scheme = SecurityScheme::ApiKey(ApiKeySecurityScheme {
            description: Some("API key auth".to_string()),
            location: "header".to_string(),
            name: "X-API-Key".to_string(),
        });
        let json = serde_json::to_value(&scheme).unwrap();
        assert_eq!(json["type"], "apiKey");
        assert_eq!(json["location"], "header");
    }
}
