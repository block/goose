use crate::config::paths::Paths;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::PathBuf;
use tracing::warn;

use super::app::GooseApp;

pub struct McpAppCache {
    cache_dir: PathBuf,
}

impl McpAppCache {
    pub fn new() -> Result<Self, std::io::Error> {
        let config_dir = Paths::config_dir();
        let cache_dir = config_dir.join("mcp-apps-cache");
        Ok(Self { cache_dir })
    }

    fn cache_key(extension_name: &str, resource_uri: &str) -> String {
        let input = format!("{}::{}", extension_name, resource_uri);
        let hash = Sha256::digest(input.as_bytes());
        format!("{}_{:x}", extension_name, hash)
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

        if let Some(ref extension_name) = app.mcp_server {
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
                        if app.mcp_server.as_deref() == Some(extension_name)
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
