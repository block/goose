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
pub use goose_sdk_types::{custom_notifications, custom_requests};
pub use provider::{
    extension_configs_to_mcp_servers, AcpProvider, AcpProviderConfig, ACP_CURRENT_MODEL,
};

pub(crate) fn is_auth_required(error: &anyhow::Error) -> bool {
    error.chain().any(|source| {
        source
            .downcast_ref::<agent_client_protocol::Error>()
            .is_some_and(|error| {
                error.code == agent_client_protocol::schema::v1::ErrorCode::AuthRequired
            })
    })
}

#[cfg(test)]
mod tests {
    use super::is_auth_required;

    #[test]
    fn identifies_typed_auth_required_errors() {
        let error = anyhow::Error::new(agent_client_protocol::Error::auth_required());

        assert!(is_auth_required(&error));
    }

    #[test]
    fn does_not_classify_other_acp_errors_as_authentication() {
        let error = anyhow::Error::new(agent_client_protocol::Error::internal_error());

        assert!(!is_auth_required(&error));
    }
}
