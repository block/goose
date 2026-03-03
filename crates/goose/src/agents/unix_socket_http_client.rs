use std::collections::HashMap;
use std::sync::Arc;

use bytes::Bytes;
use futures::stream::BoxStream;
use futures::StreamExt;
use http::{HeaderName, HeaderValue, Method, Request, StatusCode};
use http_body_util::{BodyExt, Full};
use hyper::body::Incoming;
use hyper_util::rt::TokioIo;
use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
use rmcp::transport::common::http_header::{
    EVENT_STREAM_MIME_TYPE, HEADER_LAST_EVENT_ID, HEADER_MCP_PROTOCOL_VERSION, HEADER_SESSION_ID,
    JSON_MIME_TYPE,
};
use rmcp::transport::streamable_http_client::{
    AuthRequiredError, InsufficientScopeError, StreamableHttpClient, StreamableHttpError,
    StreamableHttpPostResponse,
};
use sse_stream::SseStream;
use tokio::net::UnixStream;

#[derive(Debug, thiserror::Error)]
pub enum UnixSocketError {
    #[error("hyper error: {0}")]
    Hyper(#[from] hyper::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("HTTP error: {0}")]
    Http(#[from] http::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

#[derive(Clone, Debug)]
pub struct UnixSocketHttpClient {
    socket_path: Arc<str>,
    default_headers: HashMap<HeaderName, HeaderValue>,
}

impl UnixSocketHttpClient {
    pub fn new(
        uri: &str,
        raw_socket_path: &str,
        mut default_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Self {
        // hyper over a raw Unix socket does not set Host automatically (unlike reqwest over TCP).
        // Derive it from the URI so Envoy can route to the correct upstream cluster.
        // An explicitly provided Host in default_headers takes precedence.
        if let Ok(parsed) = uri.parse::<http::Uri>() {
            if let Some(authority) = parsed.authority() {
                if let Ok(val) = HeaderValue::from_str(authority.as_str()) {
                    default_headers.entry(http::header::HOST).or_insert(val);
                }
            }
        }
        Self {
            socket_path: resolve_socket_path(raw_socket_path).into(),
            default_headers,
        }
    }
}

/// Converts the `@`-prefixed abstract socket notation to the null-byte prefix
/// expected by the Linux kernel. Filesystem socket paths are returned unchanged.
fn resolve_socket_path(raw: &str) -> String {
    if let Some(name) = raw.strip_prefix('@') {
        format!("\0{name}")
    } else {
        raw.to_string()
    }
}

async fn connect_unix(socket_path: &str) -> Result<UnixStream, std::io::Error> {
    #[cfg(target_os = "linux")]
    if let Some(abstract_name) = socket_path.strip_prefix('\0') {
        use std::os::linux::net::SocketAddrExt;
        let addr = std::os::unix::net::SocketAddr::from_abstract_name(abstract_name)?;
        // tokio::net::UnixStream has no connect_addr; use std then convert
        let std_stream = std::os::unix::net::UnixStream::connect_addr(&addr)?;
        std_stream.set_nonblocking(true)?;
        return UnixStream::from_std(std_stream);
    }

    UnixStream::connect(socket_path).await
}

/// Opens a new Unix socket connection and sends the HTTP request.
/// One connection per request — simple and correct. The sidecar proxy
/// handles connection pooling on its end if needed.
async fn send_http_request(
    socket_path: &str,
    request: Request<Full<Bytes>>,
) -> Result<http::Response<Incoming>, UnixSocketError> {
    let stream = connect_unix(socket_path).await?;
    let io = TokioIo::new(stream);
    let (mut sender, conn) = hyper::client::conn::http1::handshake(io).await?;

    tokio::spawn(async move {
        if let Err(e) = conn.await {
            tracing::warn!("unix socket HTTP/1.1 connection error: {e}");
        }
    });

    Ok(sender.send_request(request).await?)
}

/// Extracts the `scope=` parameter from a `WWW-Authenticate` header value.
/// Mirrors the private helper in rmcp's reqwest implementation.
fn extract_scope_from_header(header: &str) -> Option<String> {
    let header_lowercase = header.to_ascii_lowercase();
    let scope_key = "scope=";
    let pos = header_lowercase.find(scope_key)?;
    let value_slice = header.get(pos + scope_key.len()..)?;
    if let Some(stripped) = value_slice.strip_prefix('"') {
        let end = stripped.find('"')?;
        stripped.get(..end).map(str::to_string)
    } else {
        let end = value_slice
            .find(|c: char| c == ',' || c == ';' || c.is_whitespace())
            .unwrap_or(value_slice.len());
        if end > 0 {
            value_slice.get(..end).map(str::to_string)
        } else {
            None
        }
    }
}

impl StreamableHttpClient for UnixSocketHttpClient {
    type Error = UnixSocketError;

    async fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
        custom_headers: HashMap<HeaderName, HeaderValue>,
    ) -> Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>> {
        let json_body = serde_json::to_string(&message)
            .map_err(|e| StreamableHttpError::Client(UnixSocketError::Json(e)))?;

        let mut builder = Request::builder()
            .method(Method::POST)
            .uri(uri.as_ref())
            .header(http::header::CONTENT_TYPE, JSON_MIME_TYPE)
            .header(
                http::header::ACCEPT,
                format!("{EVENT_STREAM_MIME_TYPE}, {JSON_MIME_TYPE}"),
            );

        for (name, value) in &self.default_headers {
            builder = builder.header(name.clone(), value.clone());
        }

        if let Some(auth) = auth_header {
            builder = builder.header(http::header::AUTHORIZATION, format!("Bearer {auth}"));
        }

        let reserved = [
            http::header::ACCEPT.as_str(),
            HEADER_SESSION_ID,
            HEADER_MCP_PROTOCOL_VERSION,
            HEADER_LAST_EVENT_ID,
        ];
        for (name, value) in custom_headers {
            if reserved
                .iter()
                .any(|&r| name.as_str().eq_ignore_ascii_case(r))
            {
                return Err(StreamableHttpError::ReservedHeaderConflict(
                    name.to_string(),
                ));
            }
            builder = builder.header(name, value);
        }

        if let Some(sid) = session_id {
            builder = builder.header(HEADER_SESSION_ID, sid.as_ref());
        }

        let request = builder
            .body(Full::new(Bytes::from(json_body)))
            .map_err(|e| StreamableHttpError::Client(UnixSocketError::Http(e)))?;

        let response = send_http_request(&self.socket_path, request)
            .await
            .map_err(StreamableHttpError::Client)?;

        let status = response.status();

        if status == StatusCode::UNAUTHORIZED {
            if let Some(header) = response.headers().get(http::header::WWW_AUTHENTICATE) {
                let www_authenticate_header = header.to_str().unwrap_or_default().to_string();
                return Err(StreamableHttpError::AuthRequired(AuthRequiredError {
                    www_authenticate_header,
                }));
            }
        }

        if status == StatusCode::FORBIDDEN {
            if let Some(header) = response.headers().get(http::header::WWW_AUTHENTICATE) {
                let header_str = header.to_str().unwrap_or_default();
                return Err(StreamableHttpError::InsufficientScope(
                    InsufficientScopeError {
                        www_authenticate_header: header_str.to_string(),
                        required_scope: extract_scope_from_header(header_str),
                    },
                ));
            }
        }

        if matches!(status, StatusCode::ACCEPTED | StatusCode::NO_CONTENT) {
            return Ok(StreamableHttpPostResponse::Accepted);
        }

        let session_id = response
            .headers()
            .get(HEADER_SESSION_ID)
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());

        let content_type = response.headers().get(http::header::CONTENT_TYPE).cloned();

        match content_type {
            Some(ref ct) if ct.as_bytes().starts_with(EVENT_STREAM_MIME_TYPE.as_bytes()) => {
                let sse_stream = SseStream::new(response.into_body()).boxed();
                Ok(StreamableHttpPostResponse::Sse(sse_stream, session_id))
            }
            Some(ref ct) if ct.as_bytes().starts_with(JSON_MIME_TYPE.as_bytes()) => {
                let body = response
                    .into_body()
                    .collect()
                    .await
                    .map_err(|e| StreamableHttpError::Client(UnixSocketError::Hyper(e)))?
                    .to_bytes();
                let message: ServerJsonRpcMessage = serde_json::from_slice(&body)
                    .map_err(|e| StreamableHttpError::Client(UnixSocketError::Json(e)))?;
                Ok(StreamableHttpPostResponse::Json(message, session_id))
            }
            _ => Err(StreamableHttpError::UnexpectedContentType(
                content_type.map(|ct| String::from_utf8_lossy(ct.as_bytes()).into_owned()),
            )),
        }
    }

    async fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_header: Option<String>,
    ) -> Result<(), StreamableHttpError<Self::Error>> {
        let mut builder = Request::builder()
            .method(Method::DELETE)
            .uri(uri.as_ref())
            .header(HEADER_SESSION_ID, session_id.as_ref());

        for (name, value) in &self.default_headers {
            builder = builder.header(name.clone(), value.clone());
        }

        if let Some(auth) = auth_header {
            builder = builder.header(http::header::AUTHORIZATION, format!("Bearer {auth}"));
        }

        let request = builder
            .body(Full::new(Bytes::new()))
            .map_err(|e| StreamableHttpError::Client(UnixSocketError::Http(e)))?;

        let response = send_http_request(&self.socket_path, request)
            .await
            .map_err(StreamableHttpError::Client)?;

        // 405 means the server doesn't support session deletion — treat as success
        if response.status() == StatusCode::METHOD_NOT_ALLOWED {
            return Ok(());
        }

        if !response.status().is_success() {
            return Err(StreamableHttpError::UnexpectedServerResponse(
                format!("delete_session returned {}", response.status()).into(),
            ));
        }

        Ok(())
    }

    async fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
    ) -> Result<
        BoxStream<'static, Result<sse_stream::Sse, sse_stream::Error>>,
        StreamableHttpError<Self::Error>,
    > {
        let mut builder = Request::builder()
            .method(Method::GET)
            .uri(uri.as_ref())
            .header(
                http::header::ACCEPT,
                format!("{EVENT_STREAM_MIME_TYPE}, {JSON_MIME_TYPE}"),
            )
            .header(HEADER_SESSION_ID, session_id.as_ref());

        for (name, value) in &self.default_headers {
            builder = builder.header(name.clone(), value.clone());
        }

        if let Some(last_id) = last_event_id {
            builder = builder.header(HEADER_LAST_EVENT_ID, last_id);
        }

        if let Some(auth) = auth_header {
            builder = builder.header(http::header::AUTHORIZATION, format!("Bearer {auth}"));
        }

        let request = builder
            .body(Full::new(Bytes::new()))
            .map_err(|e| StreamableHttpError::Client(UnixSocketError::Http(e)))?;

        let response = send_http_request(&self.socket_path, request)
            .await
            .map_err(StreamableHttpError::Client)?;

        if response.status() == StatusCode::METHOD_NOT_ALLOWED {
            return Err(StreamableHttpError::ServerDoesNotSupportSse);
        }

        if !response.status().is_success() {
            return Err(StreamableHttpError::UnexpectedServerResponse(
                format!("get_stream returned {}", response.status()).into(),
            ));
        }

        Ok(SseStream::new(response.into_body()).boxed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_abstract_socket() {
        assert_eq!(resolve_socket_path("@egress.sock"), "\0egress.sock");
    }

    #[test]
    fn test_resolve_filesystem_socket() {
        assert_eq!(
            resolve_socket_path("/var/run/envoy.sock"),
            "/var/run/envoy.sock"
        );
    }

    #[test]
    fn test_resolve_empty_abstract() {
        assert_eq!(resolve_socket_path("@"), "\0");
    }

    #[test]
    fn test_extract_scope_quoted() {
        assert_eq!(
            extract_scope_from_header(r#"Bearer realm="example", scope="read write""#),
            Some("read write".to_string())
        );
    }

    #[test]
    fn test_extract_scope_unquoted() {
        assert_eq!(
            extract_scope_from_header("Bearer scope=read"),
            Some("read".to_string())
        );
    }

    #[test]
    fn test_extract_scope_missing() {
        assert_eq!(extract_scope_from_header("Bearer realm=\"example\""), None);
    }

    #[test]
    fn test_host_header_auto_derived() {
        let client = UnixSocketHttpClient::new(
            "http://staging.ai-app-info.gns.square/mcp",
            "/var/run/envoy.sock",
            HashMap::new(),
        );
        let host = client.default_headers.get(&http::header::HOST).unwrap();
        assert_eq!(host, "staging.ai-app-info.gns.square");
    }

    #[test]
    fn test_host_header_explicit_takes_precedence() {
        let mut headers = HashMap::new();
        headers.insert(
            http::header::HOST,
            HeaderValue::from_static("custom.example.com"),
        );
        let client = UnixSocketHttpClient::new(
            "http://staging.ai-app-info.gns.square/mcp",
            "/var/run/envoy.sock",
            headers,
        );
        let host = client.default_headers.get(&http::header::HOST).unwrap();
        assert_eq!(host, "custom.example.com");
    }
}
