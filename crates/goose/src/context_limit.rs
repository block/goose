use anyhow::Result;

use crate::config::Config;
use crate::providers::base::Provider;

pub async fn get_context_limit(provider: &dyn Provider, model: &str) -> Result<usize> {
    let override_limit = Config::global().get_goose_context_limit()?;
    Ok(provider.get_context_limit(model, override_limit).await)
}
