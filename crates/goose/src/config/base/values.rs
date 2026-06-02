use super::*;
use std::path::PathBuf;

// GOOSE_PROVIDER and GOOSE_MODEL are handled by crate::config::providers
// which checks the structured `providers:` block first and falls back to
// the legacy flat keys. The accessors below delegate to that module.
impl Config {
    pub fn get_goose_provider(&self) -> Result<String, ConfigError> {
        crate::config::providers::get_active_provider(self)
            .ok_or_else(|| ConfigError::NotFound("GOOSE_PROVIDER".to_string()))
    }
    pub fn set_goose_provider(&self, v: impl Into<String>) -> Result<(), ConfigError> {
        let name = v.into();
        let model = crate::config::providers::get_provider_entry(self, &name)
            .map(|e| e.model)
            .unwrap_or_default();
        crate::config::providers::set_active_provider(self, &name, &model)
    }
    pub fn get_goose_model(&self) -> Result<String, ConfigError> {
        crate::config::providers::get_active_model(self)
            .ok_or_else(|| ConfigError::NotFound("GOOSE_MODEL".to_string()))
    }
    pub fn set_goose_model(&self, v: impl Into<String>) -> Result<(), ConfigError> {
        let model = v.into();
        if let Some(provider) = crate::config::providers::get_active_provider(self) {
            crate::config::providers::set_active_provider(self, &provider, &model)?;
        }
        Ok(())
    }
}

fn find_workspace_or_exe_root() -> Option<PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let exe_dir = exe.parent()?.to_path_buf();

    let mut path = exe;
    while let Some(parent) = path.parent() {
        let cargo_toml = parent.join("Cargo.toml");
        if cargo_toml.exists() {
            if let Ok(content) = std::fs::read_to_string(&cargo_toml) {
                if content.contains("[workspace]") {
                    return Some(parent.to_path_buf());
                }
            }
        }
        path = parent.to_path_buf();
    }

    Some(exe_dir)
}

pub fn load_init_config_from_workspace() -> Result<Mapping, ConfigError> {
    let root = find_workspace_or_exe_root().ok_or_else(|| {
        ConfigError::FileError(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Could not determine executable path",
        ))
    })?;

    let init_config_path = root.join("init-config.yaml");
    if !init_config_path.exists() {
        return Err(ConfigError::NotFound(
            "init-config.yaml not found".to_string(),
        ));
    }

    let init_content = std::fs::read_to_string(&init_config_path)?;
    parse_yaml_content(&init_content)
}
