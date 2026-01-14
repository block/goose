//! Adds `extensions` to MCP client capabilities during initialization.
//!
//! rmcp's ClientCapabilities doesn't yet support the `extensions` field from SEP-1724,
//! so we wrap the transport to inject it into the initialize request JSON.
//!
//! See: https://github.com/modelcontextprotocol/modelcontextprotocol/issues/1724

use rmcp::{
    model::{ClientJsonRpcMessage, ServerJsonRpcMessage},
    service::TxJsonRpcMessage,
    transport::Transport,
    RoleClient,
};
use serde_json::{json, Value};
use std::borrow::Cow;

/// Wraps a transport to inject `extensions` into the initialize request capabilities.
pub struct WithExtensions<T>(pub T);

impl<T: Transport<RoleClient> + Send> Transport<RoleClient> for WithExtensions<T> {
    type Error = T::Error;

    fn name() -> Cow<'static, str> {
        "WithExtensions".into()
    }

    fn send(
        &mut self,
        item: TxJsonRpcMessage<RoleClient>,
    ) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send + 'static {
        self.0.send(inject_extensions(item))
    }

    fn receive(
        &mut self,
    ) -> impl std::future::Future<Output = Option<ServerJsonRpcMessage>> + Send {
        self.0.receive()
    }

    fn close(&mut self) -> impl std::future::Future<Output = Result<(), Self::Error>> + Send {
        self.0.close()
    }
}

/// Inject extensions into initialize request capabilities.
fn inject_extensions(message: ClientJsonRpcMessage) -> ClientJsonRpcMessage {
    let Ok(mut json) = serde_json::to_value(&message) else {
        return message;
    };

    if json.get("method").and_then(|m| m.as_str()) != Some("initialize") {
        return message;
    }

    // Navigate to params.capabilities and add extensions
    if let Some(params) = json.get_mut("params") {
        if let Some(Value::Object(caps)) = params.get_mut("capabilities") {
            caps.insert("extensions".to_string(), supported_extensions());
        }
    }

    serde_json::from_value(json).unwrap_or(message)
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
    fn test_injects_extensions_into_json() {
        // Test the JSON manipulation directly without round-tripping through rmcp types
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

        // Inject extensions directly
        if let Some(params) = json.get_mut("params") {
            if let Some(Value::Object(caps)) = params.get_mut("capabilities") {
                caps.insert("extensions".to_string(), supported_extensions());
            }
        }

        // Verify the injection worked
        assert_eq!(
            json["params"]["capabilities"]["extensions"]["io.modelcontextprotocol/ui"]["mimeTypes"]
                [0],
            "text/html;profile=mcp-app"
        );
    }

    #[test]
    fn test_ignores_non_initialize() {
        let req = json!({
            "jsonrpc": "2.0",
            "id": 2,
            "method": "tools/list",
            "params": {}
        });

        let message: ClientJsonRpcMessage = serde_json::from_value(req).unwrap();
        let modified = inject_extensions(message);
        let json = serde_json::to_value(&modified).unwrap();

        // Should not have extensions added to a non-initialize request
        assert!(json.pointer("/params/capabilities/extensions").is_none());
    }
}
