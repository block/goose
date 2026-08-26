use crate::config::paths::Paths;
use anyhow::{Context, Result};
use fs2::FileExt;
use rmcp::model::Tool;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::{self, File, OpenOptions};
use std::path::{Path, PathBuf};
use std::sync::{Arc, LazyLock, Mutex, RwLock, Weak};
use tracing;

const PERMISSION_FILE: &str = "permission.yaml";
const PERMISSION_LOCK_FILE: &str = "permission.lock";

static PERMISSION_MANAGER: LazyLock<Arc<PermissionManager>> =
    LazyLock::new(|| Arc::new(PermissionManager::new(Paths::config_dir())));
static PERMISSION_STATES: LazyLock<Mutex<HashMap<PathBuf, Weak<SharedPermissionState>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Enum representing the possible permission levels for a tool.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PermissionLevel {
    AlwaysAllow, // Tool can always be used without prompt
    AskBefore,   // Tool requires permission to be granted before use
    NeverAllow,  // Tool is never allowed to be used
}

/// Struct representing the configuration of permissions, categorized by level.
#[derive(Debug, Deserialize, Serialize, Default, Clone)]
pub struct PermissionConfig {
    pub always_allow: Vec<String>, // List of tools that are always allowed
    pub ask_before: Vec<String>,   // List of tools that require user consent
    pub never_allow: Vec<String>,  // List of tools that are never allowed
}

#[derive(Debug)]
struct SharedPermissionState {
    file_lock: Mutex<()>,
    permission_map: RwLock<HashMap<String, PermissionConfig>>,
}

/// PermissionManager manages permission configurations for various tools.
#[derive(Debug)]
pub struct PermissionManager {
    config_path: PathBuf,
    state: Arc<SharedPermissionState>,
}

#[derive(Debug, Clone)]
pub struct PermissionSnapshot {
    permission_map: Option<HashMap<String, PermissionConfig>>,
}

// Constants representing specific permission categories
const USER_PERMISSION: &str = "user";
const SMART_APPROVE_PERMISSION: &str = "smart_approve";

impl PermissionManager {
    pub fn new(config_dir: PathBuf) -> Self {
        let permission_path = config_dir.join(PERMISSION_FILE);
        let state = Self::shared_state(&config_dir, &permission_path);
        PermissionManager {
            config_path: permission_path,
            state,
        }
    }

    fn shared_state(config_dir: &Path, config_path: &Path) -> Arc<SharedPermissionState> {
        let mut states = PERMISSION_STATES
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        states.retain(|_, state| state.strong_count() > 0);
        if let Some(state) = states.get(config_path).and_then(Weak::upgrade) {
            return state;
        }

        let permission_map = if config_path.exists() {
            Self::load_permission_map(config_path)
        } else {
            // Consolidate directory creation for re-use in global singleton or ACP.
            fs::create_dir_all(config_dir).expect("Failed to create config directory");
            HashMap::new()
        };
        let state = Arc::new(SharedPermissionState {
            file_lock: Mutex::new(()),
            permission_map: RwLock::new(permission_map),
        });
        states.insert(config_path.to_path_buf(), Arc::downgrade(&state));
        state
    }

    fn lock_permission_file_for_read(config_path: &Path) -> Result<Option<File>> {
        let lock_path = config_path.with_file_name(PERMISSION_LOCK_FILE);
        let lock_file = match OpenOptions::new().read(true).open(&lock_path) {
            Ok(lock_file) => lock_file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => {
                return Err(error)
                    .with_context(|| format!("Failed to open {}", lock_path.display()));
            }
        };
        FileExt::lock_shared(&lock_file)
            .with_context(|| format!("Failed to lock {}", lock_path.display()))?;
        Ok(Some(lock_file))
    }

    fn lock_permission_file_for_write(config_path: &Path) -> Result<File> {
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(config_path.with_file_name(PERMISSION_LOCK_FILE))
            .context("Failed to open permission lock file")?;
        lock_file
            .lock_exclusive()
            .context("Failed to lock permission file")?;
        Ok(lock_file)
    }

    fn load_permission_map(config_path: &Path) -> HashMap<String, PermissionConfig> {
        Self::read_permission_map(config_path).unwrap_or_else(|error| {
            tracing::error!(
                "Failed to parse {}: {}. Refusing to start with corrupted permission config.",
                config_path.display(),
                error,
            );
            panic!(
                "Corrupted permission config at {}. Fix or remove the file to continue.",
                config_path.display(),
            );
        })
    }

    fn read_permission_map(config_path: &Path) -> Result<HashMap<String, PermissionConfig>> {
        let lock_path = config_path.with_file_name(PERMISSION_LOCK_FILE);
        loop {
            let file_guard = Self::lock_permission_file_for_read(config_path)?;
            let result = if config_path.exists() {
                Self::try_load_permission_map(config_path)
            } else {
                Ok(HashMap::new())
            };
            if file_guard.is_some() || !lock_path.exists() {
                return result;
            }
        }
    }

    fn try_load_permission_map(config_path: &Path) -> Result<HashMap<String, PermissionConfig>> {
        let file_contents = fs::read_to_string(config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;
        serde_yaml::from_str(&file_contents)
            .with_context(|| format!("Failed to parse {}", config_path.display()))
    }

    fn modify_permission_map(
        &self,
        modify: impl FnOnce(&mut HashMap<String, PermissionConfig>),
    ) -> Result<()> {
        let _process_guard = self.state.file_lock.lock().unwrap();
        let _file_guard = Self::lock_permission_file_for_write(&self.config_path)?;
        let mut map = self.state.permission_map.write().unwrap();
        if self.config_path.exists() {
            *map = Self::try_load_permission_map(&self.config_path)?;
        } else {
            map.clear();
        }
        modify(&mut map);
        let yaml_content = serde_yaml::to_string(&*map)?;
        fs::write(&self.config_path, yaml_content)
            .with_context(|| format!("Failed to write {}", self.config_path.display()))?;
        Ok(())
    }

    fn refresh_permission_map(&self) -> Result<()> {
        let _process_guard = self.state.file_lock.lock().unwrap();
        let refreshed = Self::read_permission_map(&self.config_path)?;
        let mut map = self.state.permission_map.write().unwrap();
        *map = refreshed;
        Ok(())
    }

    pub fn instance() -> Arc<PermissionManager> {
        Arc::clone(&PERMISSION_MANAGER)
    }

    pub fn snapshot(&self) -> PermissionSnapshot {
        let permission_map = self
            .refresh_permission_map()
            .ok()
            .map(|()| self.state.permission_map.read().unwrap().clone());
        PermissionSnapshot { permission_map }
    }

    /// Returns a list of all the names (keys) in the permission map.
    pub fn get_permission_names(&self) -> Vec<String> {
        if self.refresh_permission_map().is_err() {
            return Vec::new();
        }
        self.state
            .permission_map
            .read()
            .unwrap()
            .keys()
            .cloned()
            .collect()
    }

    /// Retrieves the user permission level for a specific tool.
    pub fn get_user_permission(&self, principal_name: &str) -> Option<PermissionLevel> {
        self.get_permission(USER_PERMISSION, principal_name)
    }

    /// Retrieves the smart approve permission level for a specific tool.
    pub fn get_smart_approve_permission(&self, principal_name: &str) -> Option<PermissionLevel> {
        self.get_permission(SMART_APPROVE_PERMISSION, principal_name)
    }

    /// Retrieves the config file path.
    pub fn get_config_path(&self) -> &Path {
        self.config_path.as_path()
    }

    pub fn apply_tool_annotations(&self, tools: &[Tool]) -> Result<()> {
        let mut write_annotated = Vec::new();
        for tool in tools {
            let Some(anns) = &tool.annotations else {
                continue;
            };
            if anns.read_only_hint == Some(false) {
                write_annotated.push(tool.name.to_string());
            }
        }
        if !write_annotated.is_empty() {
            self.bulk_update_smart_approve_permissions(
                &write_annotated,
                PermissionLevel::AskBefore,
            )?;
        }
        Ok(())
    }

    fn bulk_update_smart_approve_permissions(
        &self,
        tool_names: &[String],
        level: PermissionLevel,
    ) -> Result<()> {
        self.modify_permission_map(|map| {
            let permission_config = map.entry(SMART_APPROVE_PERMISSION.to_string()).or_default();

            for tool_name in tool_names {
                // Remove from all lists to avoid duplicates
                permission_config.always_allow.retain(|p| p != tool_name);
                permission_config.ask_before.retain(|p| p != tool_name);
                permission_config.never_allow.retain(|p| p != tool_name);

                // Add to the appropriate list
                match &level {
                    PermissionLevel::AlwaysAllow => {
                        permission_config.always_allow.push(tool_name.clone())
                    }
                    PermissionLevel::AskBefore => {
                        permission_config.ask_before.push(tool_name.clone())
                    }
                    PermissionLevel::NeverAllow => {
                        permission_config.never_allow.push(tool_name.clone())
                    }
                }
            }
        })
    }

    /// Helper function to retrieve the permission level for a specific permission category and tool.
    fn get_permission(&self, name: &str, principal_name: &str) -> Option<PermissionLevel> {
        if self.refresh_permission_map().is_err() {
            return Some(PermissionLevel::NeverAllow);
        }
        let map = self.state.permission_map.read().unwrap();
        permission_from_map(&map, name, principal_name)
    }

    /// Updates the user permission level for a specific tool.
    pub fn update_user_permission(
        &self,
        principal_name: &str,
        level: PermissionLevel,
    ) -> Result<()> {
        self.update_permission(USER_PERMISSION, principal_name, level)
    }

    /// Updates the smart approve permission level for a specific tool.
    pub fn update_smart_approve_permission(
        &self,
        principal_name: &str,
        level: PermissionLevel,
    ) -> Result<()> {
        self.update_permission(SMART_APPROVE_PERMISSION, principal_name, level)
    }

    /// Helper function to update a permission level for a specific tool in a given permission category.
    fn update_permission(
        &self,
        name: &str,
        principal_name: &str,
        level: PermissionLevel,
    ) -> Result<()> {
        self.modify_permission_map(|map| {
            // Get or create a new PermissionConfig for the specified category
            let permission_config = map.entry(name.to_string()).or_default();

            // Remove the principal from all existing lists to avoid duplicates
            permission_config
                .always_allow
                .retain(|p| p != principal_name);
            permission_config.ask_before.retain(|p| p != principal_name);
            permission_config
                .never_allow
                .retain(|p| p != principal_name);

            // Add the principal to the appropriate list
            match level {
                PermissionLevel::AlwaysAllow => permission_config
                    .always_allow
                    .push(principal_name.to_string()),
                PermissionLevel::AskBefore => permission_config
                    .ask_before
                    .push(principal_name.to_string()),
                PermissionLevel::NeverAllow => permission_config
                    .never_allow
                    .push(principal_name.to_string()),
            }
        })
    }

    pub fn remove_extension(&self, extension_name: &str) -> Result<()> {
        self.modify_permission_map(|map| {
            for permission_config in map.values_mut() {
                permission_config
                    .always_allow
                    .retain(|p| !Self::belongs_to_extension(p, extension_name));
                permission_config
                    .ask_before
                    .retain(|p| !Self::belongs_to_extension(p, extension_name));
                permission_config
                    .never_allow
                    .retain(|p| !Self::belongs_to_extension(p, extension_name));
            }
        })
    }

    pub fn clear_permissions(&self) -> Result<()> {
        self.modify_permission_map(HashMap::clear)
    }

    fn belongs_to_extension(principal_name: &str, extension_name: &str) -> bool {
        !extension_name.is_empty()
            && principal_name
                .strip_prefix(extension_name)
                .is_some_and(|suffix| suffix.starts_with("__"))
    }
}

impl PermissionSnapshot {
    pub fn get_user_permission(&self, principal_name: &str) -> Option<PermissionLevel> {
        self.get_permission(USER_PERMISSION, principal_name)
    }

    pub fn get_smart_approve_permission(&self, principal_name: &str) -> Option<PermissionLevel> {
        self.get_permission(SMART_APPROVE_PERMISSION, principal_name)
    }

    fn get_permission(&self, name: &str, principal_name: &str) -> Option<PermissionLevel> {
        self.permission_map
            .as_ref()
            .map_or(Some(PermissionLevel::NeverAllow), |map| {
                permission_from_map(map, name, principal_name)
            })
    }
}

fn permission_from_map(
    map: &HashMap<String, PermissionConfig>,
    name: &str,
    principal_name: &str,
) -> Option<PermissionLevel> {
    let permission_config = map.get(name)?;
    if permission_config
        .never_allow
        .iter()
        .any(|permission| permission == principal_name)
    {
        Some(PermissionLevel::NeverAllow)
    } else if permission_config
        .always_allow
        .iter()
        .any(|permission| permission == principal_name)
    {
        Some(PermissionLevel::AlwaysAllow)
    } else if permission_config
        .ask_before
        .iter()
        .any(|permission| permission == principal_name)
    {
        Some(PermissionLevel::AskBefore)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rmcp::model::ToolAnnotations;
    use rmcp::object;
    use std::fs::OpenOptions;
    use std::sync::mpsc;
    use std::thread;
    use std::time::Duration;
    use tempfile::TempDir;

    // Helper function to create a test instance of PermissionManager with a temp dir
    fn create_test_permission_manager() -> (PermissionManager, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let manager = PermissionManager::new(temp_dir.path().to_path_buf());
        (manager, temp_dir)
    }

    #[test]
    fn test_get_permission_names_empty() {
        let (manager, _temp_dir) = create_test_permission_manager();

        assert!(manager.get_permission_names().is_empty());
    }

    #[test]
    fn test_permission_reads_do_not_create_lock_file() {
        let temp_dir = TempDir::new().unwrap();
        let permission_path = temp_dir.path().join(PERMISSION_FILE);
        let lock_path = temp_dir.path().join(PERMISSION_LOCK_FILE);
        fs::write(
            &permission_path,
            "user:\n  always_allow:\n    - git__status\n  ask_before: []\n  never_allow: []\n",
        )
        .unwrap();

        let manager = PermissionManager::new(temp_dir.path().to_path_buf());

        assert_eq!(
            manager.get_user_permission("git__status"),
            Some(PermissionLevel::AlwaysAllow)
        );
        assert!(!lock_path.exists());
    }

    #[test]
    fn test_update_user_permission() {
        let (manager, _temp_dir) = create_test_permission_manager();
        manager
            .update_user_permission("tool1", PermissionLevel::AlwaysAllow)
            .unwrap();

        let permission = manager.get_user_permission("tool1");
        assert_eq!(permission, Some(PermissionLevel::AlwaysAllow));
    }

    #[test]
    fn test_update_smart_approve_permission() {
        let (manager, _temp_dir) = create_test_permission_manager();
        manager
            .update_smart_approve_permission("tool2", PermissionLevel::AskBefore)
            .unwrap();

        let permission = manager.get_smart_approve_permission("tool2");
        assert_eq!(permission, Some(PermissionLevel::AskBefore));
    }

    #[test]
    fn permission_update_returns_error_when_refresh_fails() {
        let (manager, _temp_dir) = create_test_permission_manager();
        manager
            .update_user_permission("tool", PermissionLevel::AlwaysAllow)
            .unwrap();
        fs::write(manager.get_config_path(), "{{invalid yaml: [broken").unwrap();

        assert!(manager
            .update_user_permission("tool", PermissionLevel::NeverAllow)
            .is_err());
    }

    #[test]
    fn test_get_permission_not_found() {
        let (manager, _temp_dir) = create_test_permission_manager();

        let permission = manager.get_user_permission("non_existent_tool");
        assert_eq!(permission, None);
    }

    #[test]
    fn test_permission_levels() {
        let (manager, _temp_dir) = create_test_permission_manager();

        manager
            .update_user_permission("tool4", PermissionLevel::AlwaysAllow)
            .unwrap();
        manager
            .update_user_permission("tool5", PermissionLevel::AskBefore)
            .unwrap();
        manager
            .update_user_permission("tool6", PermissionLevel::NeverAllow)
            .unwrap();

        // Check the permission levels
        assert_eq!(
            manager.get_user_permission("tool4"),
            Some(PermissionLevel::AlwaysAllow)
        );
        assert_eq!(
            manager.get_user_permission("tool5"),
            Some(PermissionLevel::AskBefore)
        );
        assert_eq!(
            manager.get_user_permission("tool6"),
            Some(PermissionLevel::NeverAllow)
        );
    }

    #[test]
    fn test_persisted_never_allow_takes_precedence_over_other_levels() {
        let temp_dir = TempDir::new().unwrap();
        fs::write(
            temp_dir.path().join(PERMISSION_FILE),
            r#"user:
  always_allow:
    - denied_from_allow
    - allowed
  ask_before:
    - denied_from_ask
    - prompted
  never_allow:
    - denied_from_allow
    - denied_from_ask
    - denied
"#,
        )
        .unwrap();

        let manager = PermissionManager::new(temp_dir.path().to_path_buf());

        assert_eq!(
            manager.get_user_permission("denied_from_allow"),
            Some(PermissionLevel::NeverAllow)
        );
        assert_eq!(
            manager.get_user_permission("denied_from_ask"),
            Some(PermissionLevel::NeverAllow)
        );
        assert_eq!(
            manager.get_user_permission("allowed"),
            Some(PermissionLevel::AlwaysAllow)
        );
        assert_eq!(
            manager.get_user_permission("prompted"),
            Some(PermissionLevel::AskBefore)
        );
        assert_eq!(
            manager.get_user_permission("denied"),
            Some(PermissionLevel::NeverAllow)
        );
        assert_eq!(manager.get_user_permission("unknown"), None);
    }

    #[test]
    fn test_permission_update_replaces_existing_level() {
        let (manager, _temp_dir) = create_test_permission_manager();

        // Initially AlwaysAllow
        manager
            .update_user_permission("tool7", PermissionLevel::AlwaysAllow)
            .unwrap();
        assert_eq!(
            manager.get_user_permission("tool7"),
            Some(PermissionLevel::AlwaysAllow)
        );

        // Now change to NeverAllow
        manager
            .update_user_permission("tool7", PermissionLevel::NeverAllow)
            .unwrap();
        assert_eq!(
            manager.get_user_permission("tool7"),
            Some(PermissionLevel::NeverAllow)
        );

        // Ensure it's removed from other levels
        let map = manager.state.permission_map.read().unwrap();
        let config = map.get(USER_PERMISSION).unwrap();
        assert!(!config.always_allow.contains(&"tool7".to_string()));
        assert!(!config.ask_before.contains(&"tool7".to_string()));
        assert!(config.never_allow.contains(&"tool7".to_string()));
    }

    #[test]
    fn test_remove_extension() {
        let (manager, _temp_dir) = create_test_permission_manager();
        manager
            .update_user_permission("git__status", PermissionLevel::AlwaysAllow)
            .unwrap();
        manager
            .update_user_permission("git__tool__with__delimiter", PermissionLevel::AskBefore)
            .unwrap();
        manager
            .update_user_permission("github__delete_repo", PermissionLevel::NeverAllow)
            .unwrap();
        manager
            .update_user_permission("gitlab__deploy", PermissionLevel::AskBefore)
            .unwrap();
        manager
            .update_user_permission("__cli__ent____tool", PermissionLevel::NeverAllow)
            .unwrap();

        manager.remove_extension("git").unwrap();

        assert_eq!(manager.get_user_permission("git__status"), None);
        assert_eq!(
            manager.get_user_permission("git__tool__with__delimiter"),
            None
        );
        assert_eq!(
            manager.get_user_permission("github__delete_repo"),
            Some(PermissionLevel::NeverAllow)
        );
        assert_eq!(
            manager.get_user_permission("gitlab__deploy"),
            Some(PermissionLevel::AskBefore)
        );

        manager.remove_extension("__cli__ent__").unwrap();
        assert_eq!(manager.get_user_permission("__cli__ent____tool"), None);

        manager.remove_extension("").unwrap();
        assert_eq!(
            manager.get_user_permission("github__delete_repo"),
            Some(PermissionLevel::NeverAllow)
        );

        manager.clear_permissions().unwrap();
        assert!(manager.get_permission_names().is_empty());
    }

    #[test]
    fn test_remove_extension_preserves_newer_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let deleting_manager = PermissionManager::new(temp_dir.path().to_path_buf());
        deleting_manager
            .update_user_permission("git__status", PermissionLevel::AlwaysAllow)
            .unwrap();

        let newer_manager = PermissionManager::new(temp_dir.path().to_path_buf());
        newer_manager
            .update_user_permission("github__status", PermissionLevel::AskBefore)
            .unwrap();

        deleting_manager.remove_extension("git").unwrap();

        let persisted_manager = PermissionManager::new(temp_dir.path().to_path_buf());
        assert_eq!(persisted_manager.get_user_permission("git__status"), None);
        assert_eq!(
            persisted_manager.get_user_permission("github__status"),
            Some(PermissionLevel::AskBefore)
        );
    }

    #[test]
    fn test_stale_manager_update_does_not_restore_removed_extension_permissions() {
        let temp_dir = TempDir::new().unwrap();
        let deleting_manager = PermissionManager::new(temp_dir.path().to_path_buf());
        deleting_manager
            .update_user_permission("git__status", PermissionLevel::AlwaysAllow)
            .unwrap();
        let stale_manager = PermissionManager::new(temp_dir.path().to_path_buf());

        deleting_manager.remove_extension("git").unwrap();
        assert_eq!(stale_manager.get_user_permission("git__status"), None);
        stale_manager
            .update_user_permission("github__status", PermissionLevel::AskBefore)
            .unwrap();

        let persisted_manager = PermissionManager::new(temp_dir.path().to_path_buf());
        assert_eq!(persisted_manager.get_user_permission("git__status"), None);
        assert_eq!(
            persisted_manager.get_user_permission("github__status"),
            Some(PermissionLevel::AskBefore)
        );
    }

    #[test]
    fn test_permission_updates_wait_for_process_lock() {
        let temp_dir = TempDir::new().unwrap();
        let manager = PermissionManager::new(temp_dir.path().to_path_buf());
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(temp_dir.path().join(PERMISSION_LOCK_FILE))
            .unwrap();
        lock_file.lock_exclusive().unwrap();
        let (completed_tx, completed_rx) = mpsc::channel();

        let update = thread::spawn(move || {
            manager
                .update_user_permission("github__status", PermissionLevel::AskBefore)
                .unwrap();
            completed_tx.send(()).unwrap();
        });

        assert!(completed_rx
            .recv_timeout(Duration::from_millis(100))
            .is_err());
        FileExt::unlock(&lock_file).unwrap();
        completed_rx.recv_timeout(Duration::from_secs(2)).unwrap();
        update.join().unwrap();
    }

    #[test]
    fn test_reads_refresh_permissions_changed_by_another_process() {
        let (manager, temp_dir) = create_test_permission_manager();
        manager
            .update_user_permission("git__status", PermissionLevel::AlwaysAllow)
            .unwrap();
        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(temp_dir.path().join(PERMISSION_LOCK_FILE))
            .unwrap();
        lock_file.lock_exclusive().unwrap();
        fs::write(manager.get_config_path(), "{}\n").unwrap();
        FileExt::unlock(&lock_file).unwrap();

        assert_eq!(manager.get_user_permission("git__status"), None);
    }

    #[test]
    fn permission_snapshot_refreshes_once_per_batch() {
        let (manager, temp_dir) = create_test_permission_manager();
        manager
            .update_user_permission("git__status", PermissionLevel::AlwaysAllow)
            .unwrap();
        let snapshot = manager.snapshot();

        let lock_file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(temp_dir.path().join(PERMISSION_LOCK_FILE))
            .unwrap();
        lock_file.lock_exclusive().unwrap();
        fs::write(
            manager.get_config_path(),
            "user:\n  always_allow: []\n  ask_before: []\n  never_allow:\n    - git__status\n",
        )
        .unwrap();
        FileExt::unlock(&lock_file).unwrap();

        assert_eq!(
            snapshot.get_user_permission("git__status"),
            Some(PermissionLevel::AlwaysAllow)
        );
        assert_eq!(
            manager.snapshot().get_user_permission("git__status"),
            Some(PermissionLevel::NeverAllow)
        );
    }

    #[test]
    fn permission_snapshot_fails_closed_when_refresh_fails() {
        let (manager, _temp_dir) = create_test_permission_manager();
        manager
            .update_user_permission("git__status", PermissionLevel::AlwaysAllow)
            .unwrap();
        fs::write(manager.get_config_path(), "{{invalid yaml: [broken").unwrap();

        let snapshot = manager.snapshot();
        assert_eq!(
            snapshot.get_user_permission("git__status"),
            Some(PermissionLevel::NeverAllow)
        );
        assert_eq!(
            snapshot.get_smart_approve_permission("unknown"),
            Some(PermissionLevel::NeverAllow)
        );
    }

    #[test]
    fn test_reads_fail_closed_when_permission_refresh_fails() {
        let (manager, _temp_dir) = create_test_permission_manager();
        manager
            .update_user_permission("git__status", PermissionLevel::AlwaysAllow)
            .unwrap();
        fs::write(manager.get_config_path(), "{{invalid yaml: [broken").unwrap();

        assert_eq!(
            manager.get_user_permission("git__status"),
            Some(PermissionLevel::NeverAllow)
        );
    }

    #[test]
    fn test_update_does_not_restore_externally_deleted_permissions() {
        let (manager, temp_dir) = create_test_permission_manager();
        manager
            .update_user_permission("git__status", PermissionLevel::AlwaysAllow)
            .unwrap();
        fs::remove_file(manager.get_config_path()).unwrap();

        manager
            .update_user_permission("github__status", PermissionLevel::AskBefore)
            .unwrap();
        drop(manager);

        let persisted_manager = PermissionManager::new(temp_dir.path().to_path_buf());
        assert_eq!(persisted_manager.get_user_permission("git__status"), None);
        assert_eq!(
            persisted_manager.get_user_permission("github__status"),
            Some(PermissionLevel::AskBefore)
        );
    }

    #[test]
    #[should_panic(expected = "Corrupted permission config")]
    fn test_corrupted_permission_file_panics() {
        let temp_dir = TempDir::new().unwrap();
        let permission_path = temp_dir.path().join(PERMISSION_FILE);
        fs::write(&permission_path, "{{invalid yaml: [broken").unwrap();
        PermissionManager::new(temp_dir.path().to_path_buf());
    }

    use test_case::test_case;

    #[test_case(
        vec![Tool::new("tool".to_string(), String::new(), object!({"type": "object"}))
            .annotate(ToolAnnotations::new().read_only(false))],
        Some(PermissionLevel::AskBefore);
        "write_annotation_caches_ask"
    )]
    #[test_case(
        vec![Tool::new("tool".to_string(), String::new(), object!({"type": "object"}))],
        None;
        "unannotated_left_uncached"
    )]
    #[test_case(
        vec![Tool::new("tool".to_string(), String::new(), object!({"type": "object"}))
            .annotate(ToolAnnotations::new().read_only(true))],
        None;
        "readonly_annotation_skipped"
    )]
    fn test_apply_tool_annotations(tools: Vec<Tool>, expect_cache: Option<PermissionLevel>) {
        let (manager, _temp_dir) = create_test_permission_manager();
        manager.apply_tool_annotations(&tools).unwrap();
        assert_eq!(manager.get_smart_approve_permission("tool"), expect_cache);
    }
}
