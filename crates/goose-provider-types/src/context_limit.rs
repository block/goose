use std::collections::HashMap;
use std::future::Future;

use crate::canonical::maybe_get_canonical_model;
use crate::errors::ProviderError;
use crate::model::DEFAULT_CONTEXT_LIMIT;

#[derive(Debug, Clone, Default)]
pub struct ContextLimitResolver {
    provider_name: String,
    configured_limits: HashMap<String, usize>,
}

impl ContextLimitResolver {
    pub fn new(provider_name: impl Into<String>) -> Self {
        Self {
            provider_name: provider_name.into(),
            configured_limits: HashMap::new(),
        }
    }

    pub fn with_configured_limits(
        mut self,
        configured_limits: impl IntoIterator<Item = (String, usize)>,
    ) -> Self {
        self.configured_limits.extend(configured_limits);
        self
    }

    pub async fn resolve<F, Fut>(
        &self,
        model: &str,
        override_limit: Option<usize>,
        discover: F,
    ) -> usize
    where
        F: FnOnce() -> Fut,
        Fut: Future<Output = Result<Option<usize>, ProviderError>>,
    {
        if let Some(limit) = override_limit {
            return limit;
        }

        if let Some(limit) = self.configured_limits.get(model) {
            return *limit;
        }

        match discover().await {
            Ok(Some(limit)) => return limit,
            Ok(None) => {}
            Err(error) => tracing::warn!(
                provider = self.provider_name,
                model,
                %error,
                "Context-limit discovery failed; falling back"
            ),
        }

        maybe_get_canonical_model(&self.provider_name, model)
            .map(|canonical| canonical.limit.context)
            .unwrap_or(DEFAULT_CONTEXT_LIMIT)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn applies_precedence() {
        let resolver = ContextLimitResolver::new("anthropic")
            .with_configured_limits([("claude-sonnet-4-5".to_string(), 64_000)]);

        assert_eq!(
            resolver
                .resolve("claude-sonnet-4-5", Some(32_000), || async {
                    Ok(Some(16_000))
                })
                .await,
            32_000
        );
        assert_eq!(
            resolver
                .resolve("claude-sonnet-4-5", None, || async { Ok(Some(16_000)) })
                .await,
            64_000
        );
    }

    #[tokio::test]
    async fn uses_discovery_then_canonical_then_default() {
        let resolver = ContextLimitResolver::new("anthropic");
        assert_eq!(
            resolver
                .resolve("runtime-model", None, || async { Ok(Some(24_000)) })
                .await,
            24_000
        );
        assert_eq!(
            resolver
                .resolve("claude-sonnet-4-5", None, || async { Ok(None) })
                .await,
            1_000_000
        );
        assert_eq!(
            resolver
                .resolve("unknown-model", None, || async { Ok(None) })
                .await,
            DEFAULT_CONTEXT_LIMIT
        );
    }
}
