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

#[derive(Debug, Clone, Default)]
pub struct ModelFilter {
    pub model: Option<String>,
    pub geo: Option<String>,
    pub max_cost: Option<f64>,
    pub min_context: Option<u32>,
}

impl ModelFilter {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn model(mut self, name: &str) -> Self {
        self.model = Some(name.to_string());
        self
    }

    pub fn geo(mut self, geo: &str) -> Self {
        self.geo = Some(geo.to_string());
        self
    }

    pub fn max_cost(mut self, cost: f64) -> Self {
        self.max_cost = Some(cost);
        self
    }

    pub fn min_context(mut self, ctx: u32) -> Self {
        self.min_context = Some(ctx);
        self
    }

    pub fn matches(&self, model: &DiscoveredModel) -> bool {
        if let Some(ref name) = self.model {
            if !model.model_name.contains(name) {
                return false;
            }
        }
        if let Some(ref geo) = self.geo {
            match &model.geo {
                Some(model_geo) if model_geo == geo => {}
                _ => return false,
            }
        }
        if let Some(max) = self.max_cost {
            match model.cost {
                Some(c) if c <= max => {}
                Some(_) => return false,
                None => {} // free models pass
            }
        }
        if let Some(min) = self.min_context {
            match model.context_size {
                Some(c) if c >= min => {}
                _ => return false,
            }
        }
        true
    }
}

/// Discover available models from Nostr relays.
/// Returns the first available model, optionally filtered by model name.
pub async fn discover_model(
    relays: Option<Vec<String>>,
    preferred_model: Option<&str>,
) -> anyhow::Result<Option<DiscoveredModel>> {
    let filter = preferred_model
        .map(|m| ModelFilter::new().model(m))
        .unwrap_or_default();
    discover_model_filtered(relays, &filter).await
}

/// Discover a model with filtering options.
pub async fn discover_model_filtered(
    relays: Option<Vec<String>>,
    filter: &ModelFilter,
) -> anyhow::Result<Option<DiscoveredModel>> {
    let relays = relays.unwrap_or_else(|| DEFAULT_RELAYS.iter().map(|s| s.to_string()).collect());

    let discovery = ModelDiscovery::new(relays).await?;
    discovery.connect().await;

    let models = discovery.discover().await?;

    Ok(models.into_iter().find(|m| filter.matches(m)))
}

/// Discover all available models from Nostr relays.
pub async fn discover_models(relays: Option<Vec<String>>) -> anyhow::Result<Vec<DiscoveredModel>> {
    let relays = relays.unwrap_or_else(|| DEFAULT_RELAYS.iter().map(|s| s.to_string()).collect());

    let discovery = ModelDiscovery::new(relays).await?;
    discovery.connect().await;
    discovery.discover().await
}
