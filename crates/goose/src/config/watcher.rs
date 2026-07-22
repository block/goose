//! Hot-reload of the active provider/model from the goose config file.
//!
//! [`ConfigWatcher`] watches the config directory and, when the active provider
//! or its model changes on disk, rebuilds the provider and swaps it into a
//! running [`Agent`] via [`Agent::update_provider`], without restarting the
//! session.
//!
//! Precedence: `GOOSE_PROVIDER`/`GOOSE_MODEL` env vars still win over the file
//! (see [`crate::config::providers::get_active_provider`]); a file edit is a
//! no-op while they are set and unchanged. The config file is persisted
//! atomically (temp-file + rename), so the parent directory is watched rather
//! than the file itself.
//!
//! Dropping the [`ConfigWatcher`] guard stops watching and releases the
//! `Arc<Agent>` it holds: the guard owns the event sender, so its drop closes
//! the receiver and the reload task exits.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use futures::future::BoxFuture;
use notify::{Event, RecommendedWatcher, RecursiveMode, Watcher};
use tokio::sync::{mpsc, Mutex};
use tracing::{debug, info, warn};

use crate::agents::Agent;
use crate::config::base::CONFIG_YAML_NAME;
use crate::config::paths::Paths;
use crate::config::providers::{get_active_model, get_active_provider};
use crate::config::Config;
use crate::model_config::model_config_from_user_config;
use crate::providers::base::Provider;
use crate::providers::create;

/// Coalesce a burst of filesystem events (atomic temp + rename, editor saves)
/// into a single reload.
const DEBOUNCE: Duration = Duration::from_millis(300);

type BuildProvider =
    Arc<dyn Fn(&str) -> BoxFuture<'static, Result<Arc<dyn Provider>>> + Send + Sync>;
type ApplyProvider =
    Arc<dyn Fn(Arc<dyn Provider>, String, String) -> BoxFuture<'static, Result<()>> + Send + Sync>;

pub struct ConfigWatcher {
    _watcher: RecommendedWatcher,
}

impl ConfigWatcher {
    /// Watch the default config file and hot-swap `agent`'s provider on change.
    pub fn start(agent: Arc<Agent>, session_id: String) -> Result<Self> {
        let factory: BuildProvider = Arc::new(|name: &str| {
            let name = name.to_string();
            Box::pin(async move { create(&name, Vec::new()).await })
        });
        let apply: ApplyProvider = Arc::new({
            let agent = agent.clone();
            let session_id = session_id.clone();
            move |provider, provider_name, model| {
                let agent = agent.clone();
                let session_id = session_id.clone();
                Box::pin(async move {
                    let model_config = model_config_from_user_config(&provider_name, &model)?;
                    agent
                        .update_provider(provider, model_config, &session_id)
                        .await
                })
            }
        });
        Self::start_impl(factory, apply)
    }

    fn start_impl(factory: BuildProvider, apply: ApplyProvider) -> Result<Self> {
        let dir = Paths::config_dir();
        let target: PathBuf = dir.join(CONFIG_YAML_NAME);

        let (tx, mut rx) = mpsc::channel::<()>(16);
        let watch_target = target.clone();
        let mut watcher = notify::recommended_watcher(move |res: notify::Result<Event>| {
            if let Ok(ev) = res {
                if ev.paths.contains(&watch_target) {
                    let _ = tx.blocking_send(());
                }
            }
        })
        .context("initialize config file watcher")?;

        if let Err(e) = watcher.watch(&dir, RecursiveMode::NonRecursive) {
            // A missing config dir on a fresh install means there is nothing to
            // watch yet; reload begins working once the file is created.
            debug!(error = %e, "could not watch config dir; hot-reload inactive until it exists");
        }

        let last = Arc::new(Mutex::new(resolve_selection()));
        tokio::spawn(async move {
            loop {
                if rx.recv().await.is_none() {
                    break;
                }
                // Drain the burst, then reload once it has been quiet for DEBOUNCE.
                loop {
                    tokio::select! {
                        _ = rx.recv() => continue,
                        _ = tokio::time::sleep(DEBOUNCE) => break,
                    }
                }
                Config::global().invalidate_secrets_cache();
                let selection = resolve_selection();
                let mut guard = last.lock().await;
                match reload(&mut guard, selection, &factory, &apply).await {
                    Ok(true) => {}
                    Ok(false) => debug!("config changed but active selection unchanged"),
                    Err(e) => warn!(error = %e, "config hot-reload failed"),
                }
            }
            debug!("config watcher task exited");
        });

        Ok(Self { _watcher: watcher })
    }
}

/// Resolves the active (provider, model) from the process config. Env wins.
fn resolve_selection() -> Option<(String, String)> {
    let config = Config::global();
    let provider = get_active_provider(config)?;
    let model = get_active_model(config).unwrap_or_default();
    Some((provider, model))
}

/// Builds and applies the provider when `selection` differs from `last`.
/// Returns `Ok(true)` if a swap happened, `Ok(false)` on a no-op.
async fn reload(
    last: &mut Option<(String, String)>,
    selection: Option<(String, String)>,
    factory: &BuildProvider,
    apply: &ApplyProvider,
) -> Result<bool> {
    let selection = match selection {
        Some(s) => s,
        None => return Ok(false),
    };
    if *last == Some(selection.clone()) {
        return Ok(false);
    }
    let (name, model) = selection;
    let provider = factory(&name).await?;
    apply(provider, name.clone(), model.clone()).await?;
    let applied_model = model.clone();
    *last = Some((name, model));
    info!(model = %applied_model, "hot-reloaded provider from config change");
    Ok(true)
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose_providers::errors::ProviderError;
    use goose_providers::model::ModelConfig;

    struct StubProvider {
        name: String,
    }

    #[async_trait::async_trait]
    impl Provider for StubProvider {
        fn get_name(&self) -> &str {
            &self.name
        }
        async fn stream(
            &self,
            _model_config: &ModelConfig,
            _system: &str,
            _messages: &[crate::conversation::message::Message],
            _tools: &[rmcp::model::Tool],
        ) -> std::result::Result<crate::providers::base::MessageStream, ProviderError> {
            unimplemented!()
        }
        async fn fetch_recommended_models(
            &self,
            _toolshim: bool,
        ) -> std::result::Result<Vec<String>, ProviderError> {
            Ok(vec![])
        }
    }

    fn mocks() -> (BuildProvider, ApplyProvider, Arc<Mutex<Vec<String>>>) {
        let calls = Arc::new(Mutex::new(Vec::<String>::new()));
        let calls_c = calls.clone();
        let factory: BuildProvider = Arc::new(move |name: &str| {
            let name = name.to_string();
            let calls = calls_c.clone();
            Box::pin(async move {
                calls.lock().await.push(format!("build:{name}"));
                Ok(Arc::new(StubProvider { name }) as Arc<dyn Provider>)
            })
        });
        let calls_c2 = calls.clone();
        let apply: ApplyProvider = Arc::new(move |_provider, name, model| {
            let calls = calls_c2.clone();
            Box::pin(async move {
                calls.lock().await.push(format!("apply:{name}/{model}"));
                Ok(())
            })
        });
        (factory, apply, calls)
    }

    #[tokio::test]
    async fn reload_swaps_when_selection_changes() {
        let (factory, apply, calls) = mocks();
        let mut last = Some(("a".to_string(), "a-model".to_string()));

        // Same selection: no-op.
        assert!(!reload(
            &mut last,
            Some(("a".into(), "a-model".into())),
            &factory,
            &apply
        )
        .await
        .unwrap());
        assert!(calls.lock().await.is_empty());

        // Different selection: swap.
        assert!(reload(
            &mut last,
            Some(("b".into(), "b-model".into())),
            &factory,
            &apply
        )
        .await
        .unwrap());
        let recorded = calls.lock().await.clone();
        assert!(recorded.iter().any(|c| c == "build:b"));
        assert!(recorded.iter().any(|c| c == "apply:b/b-model"));
        assert_eq!(last, Some(("b".to_string(), "b-model".to_string())));

        // No active provider configured: no-op.
        assert!(!reload(&mut last, None, &factory, &apply).await.unwrap());
    }
}
