use super::service::{RepoIndexService, StoredEntity};

/// Summary information about the indexed repository.
#[derive(Debug, Clone)]
pub struct RepoSummary {
    pub files: usize,
    pub entities: usize,
}

/// Trait abstraction for repository search & graph queries (Step 3).
pub trait RepoSearchTool {
    /// Exact symbol lookup (case-insensitive) returning entity ids.
    fn search_symbol_exact_ids(&self, name: &str) -> Vec<u32>;
    /// Exact symbol lookup returning entity references.
    fn search_symbol_exact(&self, name: &str) -> Vec<&StoredEntity>;
    /// Depth-limited forward (callee) traversal.
    fn callees_up_to(&self, entity_id: u32, depth: u32) -> Vec<u32>;
    /// Depth-limited reverse (caller) traversal.
    fn callers_up_to(&self, entity_id: u32, depth: u32) -> Vec<u32>;
    /// Access an entity by id.
    fn entity(&self, id: u32) -> Option<&StoredEntity>;
    /// Return overall summary.
    fn summary(&self) -> RepoSummary;
}

impl RepoSearchTool for RepoIndexService {
    fn search_symbol_exact_ids(&self, name: &str) -> Vec<u32> {
        self.symbol_ids_exact(name).to_vec()
    }
    fn search_symbol_exact(&self, name: &str) -> Vec<&StoredEntity> {
        RepoIndexService::search_symbol_exact(self, name)
    }
    fn callees_up_to(&self, entity_id: u32, depth: u32) -> Vec<u32> { self.callees_up_to(entity_id, depth) }
    fn callers_up_to(&self, entity_id: u32, depth: u32) -> Vec<u32> { self.callers_up_to(entity_id, depth) }
    fn entity(&self, id: u32) -> Option<&StoredEntity> { self.entities.get(id as usize) }
    fn summary(&self) -> RepoSummary { RepoSummary { files: self.files.len(), entities: self.entities.len() } }
}
