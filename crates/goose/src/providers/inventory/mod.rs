mod db;
mod helpers;
mod types;
pub use db::*;
pub use helpers::*;
pub use types::*;

use super::base::{ConfigKey, ModelInfo, ProviderType};
use super::canonical::{map_provider_name, map_to_canonical_model, CanonicalModelRegistry};
use super::catalog::ProviderSetupCategory;
use crate::config::declarative_providers::{DeclarativeProviderConfig, ProviderEngine};
use crate::config::Config;
use crate::session::session_manager::SessionStorage;
use anyhow::{Context, Result};
use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::{Pool, Row, Sqlite, Transaction};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::sync::{Arc, PoisonError, RwLock, RwLockReadGuard, RwLockWriteGuard};
use tracing::warn;

const STALE_AFTER_HOURS: i64 = 24;

#[derive(Clone)]
pub struct ProviderInventoryService {
    storage: Arc<SessionStorage>,
    refreshing_keys: Arc<RwLock<HashSet<String>>>,
}

pub(crate) struct RefreshGuard {
    inventory_key: String,
    refreshing_keys: Arc<RwLock<HashSet<String>>>,
    completed: bool,
}

impl RefreshGuard {
    /// Mark the refresh as finished and remove its inventory key from the
    /// refreshing-keys set. `RefreshGuard` is the single owner of refresh-key
    /// removal; store methods do not clear keys themselves.
    pub fn complete(&mut self) {
        if self.completed {
            return;
        }
        let mut refreshing_keys = self
            .refreshing_keys
            .write()
            .unwrap_or_else(|poisoned| recover_poisoned_write(poisoned, "refreshing_keys"));
        refreshing_keys.remove(&self.inventory_key);
        self.completed = true;
    }
}

impl Drop for RefreshGuard {
    fn drop(&mut self) {
        self.complete();
    }
}

fn recover_poisoned_read<'a, T>(
    poisoned: PoisonError<RwLockReadGuard<'a, T>>,
    lock_name: &str,
) -> RwLockReadGuard<'a, T> {
    warn!(
        lock = lock_name,
        "recovering poisoned provider inventory read lock"
    );
    poisoned.into_inner()
}

fn recover_poisoned_write<'a, T>(
    poisoned: PoisonError<RwLockWriteGuard<'a, T>>,
    lock_name: &str,
) -> RwLockWriteGuard<'a, T> {
    warn!(
        lock = lock_name,
        "recovering poisoned provider inventory write lock"
    );
    poisoned.into_inner()
}

#[derive(Debug, Clone)]
struct InventorySnapshot {
    models: Vec<InventoryModel>,
    last_updated_at: Option<DateTime<Utc>>,
    last_refresh_attempt_at: Option<DateTime<Utc>>,
    last_refresh_error: Option<String>,
}

#[derive(Debug, Clone)]
struct ProviderDescriptor {
    provider_id: String,
    provider_name: String,
    description: String,
    default_model: String,
    identity: InventoryIdentity,
    configured: bool,
    provider_type: ProviderType,
    category: ProviderSetupCategory,
    config_keys: Vec<ConfigKey>,
    setup_steps: Vec<String>,
    supports_refresh: bool,
    static_models: Vec<ModelInfo>,
    model_selection_hint: Option<String>,
}

impl ProviderInventoryService {
    pub fn new(storage: Arc<SessionStorage>) -> ProviderInventoryService {
        ProviderInventoryService {
            storage,
            refreshing_keys: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    pub async fn entry_for_provider(
        &self,
        provider_id: &str,
    ) -> Result<Option<ProviderInventoryEntry>> {
        let Some(descriptor) = self.describe_provider(provider_id).await? else {
            return Ok(None);
        };
        let snapshot = self.read_snapshot(&descriptor.identity).await?;
        let refreshing = self
            .refreshing_keys
            .read()
            .unwrap_or_else(|poisoned| recover_poisoned_read(poisoned, "refreshing_keys"))
            .contains(&descriptor.identity.inventory_key);
        let models = inventory_models_from_snapshot(
            snapshot.as_ref(),
            &descriptor.identity.provider_family,
            &descriptor.static_models,
        );

        Ok(Some(ProviderInventoryEntry {
            provider_id: descriptor.provider_id,
            provider_name: descriptor.provider_name,
            description: descriptor.description,
            default_model: descriptor.default_model,
            configured: descriptor.configured,
            provider_type: descriptor.provider_type,
            category: descriptor.category,
            config_keys: descriptor.config_keys,
            setup_steps: descriptor.setup_steps,
            supports_refresh: descriptor.supports_refresh,
            refreshing,
            models,
            last_updated_at: snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.last_updated_at),
            last_refresh_attempt_at: snapshot
                .as_ref()
                .and_then(|snapshot| snapshot.last_refresh_attempt_at),
            last_refresh_error: snapshot.and_then(|snapshot| snapshot.last_refresh_error),
            model_selection_hint: descriptor.model_selection_hint,
        }))
    }

    pub async fn entries(&self, provider_ids: &[String]) -> Result<Vec<ProviderInventoryEntry>> {
        let ids = self.resolve_provider_ids(provider_ids).await;
        let handles: Vec<_> = ids
            .into_iter()
            .map(|id| {
                let this = self.clone();
                tokio::spawn(async move { this.entry_for_provider(&id).await })
            })
            .collect();
        let results = futures::future::join_all(handles).await;
        let mut entries = Vec::with_capacity(results.len());
        for result in results {
            let inner = result.context("provider inventory task panicked")?;
            if let Some(entry) = inner? {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    pub async fn plan_refresh(&self, provider_ids: &[String]) -> Result<RefreshPlan> {
        self.plan_refresh_jobs(provider_ids)
            .await
            .map(RefreshJobPlan::into_public_plan)
    }

    pub(crate) async fn plan_refresh_jobs(
        &self,
        provider_ids: &[String],
    ) -> Result<RefreshJobPlan> {
        let ids = self.resolve_provider_ids(provider_ids).await;
        let mut plan = RefreshJobPlan::default();
        let mut inserted_refreshing = Vec::new();

        for provider_id in ids {
            let Some(descriptor) = self.describe_provider(&provider_id).await? else {
                plan.skipped.push(RefreshSkip {
                    provider_id,
                    reason: RefreshSkipReason::UnknownProvider,
                });
                continue;
            };

            if !descriptor.supports_refresh {
                plan.skipped.push(RefreshSkip {
                    provider_id: descriptor.provider_id,
                    reason: RefreshSkipReason::DoesNotSupportRefresh,
                });
                continue;
            }

            if !descriptor.configured {
                plan.skipped.push(RefreshSkip {
                    provider_id: descriptor.provider_id,
                    reason: RefreshSkipReason::NotConfigured,
                });
                continue;
            }

            let already_refreshing = {
                let mut refreshing_keys = self
                    .refreshing_keys
                    .write()
                    .unwrap_or_else(|poisoned| recover_poisoned_write(poisoned, "refreshing_keys"));
                if refreshing_keys.contains(&descriptor.identity.inventory_key) {
                    true
                } else {
                    refreshing_keys.insert(descriptor.identity.inventory_key.clone());
                    false
                }
            };

            if already_refreshing {
                plan.skipped.push(RefreshSkip {
                    provider_id: descriptor.provider_id,
                    reason: RefreshSkipReason::AlreadyRefreshing,
                });
                continue;
            }

            inserted_refreshing.push(descriptor.identity.clone());
            if let Err(error) = self.mark_refresh_started(&descriptor.identity).await {
                self.clear_refreshing_many(&inserted_refreshing);
                return Err(error);
            }

            plan.started.push(RefreshJob {
                provider_id: descriptor.provider_id,
                identity: descriptor.identity,
            });
        }

        Ok(plan)
    }

    pub async fn store_refreshed_models(
        &self,
        provider_id: &str,
        model_ids: &[String],
    ) -> Result<()> {
        let descriptor = self.require_provider(provider_id).await?;
        self.store_refreshed_models_for_identity(&descriptor.identity, model_ids)
            .await?;
        self.clear_refreshing_many(std::slice::from_ref(&descriptor.identity));
        Ok(())
    }

    pub(crate) async fn store_refreshed_models_for_identity(
        &self,
        identity: &InventoryIdentity,
        model_ids: &[String],
    ) -> Result<()> {
        let models = enrich_model_ids_with_canonical(&identity.provider_family, model_ids);
        let now = Utc::now();
        let pool = self.storage.pool().await?;
        let mut tx = pool.begin().await?;

        sqlx::query(
            r#"
            INSERT INTO provider_inventory_entries (
                inventory_key,
                provider_id,
                provider_family,
                last_updated_at,
                last_refresh_attempt_at,
                last_refresh_error,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, NULL, CURRENT_TIMESTAMP)
            ON CONFLICT(inventory_key) DO UPDATE SET
                provider_id = excluded.provider_id,
                provider_family = excluded.provider_family,
                last_updated_at = excluded.last_updated_at,
                last_refresh_attempt_at = excluded.last_refresh_attempt_at,
                last_refresh_error = NULL,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&identity.inventory_key)
        .bind(&identity.provider_id)
        .bind(&identity.provider_family)
        .bind(now.to_rfc3339())
        .bind(now.to_rfc3339())
        .execute(&mut *tx)
        .await?;

        sqlx::query("DELETE FROM provider_inventory_models WHERE inventory_key = ?")
            .bind(&identity.inventory_key)
            .execute(&mut *tx)
            .await?;

        for (ordinal, model) in models.iter().enumerate() {
            sqlx::query(
                r#"
                INSERT INTO provider_inventory_models (
                    inventory_key,
                    ordinal,
                    model_id,
                    name,
                    family,
                    context_limit,
                    reasoning,
                    recommended
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
                "#,
            )
            .bind(&identity.inventory_key)
            .bind(i64::try_from(ordinal)?)
            .bind(&model.id)
            .bind(&model.name)
            .bind(&model.family)
            .bind(model.context_limit.map(i64::try_from).transpose()?)
            .bind(model.reasoning)
            .bind(model.recommended)
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        Ok(())
    }

    pub async fn store_refresh_error(
        &self,
        provider_id: &str,
        error: impl Into<String>,
    ) -> Result<()> {
        let descriptor = self.require_provider(provider_id).await?;
        self.store_refresh_error_for_identity(&descriptor.identity, error)
            .await?;
        self.clear_refreshing_many(std::slice::from_ref(&descriptor.identity));
        Ok(())
    }

    pub(crate) async fn store_refresh_error_for_identity(
        &self,
        identity: &InventoryIdentity,
        error: impl Into<String>,
    ) -> Result<()> {
        let error = error.into();
        let existing = self.read_snapshot(identity).await?;

        sqlx::query(
            r#"
            INSERT INTO provider_inventory_entries (
                inventory_key,
                provider_id,
                provider_family,
                last_updated_at,
                last_refresh_attempt_at,
                last_refresh_error,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, ?, CURRENT_TIMESTAMP)
            ON CONFLICT(inventory_key) DO UPDATE SET
                provider_id = excluded.provider_id,
                provider_family = excluded.provider_family,
                last_updated_at = excluded.last_updated_at,
                last_refresh_attempt_at = excluded.last_refresh_attempt_at,
                last_refresh_error = excluded.last_refresh_error,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&identity.inventory_key)
        .bind(&identity.provider_id)
        .bind(&identity.provider_family)
        .bind(existing.and_then(|snapshot| snapshot.last_updated_at.map(|time| time.to_rfc3339())))
        .bind(Utc::now().to_rfc3339())
        .bind(error)
        .execute(self.storage.pool().await?)
        .await?;

        Ok(())
    }

    fn clear_refreshing_many(&self, identities: &[InventoryIdentity]) {
        let mut refreshing_keys = self
            .refreshing_keys
            .write()
            .unwrap_or_else(|poisoned| recover_poisoned_write(poisoned, "refreshing_keys"));
        for identity in identities {
            refreshing_keys.remove(&identity.inventory_key);
        }
    }

    pub(crate) fn refresh_guard(&self, identity: &InventoryIdentity) -> RefreshGuard {
        RefreshGuard {
            inventory_key: identity.inventory_key.clone(),
            refreshing_keys: Arc::clone(&self.refreshing_keys),
            completed: false,
        }
    }

    pub fn is_stale(entry: &ProviderInventoryEntry) -> bool {
        let Some(last_updated_at) = entry.last_updated_at else {
            return false;
        };
        entry.supports_refresh && Utc::now() - last_updated_at > Duration::hours(STALE_AFTER_HOURS)
    }

    async fn describe_provider(&self, provider_id: &str) -> Result<Option<ProviderDescriptor>> {
        let entry = match crate::providers::get_from_registry(provider_id).await {
            Ok(entry) => entry,
            Err(_) => return Ok(None),
        };
        let metadata = entry.metadata().clone();
        let identity = crate::providers::inventory_identity(provider_id)
            .await
            .unwrap_or_else(|_| fallback_inventory_identity(provider_id))
            .into_identity()?;

        Ok(Some(ProviderDescriptor {
            provider_id: metadata.name.clone(),
            provider_name: metadata.display_name.clone(),
            description: metadata.description.clone(),
            default_model: metadata.default_model.clone(),
            identity,
            configured: entry.inventory_configured(),
            provider_type: entry.provider_type(),
            category: crate::providers::catalog::get_provider_setup_category(&metadata.name)
                .unwrap_or(ProviderSetupCategory::Model),
            config_keys: metadata.config_keys.clone(),
            setup_steps: metadata.setup_steps.clone(),
            supports_refresh: entry.supports_inventory_refresh(),
            static_models: metadata.known_models,
            model_selection_hint: metadata.model_selection_hint,
        }))
    }

    async fn require_provider(&self, provider_id: &str) -> Result<ProviderDescriptor> {
        self.describe_provider(provider_id)
            .await?
            .ok_or_else(|| anyhow::anyhow!("Unknown provider: {}", provider_id))
    }

    async fn mark_refresh_started(&self, identity: &InventoryIdentity) -> Result<()> {
        let existing = self.read_snapshot(identity).await?;

        sqlx::query(
            r#"
            INSERT INTO provider_inventory_entries (
                inventory_key,
                provider_id,
                provider_family,
                last_updated_at,
                last_refresh_attempt_at,
                last_refresh_error,
                updated_at
            ) VALUES (?, ?, ?, ?, ?, NULL, CURRENT_TIMESTAMP)
            ON CONFLICT(inventory_key) DO UPDATE SET
                provider_id = excluded.provider_id,
                provider_family = excluded.provider_family,
                last_updated_at = excluded.last_updated_at,
                last_refresh_attempt_at = excluded.last_refresh_attempt_at,
                last_refresh_error = NULL,
                updated_at = CURRENT_TIMESTAMP
            "#,
        )
        .bind(&identity.inventory_key)
        .bind(&identity.provider_id)
        .bind(&identity.provider_family)
        .bind(existing.and_then(|snapshot| snapshot.last_updated_at.map(|time| time.to_rfc3339())))
        .bind(Utc::now().to_rfc3339())
        .execute(self.storage.pool().await?)
        .await?;

        Ok(())
    }

    async fn read_snapshot(
        &self,
        identity: &InventoryIdentity,
    ) -> Result<Option<InventorySnapshot>> {
        let pool = self.storage.pool().await?;
        let entry = sqlx::query(
            r#"
            SELECT last_updated_at, last_refresh_attempt_at, last_refresh_error
            FROM provider_inventory_entries
            WHERE inventory_key = ?
            "#,
        )
        .bind(&identity.inventory_key)
        .fetch_optional(pool)
        .await?;

        let Some(entry) = entry else {
            return Ok(None);
        };

        let last_updated_at = parse_optional_datetime(entry.try_get("last_updated_at")?)?;
        let last_refresh_attempt_at =
            parse_optional_datetime(entry.try_get("last_refresh_attempt_at")?)?;
        let last_refresh_error = entry.try_get("last_refresh_error")?;

        let rows = sqlx::query(
            r#"
            SELECT model_id, name, family, context_limit, reasoning, recommended
            FROM provider_inventory_models
            WHERE inventory_key = ?
            ORDER BY ordinal
            "#,
        )
        .bind(&identity.inventory_key)
        .fetch_all(pool)
        .await?;

        let models = rows
            .into_iter()
            .map(|row| {
                Ok(InventoryModel {
                    id: row.try_get("model_id")?,
                    name: row.try_get("name")?,
                    family: row.try_get("family")?,
                    context_limit: row
                        .try_get::<Option<i64>, _>("context_limit")?
                        .map(usize::try_from)
                        .transpose()?,
                    reasoning: row.try_get("reasoning")?,
                    recommended: row
                        .try_get::<Option<bool>, _>("recommended")?
                        .unwrap_or(false),
                })
            })
            .collect::<Result<Vec<_>, anyhow::Error>>()?;

        Ok(Some(InventorySnapshot {
            models,
            last_updated_at,
            last_refresh_attempt_at,
            last_refresh_error,
        }))
    }

    async fn resolve_provider_ids(&self, provider_ids: &[String]) -> Vec<String> {
        let mut ids = if provider_ids.is_empty() {
            crate::providers::providers()
                .await
                .into_iter()
                .map(|(metadata, _)| metadata.name)
                .collect::<Vec<_>>()
        } else {
            provider_ids.to_vec()
        };
        ids.sort();
        ids.dedup();
        ids
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_identity(provider_id: &str, inventory_key: &str) -> InventoryIdentity {
        InventoryIdentity {
            provider_id: provider_id.to_string(),
            provider_family: provider_id.to_string(),
            inventory_key: inventory_key.to_string(),
        }
    }

    #[test]
    fn refresh_guard_complete_clears_refreshing_key() {
        let refreshing_keys = Arc::new(RwLock::new(HashSet::from(["key-a".to_string()])));
        let mut guard = RefreshGuard {
            inventory_key: "key-a".to_string(),
            refreshing_keys: Arc::clone(&refreshing_keys),
            completed: false,
        };

        guard.complete();
        guard.complete();

        assert!(!refreshing_keys.read().unwrap().contains("key-a"));
    }

    #[tokio::test]
    async fn clear_refreshing_many_removes_all_inserted_keys() {
        let service =
            ProviderInventoryService::new(Arc::new(SessionStorage::new(std::env::temp_dir())));
        let left = test_identity("openai", "key-a");
        let right = test_identity("anthropic", "key-b");
        {
            let mut refreshing_keys = service.refreshing_keys.write().unwrap();
            refreshing_keys.insert(left.inventory_key.clone());
            refreshing_keys.insert(right.inventory_key.clone());
        }

        service.clear_refreshing_many(&[left, right]);

        assert!(service.refreshing_keys.read().unwrap().is_empty());
    }

    #[tokio::test]
    async fn identity_store_writes_to_captured_inventory_key() {
        let temp_dir = tempfile::tempdir().unwrap();
        let service = ProviderInventoryService::new(Arc::new(SessionStorage::new(
            temp_dir.path().to_path_buf(),
        )));
        let plan_time_identity = test_identity("openai", "plan-time-key");
        let current_identity = test_identity("openai", "current-key");
        let sentinel_model = "stark-plan-time-model".to_string();

        service
            .store_refreshed_models_for_identity(
                &plan_time_identity,
                std::slice::from_ref(&sentinel_model),
            )
            .await
            .unwrap();

        let plan_time_snapshot = service
            .read_snapshot(&plan_time_identity)
            .await
            .unwrap()
            .unwrap();
        assert!(plan_time_snapshot
            .models
            .iter()
            .any(|model| model.id == sentinel_model));
        assert!(service
            .read_snapshot(&current_identity)
            .await
            .unwrap()
            .is_none());
    }

    #[test]
    fn inventory_identity_hash_changes_with_secret_inputs() {
        let left = InventoryIdentityInput::new("openai", "openai")
            .with_public("host", "https://api.openai.com")
            .with_secret("api_key", "secret-a")
            .into_identity()
            .unwrap();
        let right = InventoryIdentityInput::new("openai", "openai")
            .with_public("host", "https://api.openai.com")
            .with_secret("api_key", "secret-b")
            .into_identity()
            .unwrap();

        assert_ne!(left.inventory_key, right.inventory_key);
    }

    #[test]
    fn configured_models_use_canonical_enrichment() {
        let models =
            configured_models_to_inventory("anthropic", &[ModelInfo::new("claude-sonnet-4-5", 0)]);

        assert_eq!(models.len(), 1);
        assert!(models[0].name.contains("Claude"));
    }

    #[test]
    fn inventory_uses_configured_models_before_first_successful_refresh() {
        let configured_models = [ModelInfo::new("claude-sonnet-4-5", 0)];
        let snapshot = InventorySnapshot {
            models: vec![],
            last_updated_at: None,
            last_refresh_attempt_at: Some(Utc::now()),
            last_refresh_error: Some("auth failed".to_string()),
        };

        let models =
            inventory_models_from_snapshot(Some(&snapshot), "anthropic", &configured_models);

        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "claude-sonnet-4-5");
    }

    #[test]
    fn inventory_preserves_empty_models_after_successful_refresh() {
        let configured_models = [ModelInfo::new("claude-sonnet-4-5", 0)];
        let snapshot = InventorySnapshot {
            models: vec![],
            last_updated_at: Some(Utc::now()),
            last_refresh_attempt_at: Some(Utc::now()),
            last_refresh_error: None,
        };

        let models =
            inventory_models_from_snapshot(Some(&snapshot), "anthropic", &configured_models);

        assert!(models.is_empty());
    }
}
