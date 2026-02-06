use crate::agents::extension::PLATFORM_EXTENSIONS;
use crate::agents::ExtensionConfig;
use crate::config::extensions::ExtensionEntry;
use goose_mcp::BUILTIN_EXTENSION_DEFS;
use serde_yaml::Mapping;

const EXTENSIONS_CONFIG_KEY: &str = "extensions";

pub fn run_migrations(config: &mut Mapping) -> bool {
    let mut changed = false;
    changed |= migrate_platform_extensions(config);
    changed |= migrate_builtin_extensions(config);
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

    let mut needs_save = false;

    for (name, def) in BUILTIN_EXTENSION_DEFS.iter() {
        let ext_key = serde_yaml::Value::String(name.to_string());
        let existing = extensions_map.get(&ext_key);

        let needs_migration = match existing {
            None => true,
            Some(value) => match serde_yaml::from_value::<ExtensionEntry>(value.clone()) {
                Ok(entry) => {
                    if let ExtensionConfig::Builtin {
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
                config: ExtensionConfig::Builtin {
                    name: def.name.to_string(),
                    description: def.description.to_string(),
                    display_name: Some(def.display_name.to_string()),
                    timeout: None,
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
    fn test_migrate_builtin_extensions_preserves_enabled_state() {
        let mut config = Mapping::new();
        let mut extensions = Mapping::new();
        let developer_entry = ExtensionEntry {
            config: ExtensionConfig::Builtin {
                name: "developer".to_string(),
                description: "old description".to_string(),
                display_name: Some("Old Name".to_string()),
                timeout: Some(300),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: false,
        };
        extensions.insert(
            serde_yaml::Value::String("developer".to_string()),
            serde_yaml::to_value(&developer_entry).unwrap(),
        );
        config.insert(
            serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string()),
            serde_yaml::Value::Mapping(extensions),
        );

        let changed = run_migrations(&mut config);
        assert!(changed);

        let extensions_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());
        let extensions = config.get(&extensions_key).unwrap().as_mapping().unwrap();
        let developer_key = serde_yaml::Value::String("developer".to_string());
        let developer_value = extensions.get(&developer_key).unwrap();
        let developer_entry: ExtensionEntry =
            serde_yaml::from_value(developer_value.clone()).unwrap();

        assert!(!developer_entry.enabled);

        if let ExtensionConfig::Builtin {
            display_name,
            description,
            ..
        } = &developer_entry.config
        {
            assert_eq!(display_name.as_deref(), Some("Developer"));
            assert_eq!(
                description,
                "General development tools useful for software engineering."
            );
        } else {
            panic!("Expected Builtin config");
        }
    }

    #[test]
    fn test_migrate_builtin_extensions_updates_outdated_metadata() {
        let mut config = Mapping::new();
        let mut extensions = Mapping::new();
        let memory_entry = ExtensionEntry {
            config: ExtensionConfig::Builtin {
                name: "memory".to_string(),
                description: "outdated description".to_string(),
                display_name: Some("Wrong Name".to_string()),
                timeout: Some(300),
                bundled: Some(true),
                available_tools: Vec::new(),
            },
            enabled: true,
        };
        extensions.insert(
            serde_yaml::Value::String("memory".to_string()),
            serde_yaml::to_value(&memory_entry).unwrap(),
        );
        config.insert(
            serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string()),
            serde_yaml::Value::Mapping(extensions),
        );

        let changed = run_migrations(&mut config);
        assert!(changed);

        let extensions_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());
        let extensions = config.get(&extensions_key).unwrap().as_mapping().unwrap();
        let memory_key = serde_yaml::Value::String("memory".to_string());
        let memory_value = extensions.get(&memory_key).unwrap();
        let memory_entry: ExtensionEntry = serde_yaml::from_value(memory_value.clone()).unwrap();

        assert!(memory_entry.enabled);

        if let ExtensionConfig::Builtin {
            display_name,
            description,
            ..
        } = &memory_entry.config
        {
            assert_eq!(display_name.as_deref(), Some("Memory"));
            assert_eq!(description, "Teach goose your preferences as you go.");
        } else {
            panic!("Expected Builtin config");
        }
    }
}
