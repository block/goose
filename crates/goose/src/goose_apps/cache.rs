use crate::config::paths::Paths;
use crate::utils::bytes_to_hex;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tracing::warn;

use super::app::GooseApp;

static CLOCK_HTML: &str = include_str!("../goose_apps/clock.html");
const APPS_EXTENSION_NAME: &str = "apps";

pub const BUNDLED_DEFAULT_APP_URIS: &[&str] = &["ui://apps/clock"];

/// Bundled default apps: (cache URI, HTML source).
const DEFAULT_APPS: &[(&str, &str)] = &[("ui://apps/clock", CLOCK_HTML)];

pub fn mark_deletable_apps(apps: &mut [GooseApp]) {
    for app in apps.iter_mut() {
        let is_apps_extension = app
            .mcp_servers
            .iter()
            .any(|server| server == APPS_EXTENSION_NAME);
        app.deletable =
            is_apps_extension && !McpAppCache::is_bundled_default_uri(&app.resource.uri);
    }
}

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
        for (uri, html) in DEFAULT_APPS {
            if self.get_app(APPS_EXTENSION_NAME, uri).is_none() {
                if let Ok(mut app) = GooseApp::from_html(html) {
                    app.mcp_servers = vec![APPS_EXTENSION_NAME.to_string()];
                    let _ = self.store_app(&app);
                }
            }
        }
    }

    pub fn is_bundled_default_uri(uri: &str) -> bool {
        BUNDLED_DEFAULT_APP_URIS.contains(&uri)
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

    pub fn delete_app(
        &self,
        extension_name: &str,
        resource_uri: &str,
    ) -> Result<(), std::io::Error> {
        let cache_key = Self::cache_key(extension_name, resource_uri);
        let app_path = self.cache_dir.join(format!("{}.json", cache_key));

        if !app_path.exists() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotFound,
                format!(
                    "App not found in cache: {}::{}",
                    extension_name, resource_uri
                ),
            ));
        }

        fs::remove_file(app_path)
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
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    const CUSTOM_APP_HTML: &str = r#"<!DOCTYPE html>
<html>
<head>
  <script type="application/ld+json">
    {
      "@context": "https://goose.ai/schema",
      "@type": "GooseApp",
      "name": "test-app",
      "description": "Test app",
      "width": 100,
      "height": 100,
      "resizable": false
    }
  </script>
</head>
<body></body>
</html>"#;

    fn with_temp_config<F>(test: F)
    where
        F: FnOnce(),
    {
        let root = TempDir::new().unwrap();
        std::env::set_var("GOOSE_PATH_ROOT", root.path());
        test();
        std::env::remove_var("GOOSE_PATH_ROOT");
    }

    #[test]
    fn is_bundled_default_uri_matches_clock() {
        assert!(McpAppCache::is_bundled_default_uri("ui://apps/clock"));
        assert!(!McpAppCache::is_bundled_default_uri("ui://apps/chat"));
    }

    #[test]
    fn mark_deletable_apps_protects_bundled_uri_not_name() {
        let mut bundled_clock = GooseApp::from_html(CLOCK_HTML).unwrap();
        bundled_clock.mcp_servers = vec![APPS_EXTENSION_NAME.to_string()];

        let mut user_clock = GooseApp::from_html(CUSTOM_APP_HTML).unwrap();
        user_clock.resource.name = "clock".to_string();
        user_clock.resource.uri = "ui://apps/user-clock".to_string();
        user_clock.mcp_servers = vec![APPS_EXTENSION_NAME.to_string()];

        let mut custom = GooseApp::from_html(CUSTOM_APP_HTML).unwrap();
        custom.mcp_servers = vec![APPS_EXTENSION_NAME.to_string()];

        let mut external = GooseApp::from_html(CUSTOM_APP_HTML).unwrap();
        external.mcp_servers = vec!["other-extension".to_string()];

        let mut apps = vec![bundled_clock, user_clock, custom, external];
        mark_deletable_apps(&mut apps);

        assert!(!apps[0].deletable);
        assert!(apps[1].deletable);
        assert!(apps[2].deletable);
        assert!(!apps[3].deletable);
    }

    #[test]
    #[serial]
    fn delete_app_removes_cached_entry() {
        with_temp_config(|| {
            let cache = McpAppCache::new().unwrap();
            let mut app = GooseApp::from_html(CUSTOM_APP_HTML).unwrap();
            app.mcp_servers = vec![APPS_EXTENSION_NAME.to_string()];
            let uri = app.resource.uri.clone();

            cache.store_app(&app).unwrap();
            assert!(cache.get_app(APPS_EXTENSION_NAME, &uri).is_some());

            cache
                .delete_app(APPS_EXTENSION_NAME, &uri)
                .expect("delete should succeed");
            assert!(cache.get_app(APPS_EXTENSION_NAME, &uri).is_none());
        });
    }

    #[test]
    #[serial]
    fn delete_app_returns_not_found_for_missing_entry() {
        with_temp_config(|| {
            let cache = McpAppCache::new().unwrap();
            let error = cache
                .delete_app(APPS_EXTENSION_NAME, "apps://missing")
                .unwrap_err();
            assert_eq!(error.kind(), std::io::ErrorKind::NotFound);
        });
    }
}
