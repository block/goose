use thiserror::Error;

/// Errors that can occur while resolving an `mcp://` discovery URI.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("invalid mcp discovery URI: {0}")]
    InvalidUri(String),

    #[error("no MCP server could be discovered for host {0}")]
    NotFound(String),

    #[error("manifest at {url} is malformed: {reason}")]
    MalformedManifest { url: String, reason: String },

    #[error(
        "endpoint host {endpoint_host} is not the discovery host {discovery_host} or a subdomain of it"
    )]
    EndpointHostMismatch {
        endpoint_host: String,
        discovery_host: String,
    },

    #[error("insecure endpoint {0}: discovery requires https")]
    InsecureEndpoint(String),

    #[error("unsupported transport {0:?}: only \"http\" is supported")]
    UnsupportedTransport(String),

    #[error("manifest signature verification failed: {0}")]
    SignatureVerification(String),

    #[error("network error talking to {url}: {source}")]
    Network {
        url: String,
        #[source]
        source: anyhow::Error,
    },
}

pub type Result<T> = std::result::Result<T, DiscoveryError>;
