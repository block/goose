mod mistral;

use super::provider_registry::ProviderRegistry;
use anyhow::Result;

pub(super) fn register_bundled_custom_providers(registry: &mut ProviderRegistry) -> Result<()> {
    mistral::register(registry)
}
