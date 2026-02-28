use thiserror::Error;

#[derive(Error, Debug)]
pub enum GooseClientError {
    #[error("Configuration error: {0}")]
    Config(String),

    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

    #[error("Server returned {status}: {message}")]
    Server { status: u16, message: String },

    #[error("Failed to parse response: {0}")]
    Deserialization(#[from] serde_json::Error),

    #[error("SSE stream error: {0}")]
    Stream(String),

    #[error("Authentication failed: invalid or missing X-Secret-Key")]
    Unauthorized,
}

pub type Result<T> = std::result::Result<T, GooseClientError>;
