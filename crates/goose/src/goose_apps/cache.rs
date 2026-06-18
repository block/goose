use crate::config::paths::Paths;
use crate::utils::bytes_to_hex;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tracing::warn;

use super::app::GooseApp;
use super::default_apps::{
    parse_default_apps, LEGACY_DEFAULT_APP_NAMES, RETIRED_CURATED_APP_NAMES,
};
const APPS_EXTENSION_NAME: &str = "apps";

pub struct McpAppCache {
    cache_dir: PathBuf,
}

impl McpAppCache {
    pub fn new() -> Result<Self, std::io::Error> {
        let config_dir = Paths::config_dir();
        let cache_dir = config_dir.join("mcp-apps-cache");
        let cache = Self { cache_dir };
        cache.ensure_default_apps();
        Ok(cache)
    }

    fn ensure_default_apps(&self) {
        if let Err(error) = self.sync_default_apps() {
            warn!("Failed to seed default apps cache: {}", error);
        }
    }

    fn sync_default_apps(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.cache_dir)?;

        for app in self.list_apps()? {
            if !app
                .mcp_servers
                .iter()
                .any(|server| server == APPS_EXTENSION_NAME)
            {
                continue;
            }

            if LEGACY_DEFAULT_APP_NAMES.contains(&app.resource.name.as_str())
                || RETIRED_CURATED_APP_NAMES.contains(&app.resource.name.as_str())
            {
                let cache_key = Self::cache_key(APPS_EXTENSION_NAME, &app.resource.uri);
                let app_path = self.cache_dir.join(format!("{}.json", cache_key));
                if app_path.exists() {
                    fs::remove_file(app_path)?;
                }
            }
        }

        for mut app in parse_default_apps().map_err(std::io::Error::other)? {
            app.mcp_servers = vec![APPS_EXTENSION_NAME.to_string()];
            self.store_app(&app)?;
        }

        Ok(())
    }

    fn cache_key(extension_name: &str, resource_uri: &str) -> String {
        let input = format!("{}::{}", extension_name, resource_uri);
        let hash = bytes_to_hex(Sha256::digest(input.as_bytes()));
        format!("{}_{}", extension_name, hash)
    }

    pub fn list_apps(&self) -> Result<Vec<GooseApp>, std::io::Error> {
        let mut apps = Vec::new();

        if !self.cache_dir.exists() {
            return Ok(apps);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                match fs::read_to_string(&path) {
                    Ok(content) => match serde_json::from_str::<GooseApp>(&content) {
                        Ok(app) => apps.push(app),
                        Err(e) => warn!("Failed to parse cached app from {:?}: {}", path, e),
                    },
                    Err(e) => warn!("Failed to read cached app from {:?}: {}", path, e),
                }
            }
        }

        Ok(apps)
    }

    pub fn store_app(&self, app: &GooseApp) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.cache_dir)?;

        // Store the app once for each MCP server it's associated with
        for extension_name in &app.mcp_servers {
            let cache_key = Self::cache_key(extension_name, &app.resource.uri);
            let app_path = self.cache_dir.join(format!("{}.json", cache_key));
            let json = serde_json::to_string_pretty(app).map_err(std::io::Error::other)?;
            fs::write(app_path, json)?;
        }

        Ok(())
    }

    pub fn get_app(&self, extension_name: &str, resource_uri: &str) -> Option<GooseApp> {
        let cache_key = Self::cache_key(extension_name, resource_uri);
        let app_path = self.cache_dir.join(format!("{}.json", cache_key));

        if !app_path.exists() {
            return None;
        }

        fs::read_to_string(&app_path)
            .ok()
            .and_then(|content| serde_json::from_str::<GooseApp>(&content).ok())
    }

    pub fn delete_extension_apps(&self, extension_name: &str) -> Result<usize, std::io::Error> {
        let mut deleted_count = 0;

        if !self.cache_dir.exists() {
            return Ok(0);
        }

        for entry in fs::read_dir(&self.cache_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(content) = fs::read_to_string(&path) {
                    if let Ok(app) = serde_json::from_str::<GooseApp>(&content) {
                        if app.mcp_servers.contains(&extension_name.to_string())
                            && fs::remove_file(&path).is_ok()
                        {
                            deleted_count += 1;
                        }
                    }
                }
            }
        }

        Ok(deleted_count)
    }
}

#[cfg(test)]
mod tests {
    use super::McpAppCache;
    use env_lock::lock_env;
    use std::path::PathBuf;
    use tempfile::TempDir;

    fn cache_file_path(root: &str, extension_name: &str, resource_uri: &str) -> PathBuf {
        let cache_key = McpAppCache::cache_key(extension_name, resource_uri);
        PathBuf::from(root)
            .join("config")
            .join("mcp-apps-cache")
            .join(format!("{}.json", cache_key))
    }

    #[test]
    fn seeds_curated_security_default_apps_instead_of_legacy_defaults() {
        let temp_root = TempDir::new().unwrap();
        let temp_root = temp_root.path().to_string_lossy().to_string();
        let _guard = lock_env([("GOOSE_PATH_ROOT", Some(temp_root.as_str()))]);

        let cache = McpAppCache::new().unwrap();
        let mut app_names = cache
            .list_apps()
            .unwrap()
            .into_iter()
            .map(|app| app.resource.name)
            .collect::<Vec<_>>();
        app_names.sort();

        assert_eq!(
            app_names,
            vec![
                "encode-hash-lab".to_string(),
                "ioc-toolbox".to_string(),
                "jwt-inspector".to_string(),
                "secret-credential-scanner".to_string(),
            ]
        );
        assert!(!app_names
            .iter()
            .any(|name| name == "chat" || name == "clock"));
    }

    #[test]
    fn refreshes_curated_security_default_apps_when_cached_copy_drifted() {
        let temp_root = TempDir::new().unwrap();
        let temp_root = temp_root.path().to_string_lossy().to_string();
        let _guard = lock_env([("GOOSE_PATH_ROOT", Some(temp_root.as_str()))]);

        let cache = McpAppCache::new().unwrap();
        let mut stale_ioc = cache
            .list_apps()
            .unwrap()
            .into_iter()
            .find(|app| app.resource.name == "ioc-toolbox")
            .unwrap();
        stale_ioc.resource.description = Some("stale description".to_string());
        cache.store_app(&stale_ioc).unwrap();

        let refreshed_cache = McpAppCache::new().unwrap();
        let refreshed_ioc = refreshed_cache
            .list_apps()
            .unwrap()
            .into_iter()
            .find(|app| app.resource.name == "ioc-toolbox")
            .unwrap();

        assert_ne!(
            refreshed_ioc.resource.description.as_deref(),
            Some("stale description")
        );
        assert_eq!(
            refreshed_ioc.resource.description.as_deref(),
            Some("离线提取、归类、规范化和去重混合 IOC 指标。")
        );
    }

    #[test]
    fn removes_retired_curated_security_apps_from_cache() {
        let temp_root = TempDir::new().unwrap();
        let temp_root = temp_root.path().to_string_lossy().to_string();
        let _guard = lock_env([("GOOSE_PATH_ROOT", Some(temp_root.as_str()))]);

        let _cache = McpAppCache::new().unwrap();
        let retired_cache_file = cache_file_path(&temp_root, "apps", "ui://apps/header-diff-lab");
        std::fs::write(
            &retired_cache_file,
            r#"{
  "name": "header-diff-lab",
  "uri": "ui://apps/header-diff-lab",
  "description": "retired",
  "mcpServers": ["apps"],
  "mimeType": "text/html;profile=mcp-app",
  "text": "<html></html>"
}"#,
        )
        .unwrap();

        let refreshed_cache = McpAppCache::new().unwrap();
        let names = refreshed_cache
            .list_apps()
            .unwrap()
            .into_iter()
            .map(|app| app.resource.name)
            .collect::<Vec<_>>();

        assert!(!retired_cache_file.exists());
        assert!(!names.iter().any(|name| name == "header-diff-lab"));
    }
}
