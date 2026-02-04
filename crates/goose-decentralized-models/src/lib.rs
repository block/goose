pub mod config;
pub mod keys;
pub mod publisher;

pub use config::NostrShareConfig;
pub use keys::KeyManager;
pub use publisher::{DiscoveredModel, ModelDiscovery, ModelPublisher};

/// Default relays for discovery
pub const DEFAULT_RELAYS: &[&str] = &[
    "wss://relay.damus.io",
    "wss://nos.lol",
    "wss://relay.nostr.band",
];

/// Discover available models from Nostr relays.
/// Returns the first available model, optionally filtered by model name.
pub async fn discover_model(
    relays: Option<Vec<String>>,
    preferred_model: Option<&str>,
) -> anyhow::Result<Option<DiscoveredModel>> {
    let relays = relays.unwrap_or_else(|| DEFAULT_RELAYS.iter().map(|s| s.to_string()).collect());

    let discovery = ModelDiscovery::new(relays).await?;
    discovery.connect().await;

    let models = discovery.discover().await?;

    if models.is_empty() {
        return Ok(None);
    }

    // Prefer specified model if given
    if let Some(name) = preferred_model {
        if let Some(m) = models.iter().find(|m| m.model_name.contains(name)) {
            return Ok(Some(m.clone()));
        }
    }

    Ok(models.into_iter().next())
}

/// Discover all available models from Nostr relays.
pub async fn discover_models(relays: Option<Vec<String>>) -> anyhow::Result<Vec<DiscoveredModel>> {
    let relays = relays.unwrap_or_else(|| DEFAULT_RELAYS.iter().map(|s| s.to_string()).collect());

    let discovery = ModelDiscovery::new(relays).await?;
    discovery.connect().await;
    discovery.discover().await
}
