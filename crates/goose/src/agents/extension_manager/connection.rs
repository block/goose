use super::helpers::resolve_timeout;
use crate::agents::extension::{ExtensionError, ExtensionResult, ProcessExit};
use crate::agents::mcp_client::{GooseMcpClientCapabilities, McpClient, McpClientTrait};
use crate::agents::types::SharedProvider;
use crate::config::search_path::SearchPaths;
use crate::oauth::{oauth_flow, GooseCredentialStore};
use crate::subprocess::configure_subprocess;
use axum::http::{HeaderMap, HeaderName, HeaderValue};
use rmcp::service::ClientInitializeError;
use rmcp::transport::auth::{AuthClient, CredentialStore};
use rmcp::transport::streamable_http_client::{
    StreamableHttpClientTransportConfig, StreamableHttpError,
};
use rmcp::transport::{DynamicTransportError, StreamableHttpClientTransport, TokioChildProcess};
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::Duration;
use tokio::io::AsyncReadExt;
use tokio::process::Command;
use tracing::warn;

const GOOSE_USER_AGENT: reqwest::header::HeaderValue =
    reqwest::header::HeaderValue::from_static(concat!("goose/", env!("CARGO_PKG_VERSION")));

pub async fn child_process_client(
    mut command: Command,
    timeout: &Option<u64>,
    provider: SharedProvider,
    working_dir: &PathBuf,
    docker_container: Option<String>,
    client_name: String,
    capabilities: GooseMcpClientCapabilities,
) -> ExtensionResult<McpClient> {
    configure_subprocess(&mut command);

    if let Ok(path) = SearchPaths::builder().path() {
        command.env("PATH", path);
    }

    if working_dir.exists() && working_dir.is_dir() {
        tracing::info!("Setting MCP process working directory: {:?}", working_dir);
        command.current_dir(working_dir);
    } else {
        tracing::warn!(
            "Working directory doesn't exist or isn't a directory: {:?}",
            working_dir
        );
    }

    let (transport, mut stderr) = TokioChildProcess::builder(command)
        .stderr(Stdio::piped())
        .spawn()?;
    let mut stderr = stderr.take().ok_or_else(|| {
        ExtensionError::SetupError("failed to attach child process stderr".to_owned())
    })?;

    let stderr_task = tokio::spawn(async move {
        let mut all_stderr = Vec::new();
        stderr.read_to_end(&mut all_stderr).await?;
        Ok::<String, std::io::Error>(String::from_utf8_lossy(&all_stderr).into())
    });

    let client_result = McpClient::connect_with_container(
        transport,
        Duration::from_secs(resolve_timeout(*timeout)),
        provider,
        docker_container,
        client_name,
        capabilities,
        working_dir.clone(),
    )
    .await;

    match client_result {
        Ok(client) => Ok(client),
        Err(error) => {
            let error_task_out = stderr_task.await?;
            Err::<McpClient, ExtensionError>(match error_task_out {
                Ok(stderr_content) => ProcessExit::new(stderr_content, error).into(),
                Err(e) => e.into(),
            })
        }
    }
}

/// Retry with OAuth for typed auth challenges and wrapped bare HTTP 401 responses.
pub fn should_attempt_oauth_fallback(res: &Result<McpClient, ClientInitializeError>) -> bool {
    let Err(ClientInitializeError::TransportError {
        error: DynamicTransportError { error, .. },
        ..
    }) = res
    else {
        return false;
    };

    if let Some(http_err) = error.downcast_ref::<StreamableHttpError<reqwest::Error>>() {
        return match http_err {
            StreamableHttpError::AuthRequired(_) => true,
            StreamableHttpError::UnexpectedServerResponse(body) => body.starts_with("HTTP 401"),
            _ => false,
        };
    }

    #[cfg(unix)]
    if let Some(http_err) = error
        .downcast_ref::<StreamableHttpError<rmcp::transport::common::unix_socket::UnixSocketError>>(
        )
    {
        return match http_err {
            StreamableHttpError::AuthRequired(_) => true,
            StreamableHttpError::UnexpectedServerResponse(body) => body.starts_with("HTTP 401"),
            _ => false,
        };
    }

    error
        .to_string()
        .contains("unexpected server response: HTTP 401")
}

#[allow(clippy::too_many_arguments)]
pub async fn connect_with_auth(
    auth_manager: rmcp::transport::AuthorizationManager,
    uri: &str,
    timeout: Duration,
    provider: SharedProvider,
    client_name: String,
    capabilities: GooseMcpClientCapabilities,
    roots_dir: &std::path::Path,
) -> ExtensionResult<Box<dyn McpClientTrait>> {
    let mut auth_headers = HeaderMap::new();
    auth_headers.insert(reqwest::header::USER_AGENT, GOOSE_USER_AGENT);
    #[allow(unused_mut)]
    let mut auth_client_builder = reqwest::Client::builder().default_headers(auth_headers);
    #[cfg(target_os = "linux")]
    {
        auth_client_builder = auth_client_builder.tcp_user_timeout(Some(timeout));
    }
    let auth_http_client = auth_client_builder
        .build()
        .map_err(|_| ExtensionError::ConfigError("could not construct http client".to_string()))?;
    let auth_client = AuthClient::new(auth_http_client, auth_manager);
    let transport = StreamableHttpClientTransport::with_client(
        auth_client,
        StreamableHttpClientTransportConfig::with_uri(uri),
    );
    Ok(Box::new(
        McpClient::connect(
            transport,
            timeout,
            provider,
            client_name,
            capabilities,
            roots_dir.to_path_buf(),
        )
        .await?,
    ))
}

#[allow(clippy::too_many_arguments)]
pub async fn create_streamable_http_client(
    uri: &str,
    timeout: Option<u64>,
    headers: &HashMap<String, String>,
    name: &str,
    socket: Option<&str>,
    provider: SharedProvider,
    client_name: String,
    capabilities: GooseMcpClientCapabilities,
    roots_dir: &std::path::Path,
) -> ExtensionResult<Box<dyn McpClientTrait>> {
    #[cfg(unix)]
    if let Some(socket_path) = socket {
        return create_unix_socket_http_client(
            uri,
            timeout,
            headers,
            name,
            socket_path,
            provider,
            client_name,
            capabilities,
            roots_dir,
        )
        .await;
    }
    #[cfg(not(unix))]
    if socket.is_some() {
        return Err(ExtensionError::ConfigError(
            "Unix domain socket transport is not supported on this platform".to_string(),
        ));
    }

    let mut default_headers = HeaderMap::new();

    default_headers.insert(reqwest::header::USER_AGENT, GOOSE_USER_AGENT);

    for (key, value) in headers {
        default_headers.insert(
            HeaderName::try_from(key)
                .map_err(|_| ExtensionError::ConfigError(format!("invalid header: {}", key)))?,
            value.parse().map_err(|_| {
                ExtensionError::ConfigError(format!("invalid header value: {}", key))
            })?,
        );
    }

    let timeout_duration = Duration::from_secs(resolve_timeout(timeout));

    #[allow(unused_mut)]
    let mut http_client_builder = reqwest::Client::builder().default_headers(default_headers);
    #[cfg(target_os = "linux")]
    {
        http_client_builder = http_client_builder.tcp_user_timeout(Some(timeout_duration));
    }
    let http_client = http_client_builder
        .build()
        .map_err(|_| ExtensionError::ConfigError("could not construct http client".to_string()))?;

    let transport = StreamableHttpClientTransport::with_client(
        http_client,
        StreamableHttpClientTransportConfig::with_uri(uri),
    );

    // If we have stored OAuth credentials, try refreshing and connecting directly.
    // This avoids the unnecessary 401 → browser re-auth cycle on every new session.
    let credential_store = GooseCredentialStore::new(name.to_string());
    if credential_store.load().await.is_ok_and(|c| c.is_some()) {
        match oauth_flow(&uri.to_string(), &name.to_string()).await {
            Ok(auth_manager) => {
                return connect_with_auth(
                    auth_manager,
                    uri,
                    timeout_duration,
                    provider,
                    client_name,
                    capabilities,
                    roots_dir,
                )
                .await;
            }
            Err(e) => {
                warn!(
                    "[OAuth:{}] Proactive refresh failed: {}, falling back to unauthenticated attempt",
                    name, e
                );
            }
        }
    }

    let client_res = McpClient::connect(
        transport,
        timeout_duration,
        provider.clone(),
        client_name.clone(),
        capabilities.clone(),
        roots_dir.to_path_buf(),
    )
    .await;

    if should_attempt_oauth_fallback(&client_res) {
        match oauth_flow(&uri.to_string(), &name.to_string()).await {
            Ok(auth_manager) => {
                connect_with_auth(
                    auth_manager,
                    uri,
                    timeout_duration,
                    provider,
                    client_name,
                    capabilities,
                    roots_dir,
                )
                .await
            }
            Err(_) => Ok(Box::new(client_res?)),
        }
    } else {
        Ok(Box::new(client_res?))
    }
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
pub async fn create_unix_socket_http_client(
    uri: &str,
    timeout: Option<u64>,
    headers: &HashMap<String, String>,
    name: &str,
    socket_path: &str,
    provider: SharedProvider,
    client_name: String,
    capabilities: GooseMcpClientCapabilities,
    roots_dir: &std::path::Path,
) -> ExtensionResult<Box<dyn McpClientTrait>> {
    use rmcp::transport::UnixSocketHttpClient;

    let unix_client = UnixSocketHttpClient::new(socket_path, uri);

    let mut custom_headers = std::collections::HashMap::<HeaderName, HeaderValue>::new();

    custom_headers.insert(
        HeaderName::from_static("user-agent"),
        GOOSE_USER_AGENT
            .to_str()
            .unwrap_or("goose")
            .parse()
            .unwrap_or_else(|_| HeaderValue::from_static("goose")),
    );

    for (key, value) in headers {
        let header_name = HeaderName::try_from(key)
            .map_err(|_| ExtensionError::ConfigError(format!("invalid header: {}", key)))?;
        let header_value = value
            .parse::<HeaderValue>()
            .map_err(|_| ExtensionError::ConfigError(format!("invalid header value: {}", key)))?;
        custom_headers.insert(header_name, header_value);
    }

    let config = StreamableHttpClientTransportConfig::with_uri(uri).custom_headers(custom_headers);
    let transport = StreamableHttpClientTransport::with_client(unix_client, config);

    let timeout_duration = Duration::from_secs(resolve_timeout(timeout));

    let client_res = McpClient::connect(
        transport,
        timeout_duration,
        provider.clone(),
        client_name.clone(),
        capabilities.clone(),
        roots_dir.to_path_buf(),
    )
    .await;

    if should_attempt_oauth_fallback(&client_res) {
        tracing::warn!(
            "Extension '{}' returned 401 over Unix domain socket transport; \
             OAuth is not supported for UDS connections",
            name,
        );
    }
    Ok(Box::new(client_res?))
}
