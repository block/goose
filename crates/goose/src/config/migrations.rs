use crate::agents::extension::PLATFORM_EXTENSIONS;
use crate::agents::ExtensionConfig;
use crate::config::extensions::ExtensionEntry;
use serde_yaml::{Mapping, Value};

const EXTENSIONS_CONFIG_KEY: &str = "extensions";
const DEPRECATED_PLATFORM_EXTENSIONS: [&str; 1] = ["skills"];

pub fn run_migrations(config: &mut Mapping) -> bool {
    let mut changed = false;
    changed |= migrate_platform_extensions(config);
    changed |= remove_deprecated_platform_extensions(config);
    changed
}

fn migrate_platform_extensions(config: &mut Mapping) -> bool {
    let extensions_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());

    let extensions_value = config
        .get(&extensions_key)
        .cloned()
        .unwrap_or(serde_yaml::Value::Mapping(Mapping::new()));

    let mut extensions_map: Mapping = match extensions_value {
        serde_yaml::Value::Mapping(m) => m,
        _ => Mapping::new(),
    };

    let mut needs_save = false;

    for (name, def) in PLATFORM_EXTENSIONS.iter() {
        let ext_key = serde_yaml::Value::String(name.to_string());
        let existing = extensions_map.get(&ext_key);

        let needs_migration = match existing {
            None => true,
            Some(value) => match serde_yaml::from_value::<ExtensionEntry>(value.clone()) {
                Ok(entry) => {
                    if let ExtensionConfig::Platform {
                        description,
                        display_name,
                        ..
                    } = &entry.config
                    {
                        description != def.description
                            || display_name.as_deref() != Some(def.display_name)
                    } else {
                        true
                    }
                }
                Err(_) => true,
            },
        };

        if needs_migration {
            let enabled = existing
                .and_then(|v| serde_yaml::from_value::<ExtensionEntry>(v.clone()).ok())
                .map(|e| e.enabled)
                .unwrap_or(def.default_enabled);

            let new_entry = ExtensionEntry {
                config: ExtensionConfig::Platform {
                    name: def.name.to_string(),
                    description: def.description.to_string(),
                    display_name: Some(def.display_name.to_string()),
                    bundled: Some(true),
                    available_tools: Vec::new(),
                },
                enabled,
            };

            if let Ok(value) = serde_yaml::to_value(&new_entry) {
                extensions_map.insert(ext_key, value);
                needs_save = true;
            }
        }
    }

    if needs_save {
        config.insert(extensions_key, serde_yaml::Value::Mapping(extensions_map));
    }

    needs_save
}

fn remove_deprecated_platform_extensions(config: &mut Mapping) -> bool {
    let extensions_key = Value::String(EXTENSIONS_CONFIG_KEY.to_string());

    let extensions_value = config
        .get(&extensions_key)
        .cloned()
        .unwrap_or(Value::Mapping(Mapping::new()));

    let mut extensions_map: Mapping = match extensions_value {
        Value::Mapping(m) => m,
        _ => Mapping::new(),
    };

    let keys_to_remove: Vec<Value> = extensions_map
        .iter()
        .filter_map(|(key, value)| {
            let entry = serde_yaml::from_value::<ExtensionEntry>(value.clone()).ok()?;
            match entry.config {
                ExtensionConfig::Platform { name, .. }
                    if DEPRECATED_PLATFORM_EXTENSIONS
                        .iter()
                        .any(|deprecated| deprecated.eq_ignore_ascii_case(name.as_str())) =>
                {
                    Some(key.clone())
                }
                _ => None,
            }
        })
        .collect();

    if keys_to_remove.is_empty() {
        return false;
    }

    for key in keys_to_remove {
        extensions_map.remove(&key);
    }

    config.insert(extensions_key, Value::Mapping(extensions_map));
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_migrate_platform_extensions_empty_config() {
        let mut config = Mapping::new();
        let changed = run_migrations(&mut config);

        assert!(changed);
        let extensions_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());
        assert!(config.contains_key(&extensions_key));
    }

    #[test]
    fn test_migrate_platform_extensions_preserves_enabled_state() {
        let mut config = Mapping::new();
        let mut extensions = Mapping::new();
        let todo_entry = ExtensionEntry {
            config: ExtensionConfig::Platform {
                name: "todo".to_string(),
                description: "old description".to_string(),
                display_name: Some("Old Name".to_string()),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: false,
        };
        extensions.insert(
            serde_yaml::Value::String("todo".to_string()),
            serde_yaml::to_value(&todo_entry).unwrap(),
        );
        config.insert(
            serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string()),
            serde_yaml::Value::Mapping(extensions),
        );

        let changed = run_migrations(&mut config);
        assert!(changed);

        let extensions_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());
        let extensions = config.get(&extensions_key).unwrap().as_mapping().unwrap();
        let todo_key = serde_yaml::Value::String("todo".to_string());
        let todo_value = extensions.get(&todo_key).unwrap();
        let todo_entry: ExtensionEntry = serde_yaml::from_value(todo_value.clone()).unwrap();

        assert!(!todo_entry.enabled);
    }

    #[test]
    fn test_migrate_platform_extensions_idempotent() {
        let mut config = Mapping::new();
        run_migrations(&mut config);

        let changed = run_migrations(&mut config);
        assert!(!changed);
    }

    #[test]
    fn test_remove_deprecated_skills_platform_extension() {
        let mut config = Mapping::new();
        let mut extensions = Mapping::new();
        let skills_entry = ExtensionEntry {
            config: ExtensionConfig::Platform {
                name: "skills".to_string(),
                description: "deprecated".to_string(),
                display_name: Some("Skills".to_string()),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: true,
        };
        extensions.insert(
            Value::String("skills".to_string()),
            serde_yaml::to_value(&skills_entry).unwrap(),
        );
        config.insert(
            Value::String(EXTENSIONS_CONFIG_KEY.to_string()),
            Value::Mapping(extensions),
        );

        let changed = run_migrations(&mut config);
        assert!(changed);

        let extensions_key = Value::String(EXTENSIONS_CONFIG_KEY.to_string());
        let extensions = config.get(&extensions_key).unwrap().as_mapping().unwrap();
        assert!(!extensions.contains_key(&Value::String("skills".to_string())));
    }
}
