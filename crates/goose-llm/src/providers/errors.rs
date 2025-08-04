use thiserror::Error;

#[derive(Error, Debug, uniffi::Error)]
pub enum ProviderError {
    #[error("Authentication error: {0}")]
    Authentication(String),

    #[error("Context length exceeded: {0}")]
    ContextLengthExceeded(String),

    #[error("Rate limit exceeded: {0}")]
    RateLimitExceeded(String),

    #[error("Server error: {0}")]
    ServerError(String),

    #[error("Request failed: {0}")]
    RequestFailed(String),

    #[error("Execution error: {0}")]
    ExecutionError(String),

    #[error("Usage data error: {0}")]
    UsageError(String),

    #[error("Invalid response: {0}")]
    ResponseParseError(String),
}

impl From<anyhow::Error> for ProviderError {
    fn from(error: anyhow::Error) -> Self {
        ProviderError::ExecutionError(error.to_string())
    }
}

impl From<reqwest::Error> for ProviderError {
    fn from(error: reqwest::Error) -> Self {
        ProviderError::ExecutionError(error.to_string())
    }
}

#[derive(serde::Deserialize, Debug)]
pub struct OpenAIError {
    #[serde(deserialize_with = "code_as_string")]
    pub code: Option<String>,
    pub message: Option<String>,
    #[serde(rename = "type")]
    pub error_type: Option<String>,
}

fn code_as_string<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    use std::fmt;

    use serde::de::{self, Visitor};

    struct CodeVisitor;

    impl<'de> Visitor<'de> for CodeVisitor {
        type Value = Option<String>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("a string, a number, null, or none for the code field")
        }

        fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_u64<E>(self, value: u64) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(Some(value.to_string()))
        }

        fn visit_none<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_unit<E>(self) -> Result<Self::Value, E>
        where
            E: de::Error,
        {
            Ok(None)
        }

        fn visit_some<D>(self, deserializer: D) -> Result<Self::Value, D::Error>
        where
            D: serde::Deserializer<'de>,
        {
            deserializer.deserialize_any(CodeVisitor)
        }
    }

    deserializer.deserialize_option(CodeVisitor)
}

impl OpenAIError {
    pub fn is_context_length_exceeded(&self) -> bool {
        if let Some(code) = &self.code {
            code == "context_length_exceeded" || code == "string_above_max_length"
        } else {
            false
        }
    }
}

impl std::fmt::Display for OpenAIError {
    /// Format the error for display.
    /// E.g. {"message": "Invalid API key", "code": "invalid_api_key", "type": "client_error"}
    /// would be formatted as "Invalid API key (code: invalid_api_key, type: client_error)"
    /// and {"message": "Foo"} as just "Foo", etc.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if let Some(message) = &self.message {
            write!(f, "{}", message)?;
        }
        let mut in_parenthesis = false;
        if let Some(code) = &self.code {
            write!(f, " (code: {}", code)?;
            in_parenthesis = true;
        }
        if let Some(typ) = &self.error_type {
            if in_parenthesis {
                write!(f, ", type: {}", typ)?;
            } else {
                write!(f, " (type: {}", typ)?;
                in_parenthesis = true;
            }
        }
        if in_parenthesis {
            write!(f, ")")?;
        }
        Ok(())
    }
}
