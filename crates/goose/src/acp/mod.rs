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
    /// Set while the model picker lists candidate providers. ACP providers that
    /// pin a model at construction (only Copilot today) then start on their
    /// `current` model instead. Listing needs a provider's model list, not a
    /// running model, and pinning the active session's model can make an ACP
    /// agent reject a foreign model and drop the provider from the picker.
    static LISTING_MODELS_ONLY: ();
}

/// Run `f` with ACP model pinning suppressed, so an ACP provider constructs on
/// its current model rather than the active session's. The `/model` picker uses
/// this while gathering each provider's model list.
pub async fn while_listing_models<F>(f: F) -> F::Output
where
    F: std::future::Future,
{
    LISTING_MODELS_ONLY.scope((), f).await
}

/// Whether the current task is inside [`while_listing_models`].
pub(crate) fn is_listing_models() -> bool {
    LISTING_MODELS_ONLY.try_with(|_| ()).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn is_listing_models_false_outside_scope() {
        assert!(!is_listing_models());
    }

    #[tokio::test]
    async fn is_listing_models_true_inside_scope() {
        while_listing_models(async {
            assert!(is_listing_models());
        })
        .await;
        // Scope does not leak past the future.
        assert!(!is_listing_models());
    }
}
