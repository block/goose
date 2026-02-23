use crate::agents::extension::PLATFORM_EXTENSIONS;
use crate::agents::ExtensionConfig;
use crate::config::extensions::ExtensionEntry;
use crate::config::DEFAULT_EXTENSION_TIMEOUT;
use serde_yaml::Mapping;

const EXTENSIONS_CONFIG_KEY: &str = "extensions";

pub fn run_migrations(config: &mut Mapping) -> bool {
    let mut changed = false;
    changed |= migrate_platform_extensions(config);
    changed |= migrate_builtin_extensions(config);
    changed
}

fn migrate_builtin_extensions(config: &mut Mapping) -> bool {
    let extensions_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());

    let extensions_value = config
        .get(&extensions_key)
        .cloned()
        .unwrap_or(serde_yaml::Value::Mapping(Mapping::new()));

    let mut extensions_map: Mapping = match extensions_value {
        serde_yaml::Value::Mapping(m) => m,
        _ => Mapping::new(),
    };

    let ext_key = serde_yaml::Value::String("genui_service".to_string());
    let existing = extensions_map.get(&ext_key);

    let needs_migration = match existing {
        None => true,
        Some(value) => match serde_yaml::from_value::<ExtensionEntry>(value.clone()) {
            Ok(entry) => {
                if let ExtensionConfig::Builtin {
                    description,
                    display_name,
                    timeout,
                    ..
                } = &entry.config
                {
                    description != "Generate DS-first json-render specs using MCP Sampling (JSONL patches only)"
                        || display_name.as_deref() != Some("GenUI Service")
                        || *timeout != Some(DEFAULT_EXTENSION_TIMEOUT)
                } else {
                    true
                }
            }
            Err(_) => true,
        },
    };

    if !needs_migration {
        return false;
    }

    let enabled = existing
        .and_then(|v| serde_yaml::from_value::<ExtensionEntry>(v.clone()).ok())
        .map(|e| e.enabled)
        .unwrap_or(true);

    let new_entry = ExtensionEntry {
        config: ExtensionConfig::Builtin {
            name: "genui_service".to_string(),
            description:
                "Generate DS-first json-render specs using MCP Sampling (JSONL patches only)"
                    .to_string(),
            display_name: Some("GenUI Service".to_string()),
            timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
            bundled: Some(true),
            available_tools: Vec::new(),
        },
        enabled,
    };

    if let Ok(value) = serde_yaml::to_value(&new_entry) {
        extensions_map.insert(ext_key, value);
        config.insert(extensions_key, serde_yaml::Value::Mapping(extensions_map));
        return true;
    }

    false
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
    fn test_migrate_builtin_extensions_enabled_by_default() {
        let mut config = Mapping::new();
        let changed = run_migrations(&mut config);
        assert!(changed);

        let extensions_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());
        let extensions = config.get(&extensions_key).unwrap().as_mapping().unwrap();
        let key = serde_yaml::Value::String("genui_service".to_string());
        let value = extensions.get(&key).unwrap();
        let entry: ExtensionEntry = serde_yaml::from_value(value.clone()).unwrap();

        assert!(entry.enabled);
        match entry.config {
            ExtensionConfig::Builtin {
                name,
                display_name,
                timeout,
                ..
            } => {
                assert_eq!(name, "genui_service");
                assert_eq!(display_name.as_deref(), Some("GenUI Service"));
                assert_eq!(timeout, Some(DEFAULT_EXTENSION_TIMEOUT));
            }
            other => panic!("expected builtin extension, got: {other:?}"),
        }
    }

    #[test]
    fn test_migrate_builtin_extensions_preserves_enabled_state() {
        let mut config = Mapping::new();
        let mut extensions = Mapping::new();
        let entry = ExtensionEntry {
            config: ExtensionConfig::Builtin {
                name: "genui_service".to_string(),
                description: "old description".to_string(),
                display_name: Some("Old Name".to_string()),
                timeout: Some(DEFAULT_EXTENSION_TIMEOUT),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: false,
        };
        extensions.insert(
            serde_yaml::Value::String("genui_service".to_string()),
            serde_yaml::to_value(&entry).unwrap(),
        );
        config.insert(
            serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string()),
            serde_yaml::Value::Mapping(extensions),
        );

        let changed = run_migrations(&mut config);
        assert!(changed);

        let extensions_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());
        let extensions = config.get(&extensions_key).unwrap().as_mapping().unwrap();
        let key = serde_yaml::Value::String("genui_service".to_string());
        let value = extensions.get(&key).unwrap();
        let migrated: ExtensionEntry = serde_yaml::from_value(value.clone()).unwrap();
        assert!(!migrated.enabled);
    }
}
