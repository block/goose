use std::collections::HashMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::Config;
use crate::plugins::plugin_install_dir;

const PLUGINS_CONFIG_KEY: &str = "plugins";

/// Per-plugin entry stored under the `plugins` map in `config.yaml`, keyed by
/// the plugin's filesystem path.
#[derive(Debug, Clone, Serialize, Deserialize)]
struct PluginConfigEntry {
    enabled: bool,
}

/// A plugin found on disk and not disabled by any settings file.
#[derive(Debug, Clone)]
pub struct DiscoveredPlugin {
    pub name: String,
    pub root: PathBuf,
    pub scope: PluginScope,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PluginScope {
    User,
    Project,
}

/// Settings file format from <https://open-plugins.com/plugin-builders/installation>.
#[derive(Debug, Default, Deserialize)]
struct PluginSettings {
    #[serde(default, rename = "enabledPlugins")]
    enabled: Vec<String>,
    #[serde(default, rename = "disabledPlugins")]
    disabled: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum SettingsScope {
    Local,
    Project,
    User,
}

/// Discover all plugins that should be considered active.
///
/// `project_root`, when supplied, enables project + local scope settings and
/// project-scope `.agents/plugins/` lookups.
pub fn discover_enabled_plugins(project_root: Option<&Path>) -> Vec<DiscoveredPlugin> {
    discover_enabled_plugins_with_config(project_root, Config::global())
}

fn discover_enabled_plugins_with_config(
    project_root: Option<&Path>,
    config: &Config,
) -> Vec<DiscoveredPlugin> {
    let scoped_settings = load_all_settings(project_root);
    let mut found: HashMap<String, DiscoveredPlugin> = HashMap::new();

    if let Some(root) = project_root {
        for (name, root) in list_dir_children(&project_plugin_dir(root)) {
            found.entry(name.clone()).or_insert(DiscoveredPlugin {
                name,
                root,
                scope: PluginScope::Project,
            });
        }
    }
    for (name, root) in list_dir_children(&plugin_install_dir()) {
        found.entry(name.clone()).or_insert(DiscoveredPlugin {
            name,
            root,
            scope: PluginScope::User,
        });
    }

    let enabled_by_settings: Vec<DiscoveredPlugin> = found
        .into_values()
        .filter(|plugin| is_enabled(&plugin.name, &scoped_settings))
        .collect();

    filter_by_config(enabled_by_settings, config)
}

/// Apply the `plugins` map in `config.yaml`. Newly discovered plugins are added
/// to the map with `enabled: true`; plugins explicitly set to `enabled: false`
/// are dropped.
fn filter_by_config(plugins: Vec<DiscoveredPlugin>, config: &Config) -> Vec<DiscoveredPlugin> {
    let mut entries: HashMap<String, PluginConfigEntry> =
        config.get_param(PLUGINS_CONFIG_KEY).unwrap_or_default();

    let mut dirty = false;
    let mut enabled = Vec::new();
    for plugin in plugins {
        let key = plugin.root.to_string_lossy().to_string();
        match entries.get(&key) {
            Some(entry) => {
                if entry.enabled {
                    enabled.push(plugin);
                }
            }
            None => {
                entries.insert(key, PluginConfigEntry { enabled: true });
                dirty = true;
                enabled.push(plugin);
            }
        }
    }

    if dirty {
        if let Err(e) = config.set_param(PLUGINS_CONFIG_KEY, entries) {
            tracing::warn!(error = %e, "Failed to persist plugin config entries");
        }
    }

    enabled
}

fn is_enabled(plugin_name: &str, scoped_settings: &[(SettingsScope, PluginSettings)]) -> bool {
    for scope in [
        SettingsScope::Local,
        SettingsScope::Project,
        SettingsScope::User,
    ] {
        let Some(settings) = scoped_settings
            .iter()
            .find_map(|(s, settings)| (*s == scope).then_some(settings))
        else {
            continue;
        };

        let listed_disabled = settings.disabled.iter().any(|n| n == plugin_name);
        let listed_enabled = settings.enabled.iter().any(|n| n == plugin_name);

        if listed_disabled {
            return false;
        }
        if listed_enabled {
            return true;
        }
    }

    true
}

fn project_plugin_dir(project_root: &Path) -> PathBuf {
    project_root.join(".agents").join("plugins")
}

fn list_dir_children(dir: &Path) -> Vec<(String, PathBuf)> {
    let entries = match std::fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };
    entries
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if !path.is_dir() {
                return None;
            }
            let name = path.file_name()?.to_str()?.to_string();
            Some((name, path))
        })
        .collect()
}

fn load_all_settings(project_root: Option<&Path>) -> Vec<(SettingsScope, PluginSettings)> {
    let mut paths: Vec<(SettingsScope, PathBuf)> = Vec::new();
    if let Some(path) = user_settings_path() {
        paths.push((SettingsScope::User, path));
    }
    if let Some(root) = project_root {
        paths.push((SettingsScope::Project, project_settings_path(root, false)));
        paths.push((SettingsScope::Local, project_settings_path(root, true)));
    }

    paths
        .into_iter()
        .filter_map(|(scope, path)| match read_settings(&path) {
            Ok(Some(s)) => Some((scope, s)),
            Ok(None) => None,
            Err(e) => {
                tracing::warn!(path = %path.display(), error = %e, "Failed to read plugin settings");
                None
            }
        })
        .collect()
}

fn user_settings_path() -> Option<PathBuf> {
    if let Ok(test_root) = std::env::var("GOOSE_PATH_ROOT") {
        return Some(
            PathBuf::from(test_root)
                .join(".config")
                .join("goose")
                .join("settings.json"),
        );
    }
    Some(
        dirs::home_dir()?
            .join(".config")
            .join("goose")
            .join("settings.json"),
    )
}

fn project_settings_path(project_root: &Path, local: bool) -> PathBuf {
    let file = if local {
        "settings.local.json"
    } else {
        "settings.json"
    };
    project_root.join(".config").join("goose").join(file)
}

fn read_settings(path: &Path) -> anyhow::Result<Option<PluginSettings>> {
    if !path.exists() {
        return Ok(None);
    }
    let text = std::fs::read_to_string(path)?;
    let parsed: PluginSettings = serde_json::from_str(&text)?;
    Ok(Some(parsed))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_plugin_dir(root: &Path, name: &str) {
        let dir = root.join(name);
        std::fs::create_dir_all(dir.join("hooks")).unwrap();
        std::fs::write(
            dir.join("hooks").join("hooks.json"),
            r#"{"hooks":{"SessionStart":[{"hooks":[]}]}}"#,
        )
        .unwrap();
    }

    fn write_settings(dir: &Path, contents: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("settings.json"), contents).unwrap();
    }

    fn write_local_settings(dir: &Path, contents: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(dir.join("settings.local.json"), contents).unwrap();
    }

    fn test_config(dir: &Path) -> Config {
        Config::new(dir.join("config.yaml"), "goose-discovery-test").unwrap()
    }

    fn discover(project: &Path) -> Vec<DiscoveredPlugin> {
        let cfg_dir = tempfile::tempdir().unwrap();
        discover_enabled_plugins_with_config(Some(project), &test_config(cfg_dir.path()))
    }

    #[test]
    fn finds_project_scope_plugin() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        write_plugin_dir(&project.join(".agents").join("plugins"), "demo");

        let found = discover(project);
        let names: Vec<_> = found.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"demo"), "got: {names:?}");
        let demo = found.iter().find(|p| p.name == "demo").unwrap();
        assert_eq!(demo.scope, PluginScope::Project);
    }

    #[test]
    fn disabled_in_project_settings_drops_plugin() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        write_plugin_dir(&project.join(".agents").join("plugins"), "demo");

        write_settings(
            &project.join(".config").join("goose"),
            r#"{"disabledPlugins":["demo"]}"#,
        );

        let found = discover(project);
        assert!(found.iter().all(|p| p.name != "demo"));
    }

    #[test]
    fn explicit_enabled_filters_out_unlisted_plugins() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        write_plugin_dir(&project.join(".agents").join("plugins"), "demo");
        write_plugin_dir(&project.join(".agents").join("plugins"), "other");

        write_settings(
            &project.join(".config").join("goose"),
            r#"{"enabledPlugins":["demo"]}"#,
        );

        let found = discover(project);
        let names: Vec<_> = found.iter().map(|p| p.name.as_str()).collect();
        assert!(names.contains(&"demo"), "got: {names:?}");
        assert!(names.contains(&"other"), "got: {names:?}");
    }

    #[test]
    fn local_scope_overrides_project_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        write_plugin_dir(&project.join(".agents").join("plugins"), "demo");

        write_settings(
            &project.join(".config").join("goose"),
            r#"{"disabledPlugins":["demo"]}"#,
        );
        write_local_settings(
            &project.join(".config").join("goose"),
            r#"{"enabledPlugins":["demo"]}"#,
        );

        let found = discover(project);
        assert!(
            found.iter().any(|p| p.name == "demo"),
            "local scope should win; got: {:?}",
            found.iter().map(|p| &p.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn project_scope_overrides_user_scope() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        write_plugin_dir(&project.join(".agents").join("plugins"), "demo");

        let fake_home = tempfile::tempdir().unwrap();
        write_settings(
            &fake_home.path().join(".config").join("goose"),
            r#"{"disabledPlugins":["demo"]}"#,
        );

        write_settings(
            &project.join(".config").join("goose"),
            r#"{"enabledPlugins":["demo"]}"#,
        );

        let prev = std::env::var("GOOSE_PATH_ROOT").ok();
        unsafe { std::env::set_var("GOOSE_PATH_ROOT", fake_home.path()) };
        let found = discover(project);
        match prev {
            Some(v) => unsafe { std::env::set_var("GOOSE_PATH_ROOT", v) },
            None => unsafe { std::env::remove_var("GOOSE_PATH_ROOT") },
        }

        assert!(
            found.iter().any(|p| p.name == "demo"),
            "project scope should win over user; got: {:?}",
            found.iter().map(|p| &p.name).collect::<Vec<_>>()
        );
    }

    #[test]
    fn newly_discovered_plugin_is_added_to_config_as_enabled() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        write_plugin_dir(&project.join(".agents").join("plugins"), "demo");

        let cfg_dir = tempfile::tempdir().unwrap();
        let config = test_config(cfg_dir.path());

        let found = discover_enabled_plugins_with_config(Some(project), &config);
        assert!(found.iter().any(|p| p.name == "demo"));

        let entries: HashMap<String, PluginConfigEntry> =
            config.get_param(PLUGINS_CONFIG_KEY).unwrap();
        let key = project
            .join(".agents")
            .join("plugins")
            .join("demo")
            .to_string_lossy()
            .to_string();
        assert!(
            entries.get(&key).is_some_and(|e| e.enabled),
            "got: {entries:?}"
        );
    }

    #[test]
    fn disabled_in_config_drops_plugin() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        write_plugin_dir(&project.join(".agents").join("plugins"), "demo");

        let cfg_dir = tempfile::tempdir().unwrap();
        let config = test_config(cfg_dir.path());
        let key = project
            .join(".agents")
            .join("plugins")
            .join("demo")
            .to_string_lossy()
            .to_string();
        let entries = HashMap::from([(key, PluginConfigEntry { enabled: false })]);
        config.set_param(PLUGINS_CONFIG_KEY, entries).unwrap();

        let found = discover_enabled_plugins_with_config(Some(project), &config);
        assert!(found.iter().all(|p| p.name != "demo"));
    }

    #[test]
    fn enabled_in_config_keeps_plugin_without_modifying_config() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path();
        write_plugin_dir(&project.join(".agents").join("plugins"), "demo");

        let cfg_dir = tempfile::tempdir().unwrap();
        let config = test_config(cfg_dir.path());
        let key = project
            .join(".agents")
            .join("plugins")
            .join("demo")
            .to_string_lossy()
            .to_string();
        config
            .set_param(
                PLUGINS_CONFIG_KEY,
                HashMap::from([(key.clone(), PluginConfigEntry { enabled: true })]),
            )
            .unwrap();

        let found = discover_enabled_plugins_with_config(Some(project), &config);
        assert!(found.iter().any(|p| p.name == "demo"));

        let entries: HashMap<String, PluginConfigEntry> =
            config.get_param(PLUGINS_CONFIG_KEY).unwrap();
        assert!(entries.get(&key).is_some_and(|e| e.enabled));
    }
}
