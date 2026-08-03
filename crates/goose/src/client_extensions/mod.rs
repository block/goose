use crate::utils::copy_dir_all;
use anyhow::{anyhow, bail, Context, Result};
use fs_err as fs;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const MANIFEST_FILENAME: &str = "client-extension.json";
const CONFIG_FILENAME: &str = "config.json";

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientExtensionSource {
    Installed,
    Dev,
}

impl std::fmt::Display for ClientExtensionSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientExtensionSource::Installed => write!(f, "installed"),
            ClientExtensionSource::Dev => write!(f, "dev"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ClientExtensionSummary {
    pub id: String,
    pub version: String,
    pub directory: PathBuf,
    pub source: ClientExtensionSource,
    pub enabled: bool,
}

#[derive(Debug, Clone)]
pub struct ClientExtensionInstall {
    pub id: String,
    pub version: String,
    pub directory: PathBuf,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ClientExtensionsConfig {
    #[serde(default)]
    pub disabled: Vec<String>,
    #[serde(default, rename = "enabledDev")]
    pub enabled_dev: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct ClientExtensionManifest {
    id: String,
    version: String,
    main: String,
}

pub fn client_extensions_dir() -> PathBuf {
    dirs::home_dir()
        .map(|home| home.join(".agents").join("client-extensions"))
        .unwrap_or_else(|| PathBuf::from(".agents/client-extensions"))
}

fn config_path() -> PathBuf {
    client_extensions_dir().join(CONFIG_FILENAME)
}

pub fn load_client_extensions_config() -> ClientExtensionsConfig {
    let path = config_path();
    match fs::read_to_string(&path) {
        Ok(contents) => serde_json::from_str(&contents).unwrap_or_default(),
        Err(_) => ClientExtensionsConfig::default(),
    }
}

pub fn save_client_extensions_config(config: &ClientExtensionsConfig) -> Result<()> {
    let dir = client_extensions_dir();
    fs::create_dir_all(&dir)?;
    let mut disabled = config.disabled.clone();
    disabled.sort();
    disabled.dedup();
    let mut enabled_dev = config.enabled_dev.clone();
    enabled_dev.sort();
    enabled_dev.dedup();
    let normalized = ClientExtensionsConfig {
        disabled,
        enabled_dev,
    };
    fs::write(config_path(), serde_json::to_string_pretty(&normalized)?)?;
    Ok(())
}

pub fn is_client_extension_enabled(
    id: &str,
    source: ClientExtensionSource,
    config: &ClientExtensionsConfig,
) -> bool {
    if config.disabled.iter().any(|entry| entry == id) {
        return false;
    }
    match source {
        ClientExtensionSource::Installed => true,
        ClientExtensionSource::Dev => config.enabled_dev.iter().any(|entry| entry == id),
    }
}

fn set_client_extension_enabled(id: &str, enabled: bool) -> Result<ClientExtensionsConfig> {
    let summaries = list_client_extensions()?;
    let summary = summaries
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| anyhow!("client extension '{id}' is not installed"))?;

    let mut config = load_client_extensions_config();
    config.disabled.retain(|entry| entry != id);

    match summary.source {
        ClientExtensionSource::Installed => {
            if !enabled {
                config.disabled.push(id.to_string());
            }
        }
        ClientExtensionSource::Dev => {
            config.enabled_dev.retain(|entry| entry != id);
            if enabled {
                config.enabled_dev.push(id.to_string());
            } else {
                config.disabled.push(id.to_string());
            }
        }
    }

    save_client_extensions_config(&config)?;
    Ok(config)
}

pub fn enable_client_extension(id: &str) -> Result<()> {
    set_client_extension_enabled(id, true)?;
    Ok(())
}

pub fn disable_client_extension(id: &str) -> Result<()> {
    set_client_extension_enabled(id, false)?;
    Ok(())
}

pub fn uninstall_client_extension(id: &str) -> Result<()> {
    let summaries = list_client_extensions()?;
    let summary = summaries
        .iter()
        .find(|entry| entry.id == id)
        .ok_or_else(|| anyhow!("client extension '{id}' is not installed"))?;

    if summary.source != ClientExtensionSource::Installed {
        bail!("cannot uninstall dev client extension '{id}' — disable it from Add-ons instead");
    }

    if summary.directory.is_dir() {
        fs::remove_dir_all(&summary.directory)?;
    }

    let mut config = load_client_extensions_config();
    config.disabled.retain(|entry| entry != id);
    config.enabled_dev.retain(|entry| entry != id);
    save_client_extensions_config(&config)?;

    Ok(())
}

pub fn install_client_extension(source: &Path) -> Result<ClientExtensionInstall> {
    let source = source
        .canonicalize()
        .with_context(|| format!("client extension source not found: {}", source.display()))?;
    if !source.is_dir() {
        bail!("client extension source must be a directory");
    }

    let manifest = read_manifest(&source)?;
    validate_manifest_files(&source, &manifest)?;

    let destination = client_extensions_dir().join(&manifest.id);
    if destination.exists() {
        bail!(
            "client extension '{}' is already installed at {}",
            manifest.id,
            destination.display()
        );
    }

    fs::create_dir_all(client_extensions_dir())?;
    copy_dir_all(&source, &destination)?;

    Ok(ClientExtensionInstall {
        id: manifest.id,
        version: manifest.version,
        directory: destination,
    })
}

pub fn list_client_extensions() -> Result<Vec<ClientExtensionSummary>> {
    let config = load_client_extensions_config();
    let mut by_id = std::collections::BTreeMap::new();

    let install_root = client_extensions_dir();
    if install_root.is_dir() {
        collect_extensions_in_root(&install_root, ClientExtensionSource::Installed, &mut by_id)?;
    }

    if let Some(dev_root) = dev_client_extensions_dir() {
        let mut dev_by_id = std::collections::BTreeMap::new();
        collect_extensions_in_root(&dev_root, ClientExtensionSource::Dev, &mut dev_by_id)?;
        for (id, summary) in dev_by_id {
            by_id.entry(id).or_insert(summary);
        }
    }

    Ok(by_id
        .into_values()
        .map(|mut summary| {
            summary.enabled =
                is_client_extension_enabled(&summary.id, summary.source.clone(), &config);
            summary
        })
        .collect())
}

fn dev_client_extensions_dir() -> Option<PathBuf> {
    let cwd = std::env::current_dir().ok()?;
    [
        cwd.join("examples").join("client-extensions"),
        cwd.join("..")
            .join("..")
            .join("examples")
            .join("client-extensions"),
    ]
    .into_iter()
    .find(|candidate| candidate.is_dir())
}

fn collect_extensions_in_root(
    root: &Path,
    source: ClientExtensionSource,
    by_id: &mut std::collections::BTreeMap<String, ClientExtensionSummary>,
) -> Result<()> {
    for entry in fs::read_dir(root)? {
        let entry = entry?;
        if entry.file_name() == CONFIG_FILENAME {
            continue;
        }
        if !entry.file_type()?.is_dir() {
            continue;
        }
        let directory = entry.path();
        let manifest = match read_manifest(&directory) {
            Ok(manifest) => manifest,
            Err(_) => continue,
        };
        if validate_manifest_files(&directory, &manifest).is_err() {
            continue;
        }
        by_id.insert(
            manifest.id.clone(),
            ClientExtensionSummary {
                id: manifest.id.clone(),
                version: manifest.version,
                directory,
                source: source.clone(),
                enabled: false,
            },
        );
    }
    Ok(())
}

fn read_manifest(root: &Path) -> Result<ClientExtensionManifest> {
    let manifest_path = root.join(MANIFEST_FILENAME);
    let contents = fs::read_to_string(&manifest_path)
        .with_context(|| format!("missing manifest at {}", manifest_path.display()))?;
    serde_json::from_str(&contents)
        .with_context(|| format!("invalid manifest at {}", manifest_path.display()))
}

fn validate_manifest_files(root: &Path, manifest: &ClientExtensionManifest) -> Result<()> {
    if manifest.id.trim().is_empty() {
        bail!("manifest id must not be empty");
    }
    if manifest.version.trim().is_empty() {
        bail!("manifest version must not be empty");
    }
    if manifest.main.trim().is_empty() {
        bail!("manifest main must not be empty");
    }
    let main_path = root.join(&manifest.main);
    if !main_path.is_file() {
        bail!("manifest main entry missing at {}", main_path.display());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use tempfile::TempDir;

    fn write_extension(root: &Path, id: &str) {
        fs::create_dir_all(root).unwrap();
        fs::write(
            root.join(MANIFEST_FILENAME),
            format!(r#"{{"id":"{id}","version":"0.1.0","main":"index.html"}}"#),
        )
        .unwrap();
        fs::write(root.join("index.html"), "<html></html>").unwrap();
    }

    #[test]
    #[serial]
    fn installs_and_uninstalls_extension_from_directory() {
        let source = TempDir::new().unwrap();
        write_extension(source.path(), "demo-ext");

        let install_root = TempDir::new().unwrap();
        let original_home = std::env::var("HOME").ok();
        std::env::set_var("HOME", install_root.path());

        let install = install_client_extension(source.path()).unwrap();
        assert_eq!(install.id, "demo-ext");
        assert!(install.directory.join("index.html").is_file());
        assert!(client_extensions_dir().join("demo-ext").is_dir());

        disable_client_extension("demo-ext").unwrap();
        let disabled = list_client_extensions()
            .unwrap()
            .into_iter()
            .find(|entry| entry.id == "demo-ext")
            .expect("extension listed");
        assert!(!disabled.enabled);

        enable_client_extension("demo-ext").unwrap();
        let enabled = list_client_extensions()
            .unwrap()
            .into_iter()
            .find(|entry| entry.id == "demo-ext")
            .expect("extension listed");
        assert!(enabled.enabled);

        uninstall_client_extension("demo-ext").unwrap();
        assert!(!client_extensions_dir().join("demo-ext").exists());

        if let Some(home) = original_home {
            std::env::set_var("HOME", home);
        }
    }

    #[test]
    fn dev_extensions_require_opt_in() {
        let config = ClientExtensionsConfig::default();
        assert!(!is_client_extension_enabled(
            "hello-page",
            ClientExtensionSource::Dev,
            &config
        ));
        assert!(is_client_extension_enabled(
            "hello-page",
            ClientExtensionSource::Installed,
            &config
        ));
    }
}
