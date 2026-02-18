use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{OnceCell, RwLock};

use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};
use sqlx::{Pool, Row, Sqlite};

/// Registry tracking which agents are enabled, their delegation strategies,
/// and which extensions are bound to each agent.
///
/// Backed by SQLite for persistence across restarts, with an in-memory cache
/// for fast reads. Falls back to pure in-memory mode for tests.
#[derive(Clone)]
pub struct AgentSlotRegistry {
    enabled_agents: Arc<RwLock<HashMap<String, bool>>>,
    bound_extensions: Arc<RwLock<HashMap<String, HashSet<String>>>>,
    delegation_strategies: Arc<RwLock<HashMap<String, SlotDelegation>>>,
    pool: Arc<OnceCell<Pool<Sqlite>>>,
    db_path: PathBuf,
}

/// How a particular agent slot should be executed.
#[derive(Clone, Debug, PartialEq)]
#[allow(dead_code)]
pub enum SlotDelegation {
    /// Execute in-process via the local provider (builtin agents).
    InProcess,
    /// Execute via an external ACP agent process.
    ExternalAcp,
    /// Execute via a remote A2A agent over HTTP.
    RemoteA2A { url: String },
}

impl Default for AgentSlotRegistry {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl AgentSlotRegistry {
    /// Create an in-memory-only registry (for tests or when no persistence needed).
    pub fn new() -> Self {
        let mut enabled = HashMap::new();
        enabled.insert("Goose Agent".to_string(), true);
        enabled.insert("Developer Agent".to_string(), true);

        let mut strategies = HashMap::new();
        strategies.insert("Goose Agent".to_string(), SlotDelegation::InProcess);
        strategies.insert("Developer Agent".to_string(), SlotDelegation::InProcess);

        Self {
            enabled_agents: Arc::new(RwLock::new(enabled)),
            bound_extensions: Arc::new(RwLock::new(HashMap::new())),
            delegation_strategies: Arc::new(RwLock::new(strategies)),
            pool: Arc::new(OnceCell::new()),
            db_path: PathBuf::new(),
        }
    }

    /// Create a registry with SQLite persistence at the given data directory.
    pub fn with_persistence(data_dir: &Path) -> Self {
        let registry_dir = data_dir.join("registry");
        std::fs::create_dir_all(&registry_dir).ok();
        let db_path = registry_dir.join("agents.db");

        Self {
            enabled_agents: Arc::new(RwLock::new(HashMap::new())),
            bound_extensions: Arc::new(RwLock::new(HashMap::new())),
            delegation_strategies: Arc::new(RwLock::new(HashMap::new())),
            pool: Arc::new(OnceCell::new()),
            db_path,
        }
    }

    /// Initialize persistence: create tables and load state from DB.
    /// Call this once at startup for persistent registries.
    pub async fn init(&self) -> anyhow::Result<()> {
        if self.db_path.as_os_str().is_empty() {
            return Ok(());
        }

        let pool = self.get_or_init_pool().await?;
        Self::run_migrations(pool).await?;
        self.load_from_db(pool).await?;

        // Ensure builtins exist
        let agents = self.enabled_agents.read().await;
        if !agents.contains_key("Goose Agent") {
            drop(agents);
            self.register_builtin("Goose Agent").await;
        } else {
            drop(agents);
        }
        let agents = self.enabled_agents.read().await;
        if !agents.contains_key("Developer Agent") {
            drop(agents);
            self.register_builtin("Developer Agent").await;
        }

        Ok(())
    }

    async fn register_builtin(&self, name: &str) {
        self.enabled_agents
            .write()
            .await
            .insert(name.to_string(), true);
        self.delegation_strategies
            .write()
            .await
            .insert(name.to_string(), SlotDelegation::InProcess);

        if let Ok(pool) = self.get_or_init_pool().await {
            sqlx::query(
                "INSERT OR IGNORE INTO agents (name, enabled, delegation_type) VALUES (?, 1, 'in_process')",
            )
            .bind(name)
            .execute(pool)
            .await
            .ok();
        }
    }

    async fn get_or_init_pool(&self) -> anyhow::Result<&Pool<Sqlite>> {
        self.pool
            .get_or_try_init(|| async {
                let options = SqliteConnectOptions::new()
                    .filename(&self.db_path)
                    .create_if_missing(true)
                    .journal_mode(SqliteJournalMode::Wal);
                let pool = SqlitePoolOptions::new()
                    .max_connections(2)
                    .connect_lazy_with(options);
                Ok::<Pool<Sqlite>, anyhow::Error>(pool)
            })
            .await
    }

    async fn run_migrations(pool: &Pool<Sqlite>) -> anyhow::Result<()> {
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agents (
                name TEXT PRIMARY KEY,
                enabled INTEGER NOT NULL DEFAULT 1,
                delegation_type TEXT NOT NULL DEFAULT 'in_process',
                delegation_url TEXT,
                created_at TEXT NOT NULL DEFAULT (datetime('now')),
                updated_at TEXT NOT NULL DEFAULT (datetime('now'))
            )"#,
        )
        .execute(pool)
        .await?;

        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS agent_extensions (
                agent_name TEXT NOT NULL,
                extension_name TEXT NOT NULL,
                PRIMARY KEY (agent_name, extension_name),
                FOREIGN KEY (agent_name) REFERENCES agents(name) ON DELETE CASCADE
            )"#,
        )
        .execute(pool)
        .await?;

        Ok(())
    }

    async fn load_from_db(&self, pool: &Pool<Sqlite>) -> anyhow::Result<()> {
        let rows = sqlx::query("SELECT name, enabled, delegation_type, delegation_url FROM agents")
            .fetch_all(pool)
            .await?;

        let mut enabled = self.enabled_agents.write().await;
        let mut strategies = self.delegation_strategies.write().await;

        for row in rows {
            let name: String = row.get("name");
            let is_enabled: bool = row.get::<i32, _>("enabled") != 0;
            let delegation_type: String = row.get("delegation_type");
            let delegation_url: Option<String> = row.get("delegation_url");

            enabled.insert(name.clone(), is_enabled);

            let strategy = match delegation_type.as_str() {
                "external_acp" => SlotDelegation::ExternalAcp,
                "remote_a2a" => SlotDelegation::RemoteA2A {
                    url: delegation_url.unwrap_or_default(),
                },
                _ => SlotDelegation::InProcess,
            };
            strategies.insert(name, strategy);
        }

        let ext_rows = sqlx::query("SELECT agent_name, extension_name FROM agent_extensions")
            .fetch_all(pool)
            .await?;

        let mut extensions = self.bound_extensions.write().await;
        for row in ext_rows {
            let agent: String = row.get("agent_name");
            let ext: String = row.get("extension_name");
            extensions.entry(agent).or_default().insert(ext);
        }

        Ok(())
    }

    async fn persist_agent(&self, name: &str, enabled: bool, delegation: &SlotDelegation) {
        if let Ok(pool) = self.get_or_init_pool().await {
            let (dtype, url) = match delegation {
                SlotDelegation::InProcess => ("in_process", None),
                SlotDelegation::ExternalAcp => ("external_acp", None),
                SlotDelegation::RemoteA2A { url } => ("remote_a2a", Some(url.as_str())),
            };
            sqlx::query(
                r#"INSERT INTO agents (name, enabled, delegation_type, delegation_url, updated_at)
                   VALUES (?, ?, ?, ?, datetime('now'))
                   ON CONFLICT(name) DO UPDATE SET
                     enabled = excluded.enabled,
                     delegation_type = excluded.delegation_type,
                     delegation_url = excluded.delegation_url,
                     updated_at = datetime('now')"#,
            )
            .bind(name)
            .bind(enabled as i32)
            .bind(dtype)
            .bind(url)
            .execute(pool)
            .await
            .ok();
        }
    }

    pub async fn is_enabled(&self, name: &str) -> bool {
        self.enabled_agents
            .read()
            .await
            .get(name)
            .copied()
            .unwrap_or(true)
    }

    pub async fn toggle(&self, name: &str) -> bool {
        let mut agents = self.enabled_agents.write().await;
        let current = agents.get(name).copied().unwrap_or(true);
        let new_state = !current;
        agents.insert(name.to_string(), new_state);
        drop(agents);

        let delegation = self.get_delegation(name).await;
        self.persist_agent(name, new_state, &delegation).await;

        new_state
    }

    pub async fn bind_extension(&self, agent: &str, extension: &str) {
        self.bound_extensions
            .write()
            .await
            .entry(agent.to_string())
            .or_default()
            .insert(extension.to_string());

        if let Ok(pool) = self.get_or_init_pool().await {
            sqlx::query(
                "INSERT OR IGNORE INTO agent_extensions (agent_name, extension_name) VALUES (?, ?)",
            )
            .bind(agent)
            .bind(extension)
            .execute(pool)
            .await
            .ok();
        }
    }

    pub async fn unbind_extension(&self, agent: &str, extension: &str) {
        if let Some(exts) = self.bound_extensions.write().await.get_mut(agent) {
            exts.remove(extension);
        }

        if let Ok(pool) = self.get_or_init_pool().await {
            sqlx::query("DELETE FROM agent_extensions WHERE agent_name = ? AND extension_name = ?")
                .bind(agent)
                .bind(extension)
                .execute(pool)
                .await
                .ok();
        }
    }

    pub async fn get_bound_extensions(&self, agent: &str) -> Vec<String> {
        self.bound_extensions
            .read()
            .await
            .get(agent)
            .map(|s| s.iter().cloned().collect())
            .unwrap_or_default()
    }

    pub async fn get_delegation(&self, name: &str) -> SlotDelegation {
        self.delegation_strategies
            .read()
            .await
            .get(name)
            .cloned()
            .unwrap_or(SlotDelegation::InProcess)
    }

    pub async fn set_delegation(&self, name: &str, strategy: SlotDelegation) {
        let enabled = self.is_enabled(name).await;
        self.persist_agent(name, enabled, &strategy).await;
        self.delegation_strategies
            .write()
            .await
            .insert(name.to_string(), strategy);
    }

    pub async fn register_a2a_agent(&self, name: &str, url: &str) {
        let delegation = SlotDelegation::RemoteA2A {
            url: url.to_string(),
        };
        self.enabled_agents
            .write()
            .await
            .insert(name.to_string(), true);
        self.delegation_strategies
            .write()
            .await
            .insert(name.to_string(), delegation.clone());
        self.persist_agent(name, true, &delegation).await;
    }

    pub async fn register_acp_agent(&self, name: &str) {
        let delegation = SlotDelegation::ExternalAcp;
        self.enabled_agents
            .write()
            .await
            .insert(name.to_string(), true);
        self.delegation_strategies
            .write()
            .await
            .insert(name.to_string(), delegation.clone());
        self.persist_agent(name, true, &delegation).await;
    }

    pub async fn all_agents(&self) -> Vec<(String, bool, SlotDelegation)> {
        let enabled = self.enabled_agents.read().await;
        let strategies = self.delegation_strategies.read().await;
        enabled
            .iter()
            .map(|(name, is_enabled)| {
                let delegation = strategies
                    .get(name)
                    .cloned()
                    .unwrap_or(SlotDelegation::InProcess);
                (name.clone(), *is_enabled, delegation)
            })
            .collect()
    }

    pub async fn all_agent_names(&self) -> Vec<String> {
        self.enabled_agents.read().await.keys().cloned().collect()
    }

    pub async fn unregister_agent(&self, name: &str) {
        self.enabled_agents.write().await.remove(name);
        self.delegation_strategies.write().await.remove(name);
        self.bound_extensions.write().await.remove(name);

        if let Ok(pool) = self.get_or_init_pool().await {
            sqlx::query("DELETE FROM agents WHERE name = ?")
                .bind(name)
                .execute(pool)
                .await
                .ok();
            sqlx::query("DELETE FROM agent_extensions WHERE agent_name = ?")
                .bind(name)
                .execute(pool)
                .await
                .ok();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_toggle_agent() {
        let registry = AgentSlotRegistry::new();
        assert!(registry.is_enabled("Goose Agent").await);
        let new_state = registry.toggle("Goose Agent").await;
        assert!(!new_state);
        assert!(!registry.is_enabled("Goose Agent").await);
        let new_state = registry.toggle("Goose Agent").await;
        assert!(new_state);
    }

    #[tokio::test]
    async fn test_bind_unbind_extension() {
        let registry = AgentSlotRegistry::new();
        assert!(registry
            .get_bound_extensions("Goose Agent")
            .await
            .is_empty());
        registry.bind_extension("Goose Agent", "developer").await;
        registry.bind_extension("Goose Agent", "memory").await;
        let exts = registry.get_bound_extensions("Goose Agent").await;
        assert_eq!(exts.len(), 2);
        assert!(exts.contains(&"developer".to_string()));
        registry.unbind_extension("Goose Agent", "developer").await;
        let exts = registry.get_bound_extensions("Goose Agent").await;
        assert_eq!(exts.len(), 1);
    }

    #[tokio::test]
    async fn test_delegation_defaults_to_in_process() {
        let registry = AgentSlotRegistry::new();
        assert_eq!(
            registry.get_delegation("Goose Agent").await,
            SlotDelegation::InProcess
        );
        assert_eq!(
            registry.get_delegation("Unknown Agent").await,
            SlotDelegation::InProcess
        );
    }

    #[tokio::test]
    async fn test_register_a2a_agent() {
        let registry = AgentSlotRegistry::new();
        registry
            .register_a2a_agent("Remote Agent", "https://remote.example.com/a2a")
            .await;
        assert!(registry.is_enabled("Remote Agent").await);
        assert_eq!(
            registry.get_delegation("Remote Agent").await,
            SlotDelegation::RemoteA2A {
                url: "https://remote.example.com/a2a".to_string()
            }
        );
    }

    #[tokio::test]
    async fn test_register_acp_agent() {
        let registry = AgentSlotRegistry::new();
        registry.register_acp_agent("ACP Agent").await;
        assert!(registry.is_enabled("ACP Agent").await);
        assert_eq!(
            registry.get_delegation("ACP Agent").await,
            SlotDelegation::ExternalAcp
        );
    }

    #[tokio::test]
    async fn test_unregister_agent() {
        let registry = AgentSlotRegistry::new();
        registry
            .register_a2a_agent("Temp Agent", "https://example.com")
            .await;
        registry.bind_extension("Temp Agent", "developer").await;
        assert_eq!(
            registry.get_delegation("Temp Agent").await,
            SlotDelegation::RemoteA2A {
                url: "https://example.com".to_string()
            }
        );

        registry.unregister_agent("Temp Agent").await;
        assert_eq!(
            registry.get_delegation("Temp Agent").await,
            SlotDelegation::InProcess
        );
        assert!(registry.get_bound_extensions("Temp Agent").await.is_empty());
    }

    #[tokio::test]
    async fn test_all_agents() {
        let registry = AgentSlotRegistry::new();
        registry
            .register_a2a_agent("Remote", "https://example.com")
            .await;
        let all = registry.all_agents().await;
        assert!(all.len() >= 3); // Goose Agent, Developer Agent, Remote
        assert!(all.iter().any(|(name, _, _)| name == "Remote"));
    }

    #[tokio::test]
    async fn test_persistence_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let registry = AgentSlotRegistry::with_persistence(dir.path());
        registry.init().await.unwrap();

        registry
            .register_a2a_agent("Persistent Agent", "https://p.example.com")
            .await;
        registry
            .bind_extension("Persistent Agent", "developer")
            .await;
        registry.toggle("Goose Agent").await; // disable

        // Create a new registry from the same DB
        let registry2 = AgentSlotRegistry::with_persistence(dir.path());
        registry2.init().await.unwrap();

        assert!(registry2.is_enabled("Persistent Agent").await);
        assert_eq!(
            registry2.get_delegation("Persistent Agent").await,
            SlotDelegation::RemoteA2A {
                url: "https://p.example.com".to_string()
            }
        );
        let exts = registry2.get_bound_extensions("Persistent Agent").await;
        assert_eq!(exts.len(), 1);
        assert!(exts.contains(&"developer".to_string()));
        assert!(!registry2.is_enabled("Goose Agent").await);
    }
}
