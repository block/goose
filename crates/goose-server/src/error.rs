use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required environment variable: {env_var}")]
    MissingEnvVar { env_var: String },
    #[error("Configuration error: {0}")]
    Other(#[from] config::ConfigError),
}

// Helper function to format environment variable names
pub(crate) fn to_env_var(field_path: &str) -> String {
    // Handle nested fields by converting dots to double underscores
    // If the field is in the provider object, we need to prefix it appropriately
    let normalized_path = if field_path == "type" {
        "provider.type".to_string()
    } else if field_path.starts_with("provider.") {
        field_path.to_string()
    } else {
        format!("provider.{}", field_path)
    };

    format!(
        "GOOSE_{}",
        normalized_path.replace('.', "__").to_uppercase()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_env_var_conversion() {
        assert_eq!(to_env_var("type"), "GOOSE_PROVIDER__TYPE");
        assert_eq!(to_env_var("api_key"), "GOOSE_PROVIDER__API_KEY");
        assert_eq!(to_env_var("provider.host"), "GOOSE_PROVIDER__HOST");
        assert_eq!(to_env_var("provider.api_key"), "GOOSE_PROVIDER__API_KEY");
    }
}
