use super::base::{Config, ConfigError};
use crate::agents::extension::PLATFORM_EXTENSIONS;
use crate::agents::ExtensionConfig;
use indexmap::IndexMap;
use serde::{Deserialize, Serialize};
use serde_yaml::Mapping;
use thiserror::Error;
use tracing::{info, warn};

pub const DEFAULT_EXTENSION: &str = "developer";
pub const DEFAULT_EXTENSION_TIMEOUT: u64 = 300;
pub const DEFAULT_EXTENSION_DESCRIPTION: &str = "";
pub const DEFAULT_DISPLAY_NAME: &str = "Developer";
const EXTENSIONS_CONFIG_KEY: &str = "extensions";
static EXTENSION_MUTATION_GUARD: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct ExtensionEntry {
    pub enabled: bool,
    #[serde(flatten)]
    pub config: ExtensionConfig,
}

#[derive(Debug, Error)]
pub enum ExtensionAddError {
    #[error("An extension with key '{key}' already exists")]
    AlreadyExists { key: String },
    #[error(transparent)]
    Config(#[from] ConfigError),
}

#[derive(Debug, Error)]
pub enum ExtensionUpdateError {
    #[error("Extension with key '{key}' was not found")]
    NotFound { key: String },
    #[error("Extension update cannot change identity from '{key}' to '{new_key}'")]
    IdentityChanged { key: String, new_key: String },
    #[error(transparent)]
    Config(#[from] ConfigError),
}

pub fn name_to_key(name: &str) -> String {
    let mut result = String::with_capacity(name.len());
    for c in name.chars() {
        result.push(match c {
            c if c.is_ascii_alphanumeric() || c == '_' || c == '-' => c,
            c if c.is_whitespace() => continue,
            _ => '_',
        });
    }
    result.to_lowercase()
}

pub(crate) fn is_extension_available(config: &ExtensionConfig) -> bool {
    match config {
        ExtensionConfig::Platform { name, .. } => {
            crate::agents::extension::PLATFORM_EXTENSIONS.contains_key(name_to_key(name).as_str())
        }
        _ => true,
    }
}

fn inject_name_if_missing(key: &str, value: serde_yaml::Value) -> serde_yaml::Value {
    let name_key = serde_yaml::Value::String("name".to_string());
    if let serde_yaml::Value::Mapping(mut map) = value {
        if !map.contains_key(&name_key) {
            map.insert(name_key, serde_yaml::Value::String(key.to_string()));
        }
        serde_yaml::Value::Mapping(map)
    } else {
        value
    }
}

fn parse_extensions_map(raw: &Mapping) -> IndexMap<String, ExtensionEntry> {
    let mut extensions_map = IndexMap::with_capacity(raw.len());
    for (k, v) in raw {
        let Some(key) = k.as_str() else {
            warn!(key = ?k, "Skipping malformed extension config entry");
            continue;
        };

        let v = inject_name_if_missing(key, v.clone());
        match serde_yaml::from_value::<ExtensionEntry>(v) {
            Ok(entry) => {
                if !is_extension_available(&entry.config) {
                    continue;
                }
                extensions_map.insert(key.to_string(), entry);
            }
            Err(err) => {
                info!(
                    key = %key,
                    error = %err,
                    "Skipping malformed extension config entry"
                );
            }
        }
    }

    extensions_map
}

fn get_extensions_map_with_config(config: &Config) -> IndexMap<String, ExtensionEntry> {
    let raw: Mapping = config
        .get_param(EXTENSIONS_CONFIG_KEY)
        .unwrap_or_else(|err| {
            warn!(
                "Failed to load {}: {err}. Falling back to empty object.",
                EXTENSIONS_CONFIG_KEY
            );
            Default::default()
        });

    parse_extensions_map(&raw)
}

fn get_extensions_map() -> IndexMap<String, ExtensionEntry> {
    get_extensions_map_with_config(Config::global())
}

fn extension_identity_exists(raw: &Mapping, key: &str) -> bool {
    raw.keys()
        .filter_map(serde_yaml::Value::as_str)
        .any(|stored_key| name_to_key(stored_key) == key)
        || parse_extensions_map(raw)
            .values()
            .any(|entry| entry.config.key() == key)
}

fn extension_identity_key(raw: &Mapping, key: &str) -> Option<serde_yaml::Value> {
    raw.iter().find_map(|(stored_key, value)| {
        let stored_key_str = stored_key.as_str()?;
        let value = inject_name_if_missing(stored_key_str, value.clone());
        serde_yaml::from_value::<ExtensionEntry>(value)
            .ok()
            .filter(|entry| entry.config.key() == key)
            .map(|_| stored_key.clone())
    })
}

fn validate_extension_add_with_config(
    config: &Config,
    entry: &ExtensionEntry,
) -> Result<(), ExtensionAddError> {
    let raw: Mapping = match config.get_param(EXTENSIONS_CONFIG_KEY) {
        Ok(raw) => raw,
        Err(ConfigError::NotFound(_)) => Mapping::default(),
        Err(err) => return Err(err.into()),
    };
    let key = entry.config.key();
    if extension_identity_exists(&raw, &key) {
        return Err(ExtensionAddError::AlreadyExists { key });
    }
    Ok(())
}

/// Checks a prospective add without mutating configuration.
/// [`add_extension`] repeats this check while writing.
pub fn validate_extension_add(entry: &ExtensionEntry) -> Result<(), ExtensionAddError> {
    validate_extension_add_with_config(Config::global(), entry)
}

enum ExtensionMutation {
    Upsert(String, Box<ExtensionEntry>),
    Remove(String),
    Noop,
}

fn with_raw_extensions_mapping<F>(config: &Config, mutate: F)
where
    F: FnOnce(&mut IndexMap<String, ExtensionEntry>) -> ExtensionMutation,
{
    let _guard = EXTENSION_MUTATION_GUARD.lock().unwrap();
    let _file_guard = match config.lock_extension_mutation() {
        Ok(guard) => guard,
        Err(e) => {
            warn!("Failed to lock extensions config: {}", e);
            return;
        }
    };
    let mut serialize_error = None;
    let result = config.update_param::<Mapping, Mapping, _>(EXTENSIONS_CONFIG_KEY, |mut raw| {
        let mut extensions = parse_extensions_map(&raw);

        match mutate(&mut extensions) {
            ExtensionMutation::Upsert(key, entry) => match serde_yaml::to_value(entry) {
                Ok(value) => {
                    raw.insert(serde_yaml::Value::String(key), value);
                }
                Err(err) => {
                    serialize_error = Some(err);
                }
            },
            ExtensionMutation::Remove(key) => {
                raw.shift_remove(key.as_str());
            }
            ExtensionMutation::Noop => {}
        }

        raw
    });

    if let Some(e) = serialize_error {
        warn!("Failed to serialize extensions config entry: {}", e);
    } else if let Err(e) = result {
        warn!("Failed to save extensions config: {}", e);
    }
}

pub fn get_extension_by_name(name: &str) -> Option<ExtensionConfig> {
    get_extension_by_name_with_config(Config::global(), name)
}

fn get_extension_by_name_with_config(config: &Config, name: &str) -> Option<ExtensionConfig> {
    let extensions = get_extensions_map_with_config(config);
    let key = name_to_key(name);

    if let Some(entry) = extensions
        .values()
        .find(|entry| entry.config.name() == name)
        .or_else(|| extensions.get(&key))
    {
        return Some(entry.config.clone());
    }

    get_available_extensions()
        .into_iter()
        .find(|config| config.name() == name || config.key() == key)
}

/// Inserts or replaces an extension. Add flows should use [`add_extension`].
pub fn set_extension(entry: ExtensionEntry) {
    set_extension_with_config(Config::global(), entry);
}

fn set_extension_with_config(config: &Config, entry: ExtensionEntry) {
    let key = entry.config.key();
    with_raw_extensions_mapping(config, |_| ExtensionMutation::Upsert(key, Box::new(entry)));
}

/// Adds an extension only when its canonical identity is absent.
pub fn add_extension(entry: ExtensionEntry) -> Result<(), ExtensionAddError> {
    add_extension_with_config(Config::global(), entry)
}

fn add_extension_with_config(
    config: &Config,
    entry: ExtensionEntry,
) -> Result<(), ExtensionAddError> {
    let _guard = EXTENSION_MUTATION_GUARD.lock().unwrap();
    let _file_guard = config.lock_extension_mutation()?;
    add_extension_with_config_locked(config, entry)
}

pub fn add_extension_with_secrets(
    entry: ExtensionEntry,
    secret_updates: &[(String, serde_json::Value)],
) -> Result<(), ExtensionAddError> {
    add_extension_with_secrets_with_config(Config::global(), entry, secret_updates)
}

fn add_extension_with_secrets_with_config(
    config: &Config,
    entry: ExtensionEntry,
    secret_updates: &[(String, serde_json::Value)],
) -> Result<(), ExtensionAddError> {
    let _guard = EXTENSION_MUTATION_GUARD.lock().unwrap();
    let _file_guard = config.lock_extension_mutation()?;
    validate_extension_add_with_config(config, &entry)?;
    persist_with_secret_updates(config, secret_updates, || {
        add_extension_with_config_locked(config, entry)
    })
}

fn persist_with_secret_updates<E>(
    config: &Config,
    secret_updates: &[(String, serde_json::Value)],
    persist: impl FnOnce() -> Result<(), E>,
) -> Result<(), E>
where
    E: From<ConfigError>,
{
    if secret_updates.is_empty() {
        return persist();
    }

    let existing = config.all_secrets().map_err(E::from)?;
    let snapshot = secret_updates
        .iter()
        .map(|(key, _)| (key.clone(), existing.get(key).cloned()))
        .collect::<Vec<_>>();
    config.set_secret_values(secret_updates).map_err(E::from)?;

    if let Err(error) = persist() {
        config
            .restore_secret_values_if_unchanged(&snapshot, secret_updates)
            .map_err(E::from)?;
        return Err(error);
    }
    Ok(())
}

fn add_extension_with_config_locked(
    config: &Config,
    entry: ExtensionEntry,
) -> Result<(), ExtensionAddError> {
    validate_extension_add_with_config(config, &entry)?;
    let key = entry.config.key();
    let value = serde_yaml::to_value(entry).map_err(ConfigError::from)?;
    let mut collision = false;
    config.update_param::<Mapping, Mapping, _>(EXTENSIONS_CONFIG_KEY, |mut raw| {
        if extension_identity_exists(&raw, &key) {
            collision = true;
        } else {
            raw.insert(serde_yaml::Value::String(key.clone()), value);
        }
        raw
    })?;

    if collision {
        return Err(ExtensionAddError::AlreadyExists { key });
    }
    Ok(())
}

fn validate_extension_update_with_config(
    config: &Config,
    key: &str,
    entry: &ExtensionEntry,
) -> Result<serde_yaml::Value, ExtensionUpdateError> {
    let key = name_to_key(key);
    let new_key = entry.config.key();
    if new_key != key {
        return Err(ExtensionUpdateError::IdentityChanged { key, new_key });
    }

    let raw: Mapping = config.get_param(EXTENSIONS_CONFIG_KEY)?;
    extension_identity_key(&raw, &key).ok_or(ExtensionUpdateError::NotFound { key })
}

pub fn validate_extension_update(
    key: &str,
    entry: &ExtensionEntry,
) -> Result<(), ExtensionUpdateError> {
    validate_extension_update_with_config(Config::global(), key, entry).map(|_| ())
}

/// Replaces an existing extension without changing its canonical identity.
pub fn update_extension(key: &str, entry: ExtensionEntry) -> Result<(), ExtensionUpdateError> {
    update_extension_with_config(Config::global(), key, entry)
}

fn update_extension_with_config(
    config: &Config,
    key: &str,
    entry: ExtensionEntry,
) -> Result<(), ExtensionUpdateError> {
    let _guard = EXTENSION_MUTATION_GUARD.lock().unwrap();
    let _file_guard = config.lock_extension_mutation()?;
    update_extension_with_config_locked(config, key, entry)
}

pub fn update_extension_with_secrets(
    key: &str,
    entry: ExtensionEntry,
    secret_updates: &[(String, serde_json::Value)],
) -> Result<(), ExtensionUpdateError> {
    update_extension_with_secrets_with_config(Config::global(), key, entry, secret_updates)
}

fn update_extension_with_secrets_with_config(
    config: &Config,
    key: &str,
    entry: ExtensionEntry,
    secret_updates: &[(String, serde_json::Value)],
) -> Result<(), ExtensionUpdateError> {
    let _guard = EXTENSION_MUTATION_GUARD.lock().unwrap();
    let _file_guard = config.lock_extension_mutation()?;
    validate_extension_update_with_config(config, key, &entry)?;
    persist_with_secret_updates(config, secret_updates, || {
        update_extension_with_config_locked(config, key, entry)
    })
}

fn update_extension_with_config_locked(
    config: &Config,
    key: &str,
    entry: ExtensionEntry,
) -> Result<(), ExtensionUpdateError> {
    let stored_key = validate_extension_update_with_config(config, key, &entry)?;
    let value = serde_yaml::to_value(entry).map_err(ConfigError::from)?;
    config.update_param::<Mapping, Mapping, _>(EXTENSIONS_CONFIG_KEY, |mut raw| {
        raw.insert(stored_key, value);
        raw
    })?;
    Ok(())
}

pub fn remove_extension(key: &str) {
    remove_extension_with_config(Config::global(), key);
}

fn remove_extension_with_config(config: &Config, key: &str) {
    with_raw_extensions_mapping(config, |_| ExtensionMutation::Remove(key.to_string()));
}

/// Returns true when an existing extension was updated, false when the key was missing.
pub fn set_extension_enabled(key: &str, enabled: bool) -> bool {
    set_extension_enabled_with_config(Config::global(), key, enabled)
}

fn set_extension_enabled_with_config(config: &Config, key: &str, enabled: bool) -> bool {
    let mut updated = false;
    with_raw_extensions_mapping(config, |extensions| {
        let Some(entry) = extensions.get_mut(key) else {
            return ExtensionMutation::Noop;
        };

        entry.enabled = enabled;
        updated = true;
        ExtensionMutation::Upsert(key.to_string(), Box::new(entry.clone()))
    });

    updated
}

pub fn get_all_extensions() -> Vec<ExtensionEntry> {
    let extensions = get_extensions_map();
    extensions.into_values().collect()
}

pub fn get_all_extension_names() -> Vec<String> {
    let extensions = get_extensions_map();
    extensions.keys().cloned().collect()
}

pub fn is_extension_enabled(key: &str) -> bool {
    let extensions = get_extensions_map();
    extensions.get(key).map(|e| e.enabled).unwrap_or(false)
}

/// Returns the configured enabled state for an extension, or `None` when it has no entry.
pub fn configured_enabled_state(config: &Config, name: &str) -> Option<bool> {
    let extensions = get_extensions_map_with_config(config);
    let key = name_to_key(name);
    extensions
        .values()
        .find(|entry| entry.config.name() == name)
        .or_else(|| extensions.get(&key))
        .map(|entry| entry.enabled)
}

pub fn get_enabled_extensions() -> Vec<ExtensionConfig> {
    get_all_extensions()
        .into_iter()
        .filter(|ext| ext.enabled)
        .map(|ext| ext.config)
        .collect()
}

pub fn get_enabled_extensions_with_config(config: &Config) -> Vec<ExtensionConfig> {
    get_extensions_map_with_config(config)
        .into_values()
        .filter(|ext| ext.enabled)
        .map(|ext| ext.config)
        .collect()
}

pub fn get_available_extensions() -> Vec<ExtensionConfig> {
    let mut builtin_names = crate::builtin_extension::get_builtin_extension_names();
    builtin_names.sort_unstable();

    let mut platform_definitions = PLATFORM_EXTENSIONS
        .values()
        .filter(|definition| !definition.hidden)
        .collect::<Vec<_>>();
    platform_definitions.sort_unstable_by_key(|definition| definition.name);

    builtin_names
        .into_iter()
        .map(|name| ExtensionConfig::Builtin {
            name: name.to_string(),
            description: String::new(),
            display_name: Some(name.to_string()),
            timeout: None,
            bundled: Some(true),
            available_tools: Vec::new(),
        })
        .chain(
            platform_definitions
                .into_iter()
                .map(|definition| ExtensionConfig::Platform {
                    name: definition.name.to_string(),
                    description: definition.description.to_string(),
                    display_name: Some(definition.display_name.to_string()),
                    bundled: Some(true),
                    available_tools: Vec::new(),
                }),
        )
        .collect()
}

pub fn get_warnings() -> Vec<String> {
    let raw: Mapping = Config::global()
        .get_param(EXTENSIONS_CONFIG_KEY)
        .unwrap_or_default();

    let mut warnings = Vec::new();
    for (k, v) in raw {
        if let (serde_yaml::Value::String(key), Ok(entry)) =
            (k, serde_yaml::from_value::<ExtensionEntry>(v))
        {
            if matches!(entry.config, ExtensionConfig::Sse { .. }) {
                warnings.push(format!(
                    "'{}': SSE is unsupported, migrate to streamable_http",
                    key
                ));
            }
        }
    }
    warnings
}

pub fn resolve_extensions_for_new_session(
    recipe_extensions: Option<&[ExtensionConfig]>,
    override_extensions: Option<Vec<ExtensionConfig>>,
) -> Vec<ExtensionConfig> {
    let extensions = if let Some(exts) = recipe_extensions {
        exts.to_vec()
    } else if let Some(exts) = override_extensions {
        exts
    } else {
        get_enabled_extensions()
    };

    extensions
        .into_iter()
        .filter(is_extension_available)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fmt;
    use std::sync::{Arc, Barrier, Mutex};
    use tempfile::NamedTempFile;
    use tracing::{Event, Level, Subscriber};
    use tracing_subscriber::layer::SubscriberExt;

    fn test_config(content: &str) -> (Config, NamedTempFile, NamedTempFile) {
        let config_file = NamedTempFile::new().unwrap();
        let secrets_file = NamedTempFile::new().unwrap();
        std::fs::write(config_file.path(), content).unwrap();
        let config =
            Config::new_with_file_secrets(config_file.path(), secrets_file.path()).unwrap();
        (config, config_file, secrets_file)
    }

    fn test_config_with_base(
        base_content: &str,
        user_content: &str,
    ) -> (Config, NamedTempFile, NamedTempFile, NamedTempFile) {
        let base_file = NamedTempFile::new().unwrap();
        let user_file = NamedTempFile::new().unwrap();
        let secrets_file = NamedTempFile::new().unwrap();
        std::fs::write(base_file.path(), base_content).unwrap();
        std::fs::write(user_file.path(), user_content).unwrap();
        let config = Config::new_with_config_paths(
            vec![
                base_file.path().to_path_buf(),
                user_file.path().to_path_buf(),
            ],
            secrets_file.path(),
        )
        .unwrap();
        (config, base_file, user_file, secrets_file)
    }

    fn read_extensions(config: &Config) -> Mapping {
        let content = std::fs::read_to_string(config.path()).unwrap();
        let values: Mapping = serde_yaml::from_str(&content).unwrap();
        values
            .get(EXTENSIONS_CONFIG_KEY)
            .unwrap()
            .as_mapping()
            .unwrap()
            .clone()
    }

    fn builtin_entry(name: &str, enabled: bool) -> ExtensionEntry {
        ExtensionEntry {
            enabled,
            config: ExtensionConfig::Builtin {
                name: name.to_string(),
                description: format!("{name} description"),
                display_name: Some(name.to_string()),
                timeout: None,
                bundled: None,
                available_tools: Vec::new(),
            },
        }
    }

    #[test]
    fn test_is_extension_available_filters_unknown_platform() {
        let unknown_platform = ExtensionConfig::Platform {
            name: "definitely_not_real_platform_extension".to_string(),
            description: "unknown".to_string(),
            display_name: None,
            bundled: None,
            available_tools: Vec::new(),
        };

        let builtin = ExtensionConfig::Builtin {
            name: "developer".to_string(),
            description: "".to_string(),
            display_name: Some("Developer".to_string()),
            timeout: None,
            bundled: None,
            available_tools: Vec::new(),
        };

        assert!(!is_extension_available(&unknown_platform));
        assert!(is_extension_available(&builtin));
    }

    #[test]
    fn test_set_extension_enabled_preserves_clean_siblings() {
        let (config, _config_file, _secrets_file) = test_config(
            r#"
extensions:
  first:
    enabled: true
    type: builtin
    name: first
    description: first description
    display_name: First
  second:
    enabled: true
    type: builtin
    name: second
    description: second description
    display_name: Second
    extra_field: preserved
"#,
        );
        let before = read_extensions(&config);
        let second_before = before.get("second").unwrap().clone();

        set_extension_enabled_with_config(&config, "first", false);

        let extensions = read_extensions(&config);
        assert_eq!(
            extensions
                .get("first")
                .unwrap()
                .as_mapping()
                .unwrap()
                .get("enabled")
                .unwrap()
                .as_bool(),
            Some(false)
        );
        assert_eq!(extensions.get("second").unwrap(), &second_before);
    }

    #[test]
    fn test_set_extension_enabled_preserves_unparseable_sibling() {
        let (config, _config_file, _secrets_file) = test_config(
            r#"
extensions:
  valid:
    enabled: true
    type: builtin
    name: valid
    description: valid description
    display_name: Valid
  broken:
    enabled: true
    type: stdio
    name: Broken
    description: missing cmd
    args: []
"#,
        );
        let before = read_extensions(&config);
        let broken_before = before.get("broken").unwrap().clone();

        set_extension_enabled_with_config(&config, "valid", false);

        let extensions = read_extensions(&config);
        assert!(extensions.contains_key("valid"));
        assert_eq!(extensions.get("broken").unwrap(), &broken_before);
        assert_eq!(
            extensions
                .get("valid")
                .unwrap()
                .as_mapping()
                .unwrap()
                .get("enabled")
                .unwrap()
                .as_bool(),
            Some(false)
        );
    }

    #[test]
    fn test_set_extension_adds_entry_without_dropping_unparseable_entries() {
        let (config, _config_file, _secrets_file) = test_config(
            r#"
extensions:
  broken:
    enabled: true
    type: stdio
    name: Broken
    description: missing cmd
    args: []
"#,
        );
        let before = read_extensions(&config);
        let broken_before = before.get("broken").unwrap().clone();

        set_extension_with_config(&config, builtin_entry("new extension", true));

        let extensions = read_extensions(&config);
        assert_eq!(extensions.get("broken").unwrap(), &broken_before);
        assert!(extensions.contains_key("newextension"));
    }

    #[test]
    fn test_normalized_alias_cannot_replace_existing_extension() {
        let (config, _config_file, _secrets_file) = test_config("");
        let trusted = ExtensionEntry {
            enabled: true,
            config: ExtensionConfig::streamable_http(
                "github",
                "https://trusted.example/mcp",
                "trusted",
                30u64,
            ),
        };
        let replacement = ExtensionEntry {
            enabled: true,
            config: ExtensionConfig::streamable_http(
                "Git Hub",
                "https://replacement.example/mcp",
                "replacement",
                30u64,
            ),
        };
        let permission_dir = tempfile::TempDir::new().unwrap();
        let permission_manager =
            crate::config::PermissionManager::new(permission_dir.path().into());
        let principal = format!("{}__run", trusted.config.key());
        permission_manager.update_user_permission(
            &principal,
            crate::config::permission::PermissionLevel::AlwaysAllow,
        );

        add_extension_with_config(&config, trusted).unwrap();
        let before = read_extensions(&config);
        let replacement_key = replacement.config.key();
        let error = add_extension_with_config(&config, replacement).unwrap_err();

        assert!(matches!(
            error,
            ExtensionAddError::AlreadyExists { key } if key == replacement_key
        ));
        assert_eq!(read_extensions(&config), before);
        assert_eq!(
            permission_manager.get_user_permission(&principal),
            Some(crate::config::permission::PermissionLevel::AlwaysAllow)
        );

        let saved = get_extension_by_name_with_config(&config, "github").unwrap();
        let ExtensionConfig::StreamableHttp { name, uri, .. } = saved else {
            panic!("expected streamable HTTP extension");
        };
        assert_eq!(name, "github");
        assert_eq!(uri, "https://trusted.example/mcp");
    }

    #[test]
    fn test_add_extension_accepts_unique_names() {
        let (config, _config_file, _secrets_file) = test_config("");

        add_extension_with_config(&config, builtin_entry("first extension", true)).unwrap();
        add_extension_with_config(&config, builtin_entry("second extension", true)).unwrap();

        let extensions = read_extensions(&config);
        assert!(extensions.contains_key("firstextension"));
        assert!(extensions.contains_key("secondextension"));
    }

    #[test]
    fn test_concurrent_alias_add_does_not_overwrite_winner_secret() {
        let (config, _config_file, _secrets_file) = test_config("");
        let config = Arc::new(config);
        let barrier = Arc::new(Barrier::new(2));
        let attempts =
            [("Git Hub", "first-secret"), ("github", "second-secret")].map(|(name, secret)| {
                let config = Arc::clone(&config);
                let barrier = Arc::clone(&barrier);
                std::thread::spawn(move || {
                    barrier.wait();
                    let updates = [(
                        "SHARED_TOKEN".to_string(),
                        serde_json::Value::String(secret.to_string()),
                    )];
                    let result = add_extension_with_secrets_with_config(
                        &config,
                        builtin_entry(name, true),
                        &updates,
                    );
                    (result, secret)
                })
            });

        let results = attempts.map(|attempt| attempt.join().unwrap());
        let winner = results
            .iter()
            .find_map(|(result, secret)| result.is_ok().then_some(*secret))
            .unwrap();
        let rejected = results
            .iter()
            .find_map(|(result, secret)| result.is_err().then_some(*secret))
            .unwrap();

        assert_ne!(winner, rejected);
        assert!(results.iter().any(|(result, _)| matches!(
            result,
            Err(ExtensionAddError::AlreadyExists { key }) if key == "github"
        )));
        assert_eq!(config.get_secret::<String>("SHARED_TOKEN").unwrap(), winner);
    }

    #[test]
    fn test_set_extension_remains_explicit_replacement() {
        let (config, _config_file, _secrets_file) = test_config("");

        set_extension_with_config(&config, builtin_entry("replaceable", false));
        set_extension_with_config(&config, builtin_entry("Replaceable", true));

        let saved = get_extension_by_name_with_config(&config, "replaceable").unwrap();
        assert_eq!(saved.name(), "Replaceable");
    }

    #[test]
    fn test_update_extension_replaces_same_identity() {
        let (config, _config_file, _secrets_file) = test_config("");
        add_extension_with_config(&config, builtin_entry("replaceable", false)).unwrap();

        update_extension_with_config(&config, "Replaceable", builtin_entry("Replaceable", true))
            .unwrap();

        let saved = get_extension_by_name_with_config(&config, "replaceable").unwrap();
        assert_eq!(saved.name(), "Replaceable");
        assert!(get_extensions_map_with_config(&config)["replaceable"].enabled);
    }

    #[test]
    fn test_update_extension_rejects_identity_change() {
        let (config, _config_file, _secrets_file) = test_config("");
        add_extension_with_config(&config, builtin_entry("trusted", true)).unwrap();
        let before = read_extensions(&config);

        let error = update_extension_with_config(
            &config,
            "trusted",
            builtin_entry("different extension", false),
        )
        .unwrap_err();

        assert!(matches!(
            error,
            ExtensionUpdateError::IdentityChanged { key, new_key }
                if key == "trusted" && new_key == "differentextension"
        ));
        assert_eq!(read_extensions(&config), before);
    }

    #[test]
    fn test_update_extension_rejects_missing_identity() {
        let (config, _config_file, _secrets_file) = test_config("");

        let error =
            update_extension_with_config(&config, "missing", builtin_entry("missing", false))
                .unwrap_err();

        assert!(matches!(
            error,
            ExtensionUpdateError::NotFound { key } if key == "missing"
        ));
    }

    #[test]
    fn test_update_extension_creates_override_for_inherited_identity() {
        let (config, base_file, _user_file, _secrets_file) = test_config_with_base(
            r#"
extensions:
  inherited-alias:
    enabled: false
    type: builtin
    name: Inherited
    description: inherited description
    display_name: Inherited
"#,
            "",
        );
        let base_before = std::fs::read_to_string(base_file.path()).unwrap();

        update_extension_with_config(&config, "Inherited", builtin_entry("Inherited", true))
            .unwrap();

        let user_extensions = read_extensions(&config);
        assert!(user_extensions.contains_key("inherited-alias"));
        assert!(!user_extensions.contains_key("inherited"));
        assert!(get_extensions_map_with_config(&config)["inherited-alias"].enabled);
        assert_eq!(
            std::fs::read_to_string(base_file.path()).unwrap(),
            base_before
        );
    }

    #[test]
    fn test_update_extension_replaces_partial_user_override() {
        let (config, _base_file, _user_file, _secrets_file) = test_config_with_base(
            r#"
extensions:
  inherited:
    enabled: true
    type: builtin
    name: Inherited
    description: inherited description
    display_name: Inherited
"#,
            r#"
extensions:
  inherited:
    enabled: false
"#,
        );

        update_extension_with_config(&config, "Inherited", builtin_entry("Inherited", true))
            .unwrap();

        let user_extensions = read_extensions(&config);
        let entry: ExtensionEntry =
            serde_yaml::from_value(user_extensions["inherited"].clone()).unwrap();
        assert!(entry.enabled);
        assert_eq!(entry.config.name(), "Inherited");
    }

    #[test]
    fn test_concurrent_updates_keep_config_and_secret_coherent() {
        let (config, _config_file, _secrets_file) = test_config("");
        add_extension_with_config(&config, builtin_entry("shared", true)).unwrap();
        let config = Arc::new(config);
        let barrier = Arc::new(Barrier::new(2));
        let attempts = [
            ("first description", "first-secret"),
            ("second description", "second-secret"),
        ]
        .map(|(description, secret)| {
            let config = Arc::clone(&config);
            let barrier = Arc::clone(&barrier);
            std::thread::spawn(move || {
                barrier.wait();
                let mut entry = builtin_entry("shared", true);
                if let ExtensionConfig::Builtin {
                    description: entry_description,
                    ..
                } = &mut entry.config
                {
                    *entry_description = description.to_string();
                }
                let updates = [(
                    "SHARED_TOKEN".to_string(),
                    serde_json::Value::String(secret.to_string()),
                )];
                update_extension_with_secrets_with_config(&config, "shared", entry, &updates)
                    .unwrap();
            })
        });

        for attempt in attempts {
            attempt.join().unwrap();
        }

        let description = match get_extension_by_name_with_config(&config, "shared").unwrap() {
            ExtensionConfig::Builtin { description, .. } => description,
            other => panic!("expected builtin, got {other:?}"),
        };
        let secret = config.get_secret::<String>("SHARED_TOKEN").unwrap();
        assert!(matches!(
            (description.as_str(), secret.as_str()),
            ("first description", "first-secret") | ("second description", "second-secret")
        ));
    }

    #[test]
    fn extension_mutation_subprocess() {
        let Ok(config_path) = std::env::var("GOOSE_TEST_EXTENSION_CONFIG_PATH") else {
            return;
        };
        let secrets_path = std::env::var("GOOSE_TEST_EXTENSION_SECRETS_PATH").unwrap();
        let ready_path = std::env::var("GOOSE_TEST_EXTENSION_READY_PATH").unwrap();
        let operation = std::env::var("GOOSE_TEST_EXTENSION_OPERATION").unwrap();
        let config = Config::new_with_file_secrets(config_path, secrets_path).unwrap();
        std::fs::write(ready_path, "ready").unwrap();

        match operation.as_str() {
            "set" => set_extension_with_config(&config, builtin_entry("independent", true)),
            "remove" => remove_extension_with_config(&config, "initial"),
            "enable" => assert!(set_extension_enabled_with_config(&config, "initial", false)),
            other => panic!("unknown operation {other}"),
        }
    }

    #[cfg(unix)]
    #[test]
    fn test_all_persistent_mutations_share_add_transaction_lock() {
        for operation in ["set", "remove", "enable"] {
            let directory = tempfile::tempdir().unwrap();
            let config_path = directory.path().join("config.yaml");
            let secrets_path = directory.path().join("secrets.yaml");
            let ready_path = directory.path().join("ready");
            std::fs::write(&config_path, "").unwrap();
            std::fs::write(&secrets_path, "{}\n").unwrap();
            let config = Config::new_with_file_secrets(&config_path, &secrets_path).unwrap();
            add_extension_with_config(&config, builtin_entry("initial", true)).unwrap();

            let file_guard = config.lock_extension_mutation().unwrap();
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .arg("--exact")
                .arg("config::extensions::tests::extension_mutation_subprocess")
                .arg("--nocapture")
                .env("GOOSE_TEST_EXTENSION_CONFIG_PATH", &config_path)
                .env("GOOSE_TEST_EXTENSION_SECRETS_PATH", &secrets_path)
                .env("GOOSE_TEST_EXTENSION_READY_PATH", &ready_path)
                .env("GOOSE_TEST_EXTENSION_OPERATION", operation)
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();

            for _ in 0..500 {
                if ready_path.exists() {
                    break;
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            assert!(ready_path.exists(), "{operation} subprocess did not start");
            std::thread::sleep(std::time::Duration::from_millis(100));
            assert!(
                child.try_wait().unwrap().is_none(),
                "{operation} mutation bypassed the transaction lock"
            );

            add_extension_with_config_locked(&config, builtin_entry("transactional", true))
                .unwrap();
            drop(file_guard);
            let output = child.wait_with_output().unwrap();
            assert!(
                output.status.success(),
                "{operation} subprocess failed:\nstdout: {}\nstderr: {}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );

            let extensions = get_extensions_map_with_config(&config);
            assert!(extensions.contains_key("transactional"));
            match operation {
                "set" => assert!(extensions.contains_key("independent")),
                "remove" => assert!(!extensions.contains_key("initial")),
                "enable" => assert!(!extensions["initial"].enabled),
                _ => unreachable!(),
            }
        }
    }

    #[test]
    fn test_failed_persist_does_not_restore_over_newer_secret() {
        let directory = tempfile::tempdir().unwrap();
        let config_path = directory.path().join("config.yaml");
        let secrets_path = directory.path().join("secrets.yaml");
        let first = Config::new_with_file_secrets(&config_path, &secrets_path).unwrap();
        let second = Config::new_with_file_secrets(&config_path, &secrets_path).unwrap();
        first
            .set_secret("SHARED_TOKEN", &"original-secret")
            .unwrap();
        let updates = [(
            "SHARED_TOKEN".to_string(),
            serde_json::Value::String("rejected-secret".to_string()),
        )];

        let result: Result<(), ConfigError> = persist_with_secret_updates(&first, &updates, || {
            second.set_secret("SHARED_TOKEN", &"newer-secret")?;
            Err(ConfigError::NotFound("forced failure".to_string()))
        });

        assert!(matches!(result, Err(ConfigError::NotFound(_))));
        assert_eq!(
            first.get_secret::<String>("SHARED_TOKEN").unwrap(),
            "newer-secret"
        );
    }

    #[test]
    fn test_update_restores_secrets_when_config_write_fails() {
        let base_file = NamedTempFile::new().unwrap();
        let invalid_write_path = tempfile::tempdir().unwrap();
        let secrets_file = NamedTempFile::new().unwrap();
        std::fs::write(
            base_file.path(),
            r#"
extensions:
  shared:
    enabled: true
    type: builtin
    name: shared
    description: original description
    display_name: Shared
"#,
        )
        .unwrap();
        let config = Config::new_with_config_paths(
            vec![
                base_file.path().to_path_buf(),
                invalid_write_path.path().to_path_buf(),
            ],
            secrets_file.path(),
        )
        .unwrap();
        config
            .set_secret("SHARED_TOKEN", &"original-secret")
            .unwrap();
        let updates = [(
            "SHARED_TOKEN".to_string(),
            serde_json::Value::String("rejected-secret".to_string()),
        )];

        let error = update_extension_with_secrets_with_config(
            &config,
            "shared",
            builtin_entry("shared", false),
            &updates,
        )
        .unwrap_err();

        assert!(matches!(error, ExtensionUpdateError::Config(_)));
        assert_eq!(
            config.get_secret::<String>("SHARED_TOKEN").unwrap(),
            "original-secret"
        );
    }

    #[test]
    fn test_get_extension_by_name_falls_back_to_available_builtin() {
        fn spawn_builtin(_: tokio::io::DuplexStream, _: tokio::io::DuplexStream) {}
        crate::builtin_extension::register_builtin_extension("memory", spawn_builtin);

        let extension = get_extension_by_name("memory").unwrap();

        assert!(matches!(
            extension,
            ExtensionConfig::Builtin { ref name, .. } if name == "memory"
        ));
    }

    #[test]
    fn test_get_extension_by_name_resolves_saved_entry_by_key() {
        let saved = ExtensionEntry {
            enabled: true,
            config: ExtensionConfig::Stdio {
                name: "My Tool".to_string(),
                description: "saved description".to_string(),
                cmd: "my-tool".to_string(),
                args: Vec::new(),
                envs: Default::default(),
                env_keys: Vec::new(),
                timeout: Some(120),
                cwd: None,
                bundled: None,
                available_tools: vec!["run".to_string()],
            },
        };
        let key = saved.config.key();
        assert_ne!(key, saved.config.name());

        let (config, _config_file, _secrets_file) = test_config("");
        set_extension_with_config(&config, saved);

        let resolved = get_extension_by_name_with_config(&config, &key).unwrap();

        match resolved {
            ExtensionConfig::Stdio {
                timeout,
                available_tools,
                ..
            } => {
                assert_eq!(timeout, Some(120));
                assert_eq!(available_tools, vec!["run".to_string()]);
            }
            other => panic!("expected stdio, got {other:?}"),
        }
    }

    #[test]
    fn test_remove_extension_preserves_unparseable_sibling() {
        let (config, _config_file, _secrets_file) = test_config(
            r#"
extensions:
  valid:
    enabled: true
    type: builtin
    name: valid
    description: valid description
    display_name: Valid
  broken:
    enabled: true
    type: stdio
    name: Broken
    description: missing cmd
    args: []
"#,
        );
        let before = read_extensions(&config);
        let broken_before = before.get("broken").unwrap().clone();

        remove_extension_with_config(&config, "valid");

        let extensions = read_extensions(&config);
        assert!(!extensions.contains_key("valid"));
        assert_eq!(extensions.get("broken").unwrap(), &broken_before);
    }

    #[derive(Clone, Default)]
    struct CapturedLogs {
        events: Arc<Mutex<Vec<CapturedEvent>>>,
    }

    #[derive(Debug)]
    struct CapturedEvent {
        level: Level,
        message: String,
        key: Option<String>,
    }

    impl<S> tracing_subscriber::Layer<S> for CapturedLogs
    where
        S: Subscriber,
    {
        fn on_event(&self, event: &Event<'_>, _ctx: tracing_subscriber::layer::Context<'_, S>) {
            let mut visitor = EventVisitor::default();
            event.record(&mut visitor);
            self.events.lock().unwrap().push(CapturedEvent {
                level: *event.metadata().level(),
                message: visitor.message,
                key: visitor.key,
            });
        }
    }

    #[derive(Default)]
    struct EventVisitor {
        message: String,
        key: Option<String>,
    }

    impl tracing::field::Visit for EventVisitor {
        fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
            match field.name() {
                "message" => self.message = value.to_string(),
                "key" => self.key = Some(value.to_string()),
                _ => {}
            }
        }

        fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn fmt::Debug) {
            match field.name() {
                "message" => self.message = format!("{value:?}").trim_matches('"').to_string(),
                "key" => {
                    self.key = Some(format!("{value:?}").trim_matches('"').to_string());
                }
                _ => {}
            }
        }
    }

    #[test]
    fn test_stdio_without_name_uses_map_key() {
        let (config, _config_file, _secrets_file) = test_config(
            r#"
extensions:
  firecrawl:
    enabled: true
    type: stdio
    cmd: npx
    args: ["-y", "firecrawl-mcp"]
    envs:
      FIRECRAWL_API_KEY: test-key
"#,
        );

        let extensions = get_extensions_map_with_config(&config);
        let entry = extensions
            .get("firecrawl")
            .expect("firecrawl extension should parse");
        assert_eq!(entry.config.name(), "firecrawl");
        assert!(entry.enabled);
    }

    #[test]
    fn test_stdio_env_alias_accepted() {
        let (config, _config_file, _secrets_file) = test_config(
            r#"
extensions:
  brave-search:
    enabled: true
    type: stdio
    name: brave-search
    cmd: npx
    args: ["-y", "@modelcontextprotocol/server-brave-search"]
    env:
      BRAVE_API_KEY: test-key
"#,
        );

        let extensions = get_extensions_map_with_config(&config);
        let entry = extensions
            .get("brave-search")
            .expect("brave-search extension should parse");
        match &entry.config {
            ExtensionConfig::Stdio { envs, .. } => {
                assert_eq!(
                    envs.get_env().get("BRAVE_API_KEY"),
                    Some(&"test-key".to_string())
                );
            }
            other => panic!("expected Stdio, got {other:?}"),
        }
    }

    #[test]
    fn test_stdio_env_alias_without_name_uses_map_key() {
        let (config, _config_file, _secrets_file) = test_config(
            r#"
extensions:
  brave-search:
    enabled: true
    type: stdio
    cmd: npx
    args: ["-y", "@modelcontextprotocol/server-brave-search"]
    env:
      BRAVE_API_KEY: test-key
"#,
        );

        let extensions = get_extensions_map_with_config(&config);
        let entry = extensions
            .get("brave-search")
            .expect("brave-search extension should parse when name is missing and env: is used");
        assert_eq!(entry.config.name(), "brave-search");
        match &entry.config {
            ExtensionConfig::Stdio { envs, .. } => {
                assert_eq!(
                    envs.get_env().get("BRAVE_API_KEY"),
                    Some(&"test-key".to_string())
                );
            }
            other => panic!("expected Stdio, got {other:?}"),
        }
    }

    #[test]
    fn test_deserialization_failure_logs_offending_key() {
        let (config, _config_file, _secrets_file) = test_config(
            r#"
extensions:
  valid:
    enabled: true
    type: builtin
    name: valid
    description: valid description
    display_name: Valid
  broken:
    enabled: true
    type: stdio
    name: Broken
    description: missing cmd
    args: []
"#,
        );
        let logs = CapturedLogs::default();
        let subscriber = tracing_subscriber::registry().with(logs.clone());

        tracing::subscriber::with_default(subscriber, || {
            let extensions = get_enabled_extensions_with_config(&config);
            // Bundled platform extensions are auto-injected; filter to user-declared entries
            // (Builtin or anything with the test YAML's names) for the invariant check.
            let user_names: Vec<&str> = extensions
                .iter()
                .filter_map(|ext| match ext {
                    ExtensionConfig::Builtin { name, .. } => Some(name.as_str()),
                    _ => None,
                })
                .collect();
            assert_eq!(
                user_names,
                vec!["valid"],
                "expected only the parseable user extension to be enabled, got {:?}",
                user_names
            );
        });

        let matching_events: Vec<_> = logs
            .events
            .lock()
            .unwrap()
            .iter()
            .filter(|event| {
                event.level == Level::INFO
                    && event
                        .message
                        .contains("Skipping malformed extension config entry")
            })
            .map(|event| event.key.clone())
            .collect();

        let broken_logs: Vec<_> = matching_events
            .iter()
            .filter(|k| k.as_deref() == Some("broken"))
            .collect();
        assert!(
            !broken_logs.is_empty(),
            "expected at least one log naming the broken extension key, got {:?}",
            matching_events
        );
        let other_keys: Vec<_> = matching_events
            .iter()
            .filter(|k| k.as_deref() != Some("broken"))
            .collect();
        assert!(
            other_keys.is_empty(),
            "expected no logs for other extension keys, got {:?}",
            other_keys
        );
    }

    #[test]
    fn test_configured_enabled_state_unknown_extension_is_none() {
        let (config, _config_file, _secrets_file) = test_config("");

        assert_eq!(
            configured_enabled_state(&config, "not_a_real_extension"),
            None
        );
    }

    #[test]
    fn test_default_on_extension_enabled_when_config_empty() {
        let (config, _config_file, _secrets_file) = test_config("");

        assert_eq!(configured_enabled_state(&config, "developer"), Some(true));
    }

    #[test]
    fn test_configured_enabled_state_tracks_changes() {
        let (config, _config_file, _secrets_file) = test_config("");
        set_extension_with_config(&config, builtin_entry("developer", false));

        assert_eq!(configured_enabled_state(&config, "developer"), Some(false));

        set_extension_enabled_with_config(&config, "developer", true);
        assert_eq!(configured_enabled_state(&config, "developer"), Some(true));
    }

    #[test]
    fn test_default_off_extension_disabled_when_config_empty() {
        let (config, _config_file, _secrets_file) = test_config("");

        assert_eq!(configured_enabled_state(&config, "chatrecall"), Some(false));
    }
}
