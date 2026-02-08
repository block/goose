use super::api_client::AuthProvider;
use anyhow::{Context, Result};
use async_trait::async_trait;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;

const COMMAND_CACHE_TTL: Duration = Duration::from_secs(300);

/// How the resolved token should be sent in the HTTP request.
#[derive(Debug, Clone)]
pub enum AuthHeaderStyle {
    /// Standard `Authorization: Bearer <token>` header.
    BearerToken,
    /// Custom header name (e.g. Anthropic's `x-api-key`).
    CustomHeader { header_name: String },
}

/// Executes a shell command at runtime and uses stdout as the auth token.
/// The result is cached with a 5-minute TTL.
pub struct CommandAuthProvider {
    command: String,
    header_style: AuthHeaderStyle,
    cache: Arc<RwLock<Option<(String, Instant)>>>,
}

impl CommandAuthProvider {
    pub fn new(command: String, header_style: AuthHeaderStyle) -> Self {
        Self {
            command,
            header_style,
            cache: Arc::new(RwLock::new(None)),
        }
    }

    async fn execute_command(&self) -> Result<String> {
        let output = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&self.command)
            .output()
            .await
            .with_context(|| format!("Failed to execute api_key_command: {}", self.command))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!(
                "api_key_command exited with status {}: {}",
                output.status,
                stderr.trim()
            );
        }

        let token = String::from_utf8(output.stdout)
            .context("api_key_command output is not valid UTF-8")?
            .trim()
            .to_string();

        if token.is_empty() {
            anyhow::bail!("api_key_command produced empty output");
        }

        Ok(token)
    }
}

#[async_trait]
impl AuthProvider for CommandAuthProvider {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        // Fast path: check if cache is still valid
        {
            let cache = self.cache.read().await;
            if let Some((ref token, ref timestamp)) = *cache {
                if timestamp.elapsed() < COMMAND_CACHE_TTL {
                    return Ok(header_pair(&self.header_style, token));
                }
            }
        }

        // Slow path: acquire write lock and double-check
        let mut cache = self.cache.write().await;
        if let Some((ref token, ref timestamp)) = *cache {
            if timestamp.elapsed() < COMMAND_CACHE_TTL {
                return Ok(header_pair(&self.header_style, token));
            }
        }

        let token = self.execute_command().await?;
        let pair = header_pair(&self.header_style, &token);
        *cache = Some((token, Instant::now()));
        Ok(pair)
    }
}

/// Reads an auth token from a file on each call.
/// If `field` is set, the file is parsed as JSON and the named field is extracted.
pub struct FileAuthProvider {
    path: String,
    field: Option<String>,
    header_style: AuthHeaderStyle,
}

impl FileAuthProvider {
    pub fn new(path: String, field: Option<String>, header_style: AuthHeaderStyle) -> Self {
        Self {
            path,
            field,
            header_style,
        }
    }
}

#[async_trait]
impl AuthProvider for FileAuthProvider {
    async fn get_auth_header(&self) -> Result<(String, String)> {
        let expanded = shellexpand::tilde(&self.path).into_owned();
        let content = tokio::fs::read_to_string(&expanded)
            .await
            .with_context(|| format!("Failed to read api_key_file: {}", self.path))?;

        let token = match &self.field {
            Some(field_name) => {
                let json: serde_json::Value = serde_json::from_str(&content)
                    .with_context(|| format!("api_key_file is not valid JSON: {}", self.path))?;
                json.get(field_name)
                    .and_then(|v| v.as_str())
                    .ok_or_else(|| {
                        anyhow::anyhow!(
                            "Field '{}' not found or not a string in {}",
                            field_name,
                            self.path
                        )
                    })?
                    .to_string()
            }
            None => content.trim().to_string(),
        };

        if token.is_empty() {
            anyhow::bail!("api_key_file produced empty token: {}", self.path);
        }

        Ok(header_pair(&self.header_style, &token))
    }
}

fn header_pair(style: &AuthHeaderStyle, token: &str) -> (String, String) {
    match style {
        AuthHeaderStyle::BearerToken => {
            ("Authorization".to_string(), format!("Bearer {}", token))
        }
        AuthHeaderStyle::CustomHeader { header_name } => {
            (header_name.clone(), token.to_string())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_command_auth_provider_echo() {
        let provider =
            CommandAuthProvider::new("echo test-token".to_string(), AuthHeaderStyle::BearerToken);
        let (header, value) = provider.get_auth_header().await.unwrap();
        assert_eq!(header, "Authorization");
        assert_eq!(value, "Bearer test-token");
    }

    #[tokio::test]
    async fn test_command_auth_provider_caching() {
        let provider = CommandAuthProvider::new(
            "echo cached-token".to_string(),
            AuthHeaderStyle::BearerToken,
        );

        let (_, v1) = provider.get_auth_header().await.unwrap();
        let (_, v2) = provider.get_auth_header().await.unwrap();
        assert_eq!(v1, v2);
        assert_eq!(v1, "Bearer cached-token");
    }

    #[tokio::test]
    async fn test_command_auth_provider_failure() {
        let provider = CommandAuthProvider::new(
            "exit 1".to_string(),
            AuthHeaderStyle::BearerToken,
        );
        let result = provider.get_auth_header().await;
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("exited with status"));
    }

    #[tokio::test]
    async fn test_command_auth_provider_custom_header() {
        let provider = CommandAuthProvider::new(
            "echo my-key".to_string(),
            AuthHeaderStyle::CustomHeader {
                header_name: "x-api-key".to_string(),
            },
        );
        let (header, value) = provider.get_auth_header().await.unwrap();
        assert_eq!(header, "x-api-key");
        assert_eq!(value, "my-key");
    }

    #[tokio::test]
    async fn test_file_auth_provider_plain_text() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("token.txt");
        std::fs::write(&file_path, "file-token\n").unwrap();

        let provider = FileAuthProvider::new(
            file_path.to_string_lossy().to_string(),
            None,
            AuthHeaderStyle::BearerToken,
        );
        let (header, value) = provider.get_auth_header().await.unwrap();
        assert_eq!(header, "Authorization");
        assert_eq!(value, "Bearer file-token");
    }

    #[tokio::test]
    async fn test_file_auth_provider_json_field() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("creds.json");
        std::fs::write(
            &file_path,
            r#"{"access_token": "json-token", "expires": 3600}"#,
        )
        .unwrap();

        let provider = FileAuthProvider::new(
            file_path.to_string_lossy().to_string(),
            Some("access_token".to_string()),
            AuthHeaderStyle::BearerToken,
        );
        let (header, value) = provider.get_auth_header().await.unwrap();
        assert_eq!(header, "Authorization");
        assert_eq!(value, "Bearer json-token");
    }

    #[tokio::test]
    async fn test_file_auth_provider_missing_field() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("creds.json");
        std::fs::write(&file_path, r#"{"other_field": "value"}"#).unwrap();

        let provider = FileAuthProvider::new(
            file_path.to_string_lossy().to_string(),
            Some("access_token".to_string()),
            AuthHeaderStyle::BearerToken,
        );
        let result = provider.get_auth_header().await;
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Field 'access_token' not found"));
    }
}
