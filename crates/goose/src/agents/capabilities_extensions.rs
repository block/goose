//! Adds `extensions` to MCP client capabilities during initialization.
//!
//! rmcp's ClientCapabilities doesn't yet support the `extensions` field from SEP-1724,
//! so we wrap HTTP clients to inject it into the initialize request JSON body.
//!
//! See: https://github.com/modelcontextprotocol/modelcontextprotocol/issues/1724

use futures::stream::BoxStream;
use futures::StreamExt;
use reqwest::header::{ACCEPT, CONTENT_TYPE};
use rmcp::model::{ClientJsonRpcMessage, ServerJsonRpcMessage};
use rmcp::transport::streamable_http_client::{
    AuthRequiredError, SseError, StreamableHttpClient, StreamableHttpError,
    StreamableHttpPostResponse,
};
use serde_json::{json, Value};
use sse_stream::{Sse, SseStream};
use std::future::Future;
use std::sync::Arc;

const HEADER_SESSION_ID: &str = "mcp-session-id";
const EVENT_STREAM_MIME_TYPE: &str = "text/event-stream";
const JSON_MIME_TYPE: &str = "application/json";
const WWW_AUTHENTICATE: &str = "www-authenticate";

/// Wraps a reqwest::Client to inject `extensions` into the initialize request.
#[derive(Clone)]
pub struct WithExtensions(pub reqwest::Client);

#[allow(clippy::manual_async_fn)]
impl StreamableHttpClient for WithExtensions {
    type Error = reqwest::Error;

    fn post_message(
        &self,
        uri: Arc<str>,
        message: ClientJsonRpcMessage,
        session_id: Option<Arc<str>>,
        auth_header: Option<String>,
    ) -> impl Future<Output = Result<StreamableHttpPostResponse, StreamableHttpError<Self::Error>>>
           + Send
           + '_ {
        async move {
            // Serialize message to JSON
            let mut json_value =
                serde_json::to_value(&message).map_err(StreamableHttpError::Deserialize)?;

            // Inject extensions if this is an initialize request
            if json_value.get("method").and_then(|m| m.as_str()) == Some("initialize") {
                if let Some(params) = json_value.get_mut("params") {
                    if let Some(Value::Object(caps)) = params.get_mut("capabilities") {
                        caps.insert("extensions".to_string(), supported_extensions());
                    }
                }
            }

            // Build and send request with raw JSON body
            let mut request = self
                .0
                .post(uri.as_ref())
                .header(
                    ACCEPT,
                    format!("{}, {}", EVENT_STREAM_MIME_TYPE, JSON_MIME_TYPE),
                )
                .header(CONTENT_TYPE, JSON_MIME_TYPE);

            if let Some(auth) = auth_header {
                request = request.bearer_auth(auth);
            }
            if let Some(sid) = session_id {
                request = request.header(HEADER_SESSION_ID, sid.as_ref());
            }

            // Send as raw JSON string to preserve our injected extensions
            let json_body =
                serde_json::to_string(&json_value).map_err(StreamableHttpError::Deserialize)?;
            let response = request.body(json_body).send().await?;

            // Handle auth required
            if response.status() == reqwest::StatusCode::UNAUTHORIZED {
                if let Some(header) = response.headers().get(WWW_AUTHENTICATE) {
                    let header = header
                        .to_str()
                        .map_err(|_| {
                            StreamableHttpError::UnexpectedServerResponse(std::borrow::Cow::from(
                                "invalid www-authenticate header value",
                            ))
                        })?
                        .to_string();
                    return Err(StreamableHttpError::AuthRequired(AuthRequiredError {
                        www_authenticate_header: header,
                    }));
                }
            }

            let status = response.status();
            let response = response.error_for_status()?;

            if matches!(
                status,
                reqwest::StatusCode::ACCEPTED | reqwest::StatusCode::NO_CONTENT
            ) {
                return Ok(StreamableHttpPostResponse::Accepted);
            }

            let content_type = response.headers().get(reqwest::header::CONTENT_TYPE);
            let session_id = response
                .headers()
                .get(HEADER_SESSION_ID)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string());

            match content_type {
                Some(ct) if ct.as_bytes().starts_with(EVENT_STREAM_MIME_TYPE.as_bytes()) => {
                    let event_stream = SseStream::from_byte_stream(response.bytes_stream()).boxed();
                    Ok(StreamableHttpPostResponse::Sse(event_stream, session_id))
                }
                Some(ct) if ct.as_bytes().starts_with(JSON_MIME_TYPE.as_bytes()) => {
                    let message: ServerJsonRpcMessage = response.json().await?;
                    Ok(StreamableHttpPostResponse::Json(message, session_id))
                }
                _ => Err(StreamableHttpError::UnexpectedContentType(
                    content_type.map(|ct| String::from_utf8_lossy(ct.as_bytes()).to_string()),
                )),
            }
        }
    }

    fn delete_session(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        auth_header: Option<String>,
    ) -> impl Future<Output = Result<(), StreamableHttpError<Self::Error>>> + Send + '_ {
        async move {
            let mut request = self.0.delete(uri.as_ref());
            if let Some(auth) = auth_header {
                request = request.bearer_auth(auth);
            }
            let response = request
                .header(HEADER_SESSION_ID, session_id.as_ref())
                .send()
                .await?;

            if response.status() == reqwest::StatusCode::METHOD_NOT_ALLOWED {
                return Ok(());
            }
            let _response = response.error_for_status()?;
            Ok(())
        }
    }

    fn get_stream(
        &self,
        uri: Arc<str>,
        session_id: Arc<str>,
        last_event_id: Option<String>,
        auth_header: Option<String>,
    ) -> impl Future<
        Output = Result<
            BoxStream<'static, Result<Sse, SseError>>,
            StreamableHttpError<Self::Error>,
        >,
    > + Send
           + '_ {
        async move {
            let mut request = self
                .0
                .get(uri.as_ref())
                .header(ACCEPT, EVENT_STREAM_MIME_TYPE)
                .header(HEADER_SESSION_ID, session_id.as_ref());

            if let Some(auth) = auth_header {
                request = request.bearer_auth(auth);
            }
            if let Some(last_id) = last_event_id {
                request = request.header("Last-Event-ID", last_id);
            }

            let response = request.send().await?.error_for_status()?;

            if let Some(ct) = response.headers().get(reqwest::header::CONTENT_TYPE) {
                if !ct.as_bytes().starts_with(EVENT_STREAM_MIME_TYPE.as_bytes())
                    && !ct.as_bytes().starts_with(JSON_MIME_TYPE.as_bytes())
                {
                    return Err(StreamableHttpError::UnexpectedContentType(Some(
                        String::from_utf8_lossy(ct.as_bytes()).to_string(),
                    )));
                }
            } else {
                return Err(StreamableHttpError::UnexpectedContentType(None));
            }

            let event_stream = SseStream::from_byte_stream(response.bytes_stream()).boxed();
            Ok(event_stream)
        }
    }
}

/// Returns the extensions goose supports.
fn supported_extensions() -> Value {
    json!({
        // MCP Apps UI extension
        // https://github.com/modelcontextprotocol/ext-apps/blob/main/specification/draft/apps.mdx#client-host-capabilities
        "io.modelcontextprotocol/ui": {
            "mimeTypes": ["text/html;profile=mcp-app"]
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_supported_extensions_format() {
        let extensions = supported_extensions();
        assert_eq!(
            extensions["io.modelcontextprotocol/ui"]["mimeTypes"][0],
            "text/html;profile=mcp-app"
        );
    }

    #[test]
    fn test_inject_extensions_into_json() {
        let mut json = json!({
            "jsonrpc": "2.0",
            "id": 1,
            "method": "initialize",
            "params": {
                "protocolVersion": "2024-11-05",
                "capabilities": { "sampling": {} },
                "clientInfo": { "name": "goose", "version": "1.0.0" }
            }
        });

        // Simulate the injection logic
        if let Some(params) = json.get_mut("params") {
            if let Some(Value::Object(caps)) = params.get_mut("capabilities") {
                caps.insert("extensions".to_string(), supported_extensions());
            }
        }

        assert_eq!(
            json["params"]["capabilities"]["extensions"]["io.modelcontextprotocol/ui"]["mimeTypes"]
                [0],
            "text/html;profile=mcp-app"
        );
    }
}
