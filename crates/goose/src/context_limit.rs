use anyhow::Result;
use goose_providers::context_limit::ContextLimitResolver;

use crate::config::Config;
use crate::providers::base::Provider;

pub async fn get_context_limit(provider: &dyn Provider, model: &str) -> Result<usize> {
    let override_limit = Config::global().get_goose_context_limit()?;
    Ok(provider.get_context_limit(model, override_limit).await)
}

pub fn get_local_context_limit(provider_name: &str, model: &str) -> Result<usize> {
    let override_limit = Config::global().get_goose_context_limit()?;
    let configured_limits = crate::config::declarative_providers::load_provider(provider_name)
        .ok()
        .into_iter()
        .flat_map(|loaded| loaded.config.models)
        .filter_map(|model| model.context_limit.map(|limit| (model.name.clone(), limit)));

    Ok(ContextLimitResolver::new(provider_name)
        .with_configured_limits(configured_limits)
        .resolve_local(model, override_limit))
}
