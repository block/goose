//! JSON Schema validation for MCP secrets configuration.

use jsonschema::{Draft, JSONSchema};
use serde_json::Value;
use std::sync::LazyLock;

use crate::error::SecretError;

/// The JSON schema for validating MCP secrets configuration.
/// This is embedded at compile time from the schema file.
static SCHEMA_JSON: &str = include_str!("../mcp_secrets.schema.json");

/// Compiled JSON schema validator, initialized once and reused.
static SCHEMA_VALIDATOR: LazyLock<JSONSchema> = LazyLock::new(|| {
    let schema: Value = serde_json::from_str(SCHEMA_JSON)
        .expect("Embedded schema should be valid JSON");
    
    JSONSchema::options()
        .with_draft(Draft::Draft7)
        .compile(&schema)
        .expect("Embedded schema should be valid JSON Schema")
});

/// Validates a secrets configuration array against the JSON schema.
///
/// # Arguments
/// * `secrets_json` - The JSON value representing the secrets array to validate
///
/// # Returns
/// * `Ok(())` if validation passes
/// * `Err(SecretError)` if validation fails with detailed error messages
///
/// # Example
/// ```rust
/// use serde_json::json;
/// use goose_secure_store::validation::validate_secrets_config;
///
/// let valid_config = json!([
///     {
///         "name": "API_KEY",
///         "description": "My API key",
///         "acquisition": {
///             "method": "prompt",
///             "prompt_message": "Enter your API key"
///         }
///     }
/// ]);
///
/// assert!(validate_secrets_config(&valid_config).is_ok());
/// ```
pub fn validate_secrets_config(secrets_json: &Value) -> Result<(), SecretError> {
    let result = SCHEMA_VALIDATOR.validate(secrets_json);
    
    match result {
        Ok(_) => Ok(()),
        Err(errors) => {
            let error_messages: Vec<String> = errors
                .map(|error| {
                    format!("Validation error at '{}': {}", 
                        error.instance_path, 
                        error
                    )
                })
                .collect();
            
            Err(SecretError::InvalidParameters(format!(
                "Schema validation failed:\n{}",
                error_messages.join("\n")
            )))
        }
    }
}

/// Validates a single secret configuration object.
///
/// This is a convenience function for validating individual secret objects
/// by wrapping them in an array and validating against the schema.
pub fn validate_single_secret(secret_json: &Value) -> Result<(), SecretError> {
    let array = serde_json::json!([secret_json]);
    validate_secrets_config(&array)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_valid_prompt_secret() {
        let config = json!([
            {
                "name": "API_KEY",
                "description": "Test API key",
                "acquisition": {
                    "method": "prompt",
                    "prompt_message": "Enter your API key"
                }
            }
        ]);
        
        assert!(validate_secrets_config(&config).is_ok());
    }

    #[test]
    fn test_valid_prompt_secret_without_message() {
        let config = json!([
            {
                "name": "API_KEY",
                "description": "Test API key",
                "acquisition": {
                    "method": "prompt"
                }
            }
        ]);
        
        assert!(validate_secrets_config(&config).is_ok());
    }

    #[test]
    fn test_invalid_secret_name_pattern() {
        let config = json!([
            {
                "name": "invalid-name",  // Should be uppercase with underscores
                "description": "Test API key",
                "acquisition": {
                    "method": "prompt"
                }
            }
        ]);
        
        let result = validate_secrets_config(&config);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Check that validation failed - the exact error message may vary
        assert!(error_msg.contains("Schema validation failed") || error_msg.contains("Validation error"));
    }

    #[test]
    fn test_missing_required_fields() {
        let config = json!([
            {
                "name": "API_KEY"
                // Missing description and acquisition
            }
        ]);
        
        let result = validate_secrets_config(&config);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("required"));
    }

    #[test]
    fn test_invalid_acquisition_method() {
        let config = json!([
            {
                "name": "API_KEY",
                "description": "Test API key",
                "acquisition": {
                    "method": "invalid_method"
                }
            }
        ]);
        
        let result = validate_secrets_config(&config);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        // Check that validation failed - the exact error message may vary
        assert!(error_msg.contains("Schema validation failed") || error_msg.contains("Validation error"));
    }

    #[test]
    fn test_oauth2_validation() {
        let config = json!([
            {
                "name": "OAUTH_TOKEN",
                "description": "OAuth access token",
                "acquisition": {
                    "method": "oauth2",
                    "authorization_url": "https://example.com/oauth/authorize",
                    "token_url": "https://example.com/oauth/token",
                    "client_id": "my-client-id",
                    "scopes": ["read", "write"]
                }
            }
        ]);
        
        assert!(validate_secrets_config(&config).is_ok());
    }

    #[test]
    fn test_oauth2_missing_required_fields() {
        let config = json!([
            {
                "name": "OAUTH_TOKEN",
                "description": "OAuth access token",
                "acquisition": {
                    "method": "oauth2"
                    // Missing required fields
                }
            }
        ]);
        
        let result = validate_secrets_config(&config);
        assert!(result.is_err());
    }

    #[test]
    fn test_command_validation() {
        let config = json!([
            {
                "name": "SESSION_TOKEN",
                "description": "AWS session token",
                "acquisition": {
                    "method": "command",
                    "command": "aws sts get-session-token --output text --query Credentials.SessionToken"
                }
            }
        ]);
        
        assert!(validate_secrets_config(&config).is_ok());
    }

    #[test]
    fn test_single_secret_validation() {
        let secret = json!({
            "name": "API_KEY",
            "description": "Test API key",
            "acquisition": {
                "method": "prompt"
            }
        });
        
        assert!(validate_single_secret(&secret).is_ok());
    }

    #[test]
    fn test_empty_array_is_valid() {
        let config = json!([]);
        assert!(validate_secrets_config(&config).is_ok());
    }

    #[test]
    fn test_multiple_secrets() {
        let config = json!([
            {
                "name": "API_KEY",
                "description": "API key",
                "acquisition": {
                    "method": "prompt"
                }
            },
            {
                "name": "OAUTH_TOKEN",
                "description": "OAuth token",
                "acquisition": {
                    "method": "oauth2",
                    "authorization_url": "https://example.com/oauth/authorize",
                    "token_url": "https://example.com/oauth/token",
                    "client_id": "client-id"
                }
            }
        ]);
        
        assert!(validate_secrets_config(&config).is_ok());
    }
}
