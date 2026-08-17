use crate::config::paths::Paths;
use crate::utils::bytes_to_hex;
use sha2::{Digest, Sha256};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};
use tracing::warn;

use super::app::GooseApp;

static CLOCK_HTML: &str = include_str!("../goose_apps/clock.html");
const APPS_EXTENSION_NAME: &str = "apps";

const MAX_APP_CONTENT_BYTES: usize = 5 * 1024 * 1024;
const MAX_APP_INPUT_BYTES: usize = 6 * 1024 * 1024;
const MAX_SERIALIZED_APP_BYTES: u64 = 8 * 1024 * 1024;
const MAX_CACHE_BYTES: u64 = 25 * 1024 * 1024;
const MAX_CACHE_ENTRIES: usize = 128;

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
        cache.ensure_default_apps()?;
        Ok(cache)
    }

    fn ensure_default_apps(&self) -> Result<(), std::io::Error> {
        fs::create_dir_all(&self.cache_dir)?;

        for (uri, _) in DEFAULT_APPS {
            let app = Self::bundled_default_app(uri)
                .ok_or_else(|| std::io::Error::other("Invalid bundled MCP app"))?;
            let app_path = self.app_path(APPS_EXTENSION_NAME, uri);
            Self::write_app_file(&app_path, &app)?;
        }

        Ok(())
    }

    pub fn is_bundled_default_uri(uri: &str) -> bool {
        BUNDLED_DEFAULT_APP_URIS.contains(&uri)
    }

    fn cache_key(extension_name: &str, resource_uri: &str) -> String {
        let input = format!("{}::{}", extension_name, resource_uri);
        let hash = bytes_to_hex(Sha256::digest(input.as_bytes()));
        format!("{}_{}", extension_name, hash)
    }

    fn app_path(&self, extension_name: &str, resource_uri: &str) -> PathBuf {
        self.cache_dir.join(format!(
            "{}.json",
            Self::cache_key(extension_name, resource_uri)
        ))
    }

    fn is_bundled_default_identity(extension_name: &str, resource_uri: &str) -> bool {
        extension_name == APPS_EXTENSION_NAME && Self::is_bundled_default_uri(resource_uri)
    }

    fn bundled_default_app(resource_uri: &str) -> Option<GooseApp> {
        let (_, html) = DEFAULT_APPS.iter().find(|(uri, _)| *uri == resource_uri)?;
        let mut app = GooseApp::from_html(html).ok()?;
        app.mcp_servers = vec![APPS_EXTENSION_NAME.to_string()];
        Some(app)
    }

    pub fn restore_bundled_default_apps(apps: &mut [GooseApp]) {
        for app in apps {
            let is_bundled_identity = app.mcp_servers.iter().any(|extension_name| {
                Self::is_bundled_default_identity(extension_name, &app.resource.uri)
            });
            if is_bundled_identity {
                if let Some(default_app) = Self::bundled_default_app(&app.resource.uri) {
                    *app = default_app;
                }
            }
        }
    }

    fn write_app_file(path: &Path, app: &GooseApp) -> Result<(), std::io::Error> {
        let json = serde_json::to_vec_pretty(app).map_err(std::io::Error::other)?;
        fs::write(path, json)
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
                match Self::read_cache_file(&path) {
                    Some(app) => apps.push(app),
                    None => warn!("Failed to read valid cached app from {:?}", path),
                }
            }
        }

        Ok(apps)
    }

    pub fn store_app(&self, app: &GooseApp) -> Result<(), std::io::Error> {
        self.ensure_default_apps()?;
        self.write_generation(std::slice::from_ref(app), None)
    }

    pub fn refresh_apps(&self, apps: &[GooseApp]) -> Result<(), std::io::Error> {
        self.ensure_default_apps()?;

        let refreshed_extensions = apps
            .iter()
            .flat_map(|app| app.mcp_servers.iter().cloned())
            .collect::<HashSet<_>>();
        self.write_generation(apps, Some(&refreshed_extensions))
    }

    pub fn get_app(&self, extension_name: &str, resource_uri: &str) -> Option<GooseApp> {
        let app_path = self.app_path(extension_name, resource_uri);

        if !app_path.exists() {
            return None;
        }

        Self::read_cache_file(&app_path)
    }

    pub fn delete_app(
        &self,
        extension_name: &str,
        resource_uri: &str,
    ) -> Result<(), std::io::Error> {
        if Self::is_bundled_default_identity(extension_name, resource_uri) {
            self.ensure_default_apps()?;
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Cannot delete bundled default app",
            ));
        }

        let app_path = self.app_path(extension_name, resource_uri);

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
                if let Some(app) = Self::read_cache_file(&path) {
                    if app.mcp_servers.contains(&extension_name.to_string())
                        && !Self::is_bundled_default_identity(extension_name, &app.resource.uri)
                        && fs::remove_file(&path).is_ok()
                    {
                        deleted_count += 1;
                    }
                }
            }
        }

        Ok(deleted_count)
    }

    fn write_generation(
        &self,
        apps: &[GooseApp],
        refreshed_extensions: Option<&HashSet<String>>,
    ) -> Result<(), std::io::Error> {
        Self::validate_apps(apps)?;

        let mut entries = HashMap::new();
        for app in apps {
            for extension_name in &app.mcp_servers {
                if !Self::is_bundled_default_identity(extension_name, &app.resource.uri) {
                    entries.insert(self.app_path(extension_name, &app.resource.uri), app);
                }
            }
        }

        if entries.len() > MAX_CACHE_ENTRIES {
            return Err(Self::limit_error("MCP app cache entry limit exceeded"));
        }

        fs::create_dir_all(&self.cache_dir)?;
        let staging_dir = tempfile::tempdir_in(&self.cache_dir)?;
        let mut staged_entries = Vec::with_capacity(entries.len());
        let mut staged_bytes = 0u64;

        for (target_path, app) in entries {
            let json = serde_json::to_vec_pretty(app).map_err(std::io::Error::other)?;
            let serialized_bytes = u64::try_from(json.len())
                .map_err(|_| Self::limit_error("MCP app record is too large"))?;
            if serialized_bytes > MAX_SERIALIZED_APP_BYTES {
                return Err(Self::limit_error("MCP app record is too large"));
            }
            staged_bytes = staged_bytes
                .checked_add(serialized_bytes)
                .ok_or_else(|| Self::limit_error("MCP app cache size limit exceeded"))?;
            if staged_bytes > MAX_CACHE_BYTES {
                return Err(Self::limit_error("MCP app cache size limit exceeded"));
            }

            let file_name = target_path
                .file_name()
                .ok_or_else(|| std::io::Error::other("Invalid MCP app cache path"))?;
            let staged_path = staging_dir.path().join(file_name);
            fs::write(&staged_path, json)?;
            staged_entries.push((staged_path, target_path));
        }

        let replaced_paths = staged_entries
            .iter()
            .map(|(_, target)| target.clone())
            .collect::<HashSet<_>>();
        let (retained_entries, retained_bytes) =
            self.retained_cache_usage(refreshed_extensions, &replaced_paths)?;
        let total_entries = retained_entries
            .checked_add(staged_entries.len())
            .ok_or_else(|| Self::limit_error("MCP app cache entry limit exceeded"))?;
        let total_bytes = retained_bytes
            .checked_add(staged_bytes)
            .ok_or_else(|| Self::limit_error("MCP app cache size limit exceeded"))?;

        if total_entries > MAX_CACHE_ENTRIES {
            return Err(Self::limit_error("MCP app cache entry limit exceeded"));
        }
        if total_bytes > MAX_CACHE_BYTES {
            return Err(Self::limit_error("MCP app cache size limit exceeded"));
        }

        if let Some(extension_names) = refreshed_extensions {
            for extension_name in extension_names {
                self.delete_extension_apps(extension_name)?;
            }
        }

        for (staged_path, target_path) in staged_entries {
            if target_path.exists() {
                fs::remove_file(&target_path)?;
            }
            fs::rename(staged_path, target_path)?;
        }

        self.ensure_default_apps()
    }

    pub fn validate_apps(apps: &[GooseApp]) -> Result<(), std::io::Error> {
        if apps.len() > MAX_CACHE_ENTRIES {
            return Err(Self::limit_error("MCP app refresh entry limit exceeded"));
        }

        let mut refresh_bytes = 0usize;
        for app in apps {
            let content_bytes = app
                .resource
                .text
                .as_ref()
                .map_or(0, String::len)
                .checked_add(app.resource.blob.as_ref().map_or(0, String::len))
                .ok_or_else(|| Self::limit_error("MCP app content is too large"))?;
            if content_bytes > MAX_APP_CONTENT_BYTES {
                return Err(Self::limit_error("MCP app content is too large"));
            }

            let input_bytes = Self::app_input_bytes(app)
                .ok_or_else(|| Self::limit_error("MCP app input is too large"))?;
            if input_bytes > MAX_APP_INPUT_BYTES {
                return Err(Self::limit_error("MCP app input is too large"));
            }
            refresh_bytes = refresh_bytes
                .checked_add(input_bytes)
                .ok_or_else(|| Self::limit_error("MCP app refresh size limit exceeded"))?;
            if refresh_bytes > MAX_CACHE_BYTES as usize {
                return Err(Self::limit_error("MCP app refresh size limit exceeded"));
            }
        }

        Ok(())
    }

    fn app_input_bytes(app: &GooseApp) -> Option<usize> {
        let mut total = 0usize;
        for value in [
            Some(&app.resource.uri),
            Some(&app.resource.name),
            app.resource.description.as_ref(),
            Some(&app.resource.mime_type),
            app.resource.text.as_ref(),
            app.resource.blob.as_ref(),
            app.prd.as_ref(),
        ]
        .into_iter()
        .flatten()
        {
            total = total.checked_add(value.len())?;
        }
        for server in &app.mcp_servers {
            total = total.checked_add(server.len())?;
        }

        if let Some(ui) = app.resource.meta.as_ref().and_then(|meta| meta.ui.as_ref()) {
            if let Some(domain) = &ui.domain {
                total = total.checked_add(domain.len())?;
            }
            if let Some(csp) = &ui.csp {
                for domains in [
                    csp.connect_domains.as_ref(),
                    csp.resource_domains.as_ref(),
                    csp.frame_domains.as_ref(),
                    csp.base_uri_domains.as_ref(),
                ]
                .into_iter()
                .flatten()
                {
                    for domain in domains {
                        total = total.checked_add(domain.len())?;
                    }
                }
            }
        }

        Some(total)
    }

    fn retained_cache_usage(
        &self,
        refreshed_extensions: Option<&HashSet<String>>,
        replaced_paths: &HashSet<PathBuf>,
    ) -> Result<(usize, u64), std::io::Error> {
        let mut entry_count = 0usize;
        let mut byte_count = 0u64;

        for entry in fs::read_dir(&self.cache_dir)? {
            let path = entry?.path();
            if path.extension().and_then(|extension| extension.to_str()) != Some("json")
                || replaced_paths.contains(&path)
            {
                continue;
            }

            if let Some(extension_names) = refreshed_extensions {
                if let Some(app) = Self::read_cache_file(&path) {
                    let replaced = extension_names.iter().any(|extension_name| {
                        app.mcp_servers.contains(extension_name)
                            && !Self::is_bundled_default_identity(extension_name, &app.resource.uri)
                    });
                    if replaced {
                        continue;
                    }
                }
            }

            entry_count = entry_count
                .checked_add(1)
                .ok_or_else(|| Self::limit_error("MCP app cache entry limit exceeded"))?;
            byte_count = byte_count
                .checked_add(fs::metadata(path)?.len())
                .ok_or_else(|| Self::limit_error("MCP app cache size limit exceeded"))?;
        }

        Ok((entry_count, byte_count))
    }

    fn read_cache_file(path: &Path) -> Option<GooseApp> {
        let metadata = fs::metadata(path).ok()?;
        if metadata.len() > MAX_SERIALIZED_APP_BYTES {
            return None;
        }
        let content = fs::read_to_string(path).ok()?;
        serde_json::from_str(&content).ok()
    }

    fn limit_error(message: &'static str) -> std::io::Error {
        std::io::Error::new(std::io::ErrorKind::InvalidData, message)
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

    fn remote_app(name: &str, extension_name: &str) -> GooseApp {
        let mut app = GooseApp::from_html(CUSTOM_APP_HTML).unwrap();
        app.resource.name = name.to_string();
        app.resource.uri = format!("ui://remote/{name}");
        app.mcp_servers = vec![extension_name.to_string()];
        app
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

    #[test]
    #[serial]
    fn bundled_default_identity_ignores_overwrite_and_survives_refresh_cleanup() {
        with_temp_config(|| {
            let cache = McpAppCache::new().unwrap();
            let expected = GooseApp::from_html(CLOCK_HTML).unwrap();
            let mut attacker_app = remote_app("clock", APPS_EXTENSION_NAME);
            attacker_app.resource.uri = "ui://apps/clock".to_string();
            attacker_app.resource.text = Some("<script>malicious()</script>".to_string());

            let mut exposed_apps = vec![attacker_app.clone()];
            McpAppCache::restore_bundled_default_apps(&mut exposed_apps);
            assert_eq!(exposed_apps[0].resource.text, expected.resource.text);

            cache.store_app(&attacker_app).unwrap();
            cache
                .refresh_apps(std::slice::from_ref(&attacker_app))
                .unwrap();
            cache.delete_extension_apps(APPS_EXTENSION_NAME).unwrap();

            let reopened = McpAppCache::new().unwrap();
            let cached = reopened
                .get_app(APPS_EXTENSION_NAME, "ui://apps/clock")
                .unwrap();
            assert_eq!(cached.resource.text, expected.resource.text);
            assert_eq!(cached.resource.name, expected.resource.name);
            assert_eq!(cached.mcp_servers, vec![APPS_EXTENSION_NAME.to_string()]);
        });
    }

    #[test]
    #[serial]
    fn oversized_single_app_is_rejected_before_persistence() {
        with_temp_config(|| {
            let cache = McpAppCache::new().unwrap();
            let mut app = remote_app("oversized", "remote-extension");
            app.resource.text = Some("A".repeat(MAX_APP_CONTENT_BYTES + 1));

            let error = cache.store_app(&app).unwrap_err();

            assert_eq!(error.kind(), std::io::ErrorKind::InvalidData);
            assert!(cache
                .get_app("remote-extension", "ui://remote/oversized")
                .is_none());
        });
    }

    #[test]
    #[serial]
    fn refresh_rejects_aggregate_bytes_and_entry_count_limits() {
        with_temp_config(|| {
            let cache = McpAppCache::new().unwrap();
            let mut large_apps = Vec::new();
            for index in 0..6 {
                let mut app = remote_app(&format!("large-{index}"), "remote-extension");
                app.resource.text = Some("A".repeat(MAX_APP_CONTENT_BYTES));
                large_apps.push(app);
            }
            assert_eq!(
                cache.refresh_apps(&large_apps).unwrap_err().kind(),
                std::io::ErrorKind::InvalidData
            );

            let many_apps = (0..=MAX_CACHE_ENTRIES)
                .map(|index| remote_app(&format!("app-{index}"), "remote-extension"))
                .collect::<Vec<_>>();
            assert_eq!(
                cache.refresh_apps(&many_apps).unwrap_err().kind(),
                std::io::ErrorKind::InvalidData
            );
            assert_eq!(cache.list_apps().unwrap().len(), DEFAULT_APPS.len());
        });
    }

    #[test]
    #[serial]
    fn store_app_enforces_the_existing_cache_byte_quota() {
        with_temp_config(|| {
            let cache = McpAppCache::new().unwrap();
            let existing_apps = (0..4)
                .map(|index| {
                    let mut app = remote_app(&format!("existing-{index}"), "remote-extension");
                    app.resource.text = Some("A".repeat(MAX_APP_CONTENT_BYTES));
                    app
                })
                .collect::<Vec<_>>();
            cache.refresh_apps(&existing_apps).unwrap();

            let mut rejected = remote_app("over-quota", "other-extension");
            rejected.resource.text = Some("A".repeat(MAX_APP_CONTENT_BYTES));
            assert_eq!(
                cache.store_app(&rejected).unwrap_err().kind(),
                std::io::ErrorKind::InvalidData
            );
            assert!(cache
                .get_app("other-extension", &rejected.resource.uri)
                .is_none());
            for app in existing_apps {
                assert!(cache
                    .get_app("remote-extension", &app.resource.uri)
                    .is_some());
            }
        });
    }

    #[test]
    #[serial]
    fn legitimate_refresh_replaces_only_the_extensions_generation() {
        with_temp_config(|| {
            let cache = McpAppCache::new().unwrap();
            let first = remote_app("first", "remote-extension");
            let second = remote_app("second", "remote-extension");
            let independent = remote_app("independent", "other-extension");

            cache.refresh_apps(&[first.clone(), second]).unwrap();
            cache.store_app(&independent).unwrap();
            cache.refresh_apps(std::slice::from_ref(&first)).unwrap();

            assert!(cache
                .get_app("remote-extension", &first.resource.uri)
                .is_some());
            assert!(cache
                .get_app("remote-extension", "ui://remote/second")
                .is_none());
            assert!(cache
                .get_app("other-extension", &independent.resource.uri)
                .is_some());
            assert!(cache
                .get_app(APPS_EXTENSION_NAME, "ui://apps/clock")
                .is_some());
        });
    }

    #[test]
    #[serial]
    fn rejected_refresh_preserves_previous_generation_without_staged_files() {
        with_temp_config(|| {
            let cache = McpAppCache::new().unwrap();
            let previous = remote_app("previous", "remote-extension");
            cache.refresh_apps(std::slice::from_ref(&previous)).unwrap();

            let replacement = remote_app("replacement", "remote-extension");
            let mut oversized = remote_app("oversized", "remote-extension");
            oversized.resource.text = Some("A".repeat(MAX_APP_CONTENT_BYTES + 1));
            assert!(cache.refresh_apps(&[replacement, oversized]).is_err());

            assert!(cache
                .get_app("remote-extension", &previous.resource.uri)
                .is_some());
            assert!(cache
                .get_app("remote-extension", "ui://remote/replacement")
                .is_none());
            assert!(fs::read_dir(&cache.cache_dir).unwrap().all(|entry| {
                entry
                    .unwrap()
                    .path()
                    .extension()
                    .and_then(|value| value.to_str())
                    == Some("json")
            }));
        });
    }
}
