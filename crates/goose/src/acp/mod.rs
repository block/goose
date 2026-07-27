mod common;
pub(crate) mod fs;
mod mcp_app_proxy;
mod provider;
mod response_builder;
pub mod server;
pub mod server_factory;
pub(crate) mod tool_call_notifier;
pub(crate) mod tools;
pub mod transport;

pub use common::{map_permission_response, PermissionDecision};

pub(crate) fn is_terminal_notification(notification: &serde_json::Value, session_id: &str) -> bool {
    if notification
        .get("sessionId")
        .and_then(|value| value.as_str())
        != Some(session_id)
    {
        return false;
    }

    notification
        .get("update")
        .and_then(|value| value.get("_meta"))
        .and_then(|value| value.as_object())
        .is_some_and(|meta| {
            meta.contains_key("terminal_output")
                || meta.contains_key("terminal_output_delta")
                || meta.contains_key("terminal_exit")
        })
}
pub use goose_sdk_types::{custom_notifications, custom_requests};
pub use provider::{
    extension_configs_to_mcp_servers, AcpProvider, AcpProviderConfig, ACP_CURRENT_MODEL,
};
