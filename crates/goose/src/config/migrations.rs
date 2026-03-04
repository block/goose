use crate::agents::extension::PLATFORM_EXTENSIONS;
use crate::agents::ExtensionConfig;
use crate::config::extensions::ExtensionEntry;
use serde_yaml::Mapping;

const EXTENSIONS_CONFIG_KEY: &str = "extensions";

pub fn run_migrations(config: &mut Mapping) -> bool {
    let mut changed = false;
    changed |= migrate_platform_extensions(config);
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
                Ok(entry) => !matches!(
                    &entry.config,
                    ExtensionConfig::Platform {
                        description,
                        display_name,
                        ..
                    } if description == def.description
                        && display_name.as_deref() == Some(def.display_name)
                ),
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

    // Remove stale bundled field from any extension entries
    for (_key, value) in extensions_map.iter_mut() {
        if let serde_yaml::Value::Mapping(ref mut entry_map) = value {
            let bundled_key = serde_yaml::Value::String("bundled".to_string());
            if entry_map.remove(&bundled_key).is_some() {
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
    fn test_migrate_removes_bundled_field() {
        let mut config = Mapping::new();
        let mut extensions = Mapping::new();
        let mut entry_map = Mapping::new();
        entry_map.insert(
            serde_yaml::Value::String("type".to_string()),
            serde_yaml::Value::String("stdio".to_string()),
        );
        entry_map.insert(
            serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String("my-ext".to_string()),
        );
        entry_map.insert(
            serde_yaml::Value::String("description".to_string()),
            serde_yaml::Value::String("test".to_string()),
        );
        entry_map.insert(
            serde_yaml::Value::String("cmd".to_string()),
            serde_yaml::Value::String("echo".to_string()),
        );
        entry_map.insert(
            serde_yaml::Value::String("args".to_string()),
            serde_yaml::Value::Sequence(vec![]),
        );
        entry_map.insert(
            serde_yaml::Value::String("enabled".to_string()),
            serde_yaml::Value::Bool(true),
        );
        entry_map.insert(
            serde_yaml::Value::String("bundled".to_string()),
            serde_yaml::Value::Bool(true),
        );
        extensions.insert(
            serde_yaml::Value::String("my-ext".to_string()),
            serde_yaml::Value::Mapping(entry_map),
        );
        config.insert(
            serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string()),
            serde_yaml::Value::Mapping(extensions),
        );

        let changed = run_migrations(&mut config);
        assert!(changed);

        let ext_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());
        let exts = config.get(ext_key).unwrap().as_mapping().unwrap();
        let my_ext = exts
            .get(serde_yaml::Value::String("my-ext".to_string()))
            .unwrap()
            .as_mapping()
            .unwrap();
        assert!(!my_ext.contains_key(serde_yaml::Value::String("bundled".to_string())));
    }

    #[test]
    fn test_migrate_builtin_to_platform() {
        let mut config = Mapping::new();
        let mut extensions = Mapping::new();
        let mut entry_map = Mapping::new();
        entry_map.insert(
            serde_yaml::Value::String("type".to_string()),
            serde_yaml::Value::String("builtin".to_string()),
        );
        entry_map.insert(
            serde_yaml::Value::String("name".to_string()),
            serde_yaml::Value::String("computercontroller".to_string()),
        );
        entry_map.insert(
            serde_yaml::Value::String("description".to_string()),
            serde_yaml::Value::String("old desc".to_string()),
        );
        entry_map.insert(
            serde_yaml::Value::String("enabled".to_string()),
            serde_yaml::Value::Bool(true),
        );
        entry_map.insert(
            serde_yaml::Value::String("bundled".to_string()),
            serde_yaml::Value::Bool(true),
        );
        extensions.insert(
            serde_yaml::Value::String("computercontroller".to_string()),
            serde_yaml::Value::Mapping(entry_map),
        );
        config.insert(
            serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string()),
            serde_yaml::Value::Mapping(extensions),
        );

        let changed = run_migrations(&mut config);
        assert!(changed);

        let ext_key = serde_yaml::Value::String(EXTENSIONS_CONFIG_KEY.to_string());
        let exts = config.get(ext_key).unwrap().as_mapping().unwrap();
        let cc = exts
            .get(serde_yaml::Value::String("computercontroller".to_string()))
            .unwrap();
        let entry: ExtensionEntry = serde_yaml::from_value(cc.clone()).unwrap();
        assert!(matches!(entry.config, ExtensionConfig::Platform { .. }));
        assert!(entry.enabled);
    }
}
