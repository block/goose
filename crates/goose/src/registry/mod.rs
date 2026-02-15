pub mod formats;
pub mod install;
pub mod manifest;
pub mod publish;
pub mod source;
pub mod sources;

use std::collections::HashMap;

use anyhow::Result;
use manifest::{RegistryEntry, RegistryEntryKind};
use source::RegistrySource;

/// Aggregates multiple registry sources into a unified search interface.
pub struct RegistryManager {
    sources: Vec<Box<dyn RegistrySource>>,
}

impl RegistryManager {
    pub fn new() -> Self {
        Self {
            sources: Vec::new(),
        }
    }

    pub fn add_source(&mut self, source: Box<dyn RegistrySource>) {
        self.sources.push(source);
    }

    pub fn source_names(&self) -> Vec<String> {
        self.sources.iter().map(|s| s.name().to_string()).collect()
    }

    pub async fn search(
        &self,
        query: Option<&str>,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Vec<RegistryEntry>> {
        let mut results: Vec<RegistryEntry> = Vec::new();
        let mut seen: HashMap<(RegistryEntryKind, String), usize> = HashMap::new();

        for source in &self.sources {
            let entries = source.search(query, kind).await?;
            for entry in entries {
                let key = (entry.kind, entry.name.clone());
                if let Some(&idx) = seen.get(&key) {
                    results[idx].merge_metadata(&entry);
                } else {
                    seen.insert(key, results.len());
                    results.push(entry);
                }
            }
        }

        Ok(results)
    }

    pub async fn list(&self, kind: Option<RegistryEntryKind>) -> Result<Vec<RegistryEntry>> {
        self.search(None, kind).await
    }

    pub async fn get(
        &self,
        name: &str,
        kind: Option<RegistryEntryKind>,
    ) -> Result<Option<RegistryEntry>> {
        for source in &self.sources {
            if let Some(entry) = source.get(name, kind).await? {
                return Ok(Some(entry));
            }
        }
        Ok(None)
    }
}

impl Default for RegistryManager {
    fn default() -> Self {
        Self::new()
    }
}
