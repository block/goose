mod common;
pub(crate) mod fs;
mod mcp_app_proxy;
mod provider;
mod response_builder;
pub mod server;
pub mod server_factory;
pub(crate) mod tools;
pub mod transport;

pub use common::{map_permission_response, PermissionDecision};
pub use goose_sdk_types::{custom_notifications, custom_requests};
pub use provider::{
    extension_configs_to_mcp_servers, AcpProvider, AcpProviderConfig, ACP_CURRENT_MODEL,
};

tokio::task_local! {
    /// Set while an ACP provider must construct on its current model instead of
    /// pinning the global `GOOSE_MODEL` (only Copilot pins today). The `/model`
    /// picker needs this twice: while listing candidates, and while installing a
    /// selection, because in both cases the global model still belongs to the
    /// previous provider and the ACP agent would reject it as foreign. Any model
    /// the user picks is carried in the session's `ModelConfig` and applied on
    /// first use by `apply_model_if_changed`.
    static SUPPRESS_MODEL_PINNING: ();
}

/// Run `f` with ACP model pinning suppressed, so an ACP provider constructs on
/// its current model rather than the global `GOOSE_MODEL`.
pub async fn without_model_pinning<F>(f: F) -> F::Output
where
    F: std::future::Future,
{
    SUPPRESS_MODEL_PINNING.scope((), f).await
}

/// Whether the current task is inside [`without_model_pinning`].
pub(crate) fn is_model_pinning_suppressed() -> bool {
    SUPPRESS_MODEL_PINNING.try_with(|_| ()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn model_pinning_not_suppressed_outside_scope() {
        assert!(!is_model_pinning_suppressed());
    }

    #[tokio::test]
    async fn model_pinning_suppressed_inside_scope() {
        without_model_pinning(async {
            assert!(is_model_pinning_suppressed());
        })
        .await;
        // Scope does not leak past the future.
        assert!(!is_model_pinning_suppressed());
    }
}
