use crate::config::paths::Paths;
use crate::goose_apps::GooseApp;
use anyhow::Result;
use std::fs;
use std::path::PathBuf;

pub struct GooseAppsManager {
    apps_dir: PathBuf,
}

impl GooseAppsManager {
    pub fn new() -> Result<Self> {
        let config_dir = Paths::config_dir();

        let apps_dir = config_dir.join("apps");

        Ok(Self { apps_dir })
    }

    pub fn list_apps(&self) -> Result<Vec<GooseApp>> {
        let mut apps = Vec::new();

        if !self.apps_dir.exists() {
            return Ok(apps);
        }

        for entry in fs::read_dir(&self.apps_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.extension().and_then(|s| s.to_str()) == Some("gapp") {
                match GooseApp::from_file(&path) {
                    Ok(app) => apps.push(app),
                    Err(e) => eprintln!("Failed to load app from {:?}: {}", path, e),
                }
            }
        }

        Ok(apps)
    }

    pub fn get_app(&self, name: &str) -> Result<Option<GooseApp>> {
        let app_path = self.apps_dir.join(format!("{}.gapp", name));

        if !app_path.exists() {
            return Ok(None);
        }

        Ok(Some(GooseApp::from_file(app_path)?))
    }

    pub fn update_app(&self, app: &GooseApp) -> Result<()> {
        if !app.js_implementation.contains("extends GooseWidget") {
            return Err(anyhow::anyhow!(
                "Implementation must contain a class extending GooseWidget"
            ));
        }

        fs::create_dir_all(&self.apps_dir)?;
        let app_path = self.apps_dir.join(format!("{}.gapp", app.name));

        let file_content = app.to_file_content()?;
        fs::write(app_path, file_content)?;

        Ok(())
    }

    pub fn delete_app(&self, name: &str) -> Result<()> {
        let app_path = self.apps_dir.join(format!("{}.gapp", name));

        if !app_path.exists() {
            return Err(anyhow::anyhow!("App '{}' not found", name));
        }

        fs::remove_file(app_path)?;
        Ok(())
    }

    pub fn app_exists(&self, name: &str) -> bool {
        self.apps_dir.join(format!("{}.gapp", name)).exists()
    }
}
