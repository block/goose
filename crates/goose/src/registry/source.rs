use anyhow::Result;
use async_trait::async_trait;

use super::manifest::{RegistryEntry, RegistryEntryKind};

/// A pluggable source of registry entries.
///
/// Implementations scan a specific backend (filesystem, GitHub, HTTP endpoint)
/// and return `RegistryEntry` items matching the query.
///
/// Sources are ordered by priority in `RegistryManager`: local sources first,
/// then remote. The first match for a given (name, kind) pair wins.
#[async_trait]
pub trait RegistrySource: Send + Sync {
    /// Human-readable name for this source (e.g. "local", "github:block/goose").
    fn name(&self) -> &str;

    /// Search for entries matching the query string and optional kind filter.
    /// A `None` query returns all entries.
    async fn search(
        &self,
        query: Option<&str>,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Vec<RegistryEntry>>;

    /// Get a specific entry by exact name and optional kind filter.
    async fn get(
        &self,
        name: &str,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Option<RegistryEntry>>;
}
