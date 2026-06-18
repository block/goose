use std::fs;
use std::path::Path;

use super::app::GooseApp;

const IOC_TOOLBOX_HTML: &str = include_str!("ioc-toolbox.html");
const ENCODE_HASH_LAB_HTML: &str = include_str!("encode-hash-lab.html");
const SECRET_CREDENTIAL_SCANNER_HTML: &str = include_str!("secret-credential-scanner.html");
const JWT_INSPECTOR_HTML: &str = include_str!("jwt-inspector.html");

pub const BUILTIN_SECURITY_APP_NAMES: &[&str] = &[
    "ioc-toolbox",
    "encode-hash-lab",
    "secret-credential-scanner",
    "jwt-inspector",
];
pub const LEGACY_DEFAULT_APP_NAMES: &[&str] = &["chat", "clock"];
pub const RETIRED_CURATED_APP_NAMES: &[&str] = &["header-diff-lab"];

pub fn parse_default_apps() -> Result<Vec<GooseApp>, String> {
    [
        IOC_TOOLBOX_HTML,
        ENCODE_HASH_LAB_HTML,
        SECRET_CREDENTIAL_SCANNER_HTML,
        JWT_INSPECTOR_HTML,
    ]
    .into_iter()
    .map(GooseApp::from_html)
    .collect()
}

pub fn sync_default_apps_dir(apps_dir: &Path) -> Result<(), String> {
    fs::create_dir_all(apps_dir).map_err(|e| format!("Failed to create apps directory: {}", e))?;

    for removed_name in LEGACY_DEFAULT_APP_NAMES
        .iter()
        .chain(RETIRED_CURATED_APP_NAMES.iter())
    {
        let removed_path = apps_dir.join(format!("{}.html", removed_name));
        if removed_path.exists() {
            fs::remove_file(&removed_path)
                .map_err(|e| format!("Failed to remove retired app '{}': {}", removed_name, e))?;
        }
    }

    for app in parse_default_apps()? {
        let app_path = apps_dir.join(format!("{}.html", app.resource.name));
        let html = app.to_html()?;
        fs::write(&app_path, html)
            .map_err(|e| format!("Failed to write default app '{}': {}", app.resource.name, e))?;
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{parse_default_apps, sync_default_apps_dir};
    use std::fs;
    use tempfile::TempDir;

    #[test]
    fn default_apps_parse_expected_curated_names() {
        let mut names = parse_default_apps()
            .unwrap()
            .into_iter()
            .map(|app| app.resource.name)
            .collect::<Vec<_>>();
        names.sort();

        assert_eq!(
            names,
            vec![
                "encode-hash-lab".to_string(),
                "ioc-toolbox".to_string(),
                "jwt-inspector".to_string(),
                "secret-credential-scanner".to_string(),
            ]
        );
    }

    #[test]
    fn sync_default_apps_dir_replaces_legacy_defaults() {
        let temp_dir = TempDir::new().unwrap();
        let apps_dir = temp_dir.path();

        fs::write(apps_dir.join("clock.html"), "<html>clock</html>").unwrap();
        fs::write(apps_dir.join("chat.html"), "<html>chat</html>").unwrap();
        fs::write(
            apps_dir.join("header-diff-lab.html"),
            "<html>retired curated app</html>",
        )
        .unwrap();

        sync_default_apps_dir(apps_dir).unwrap();

        assert!(!apps_dir.join("clock.html").exists());
        assert!(!apps_dir.join("chat.html").exists());
        assert!(!apps_dir.join("header-diff-lab.html").exists());
        assert!(apps_dir.join("ioc-toolbox.html").exists());
        assert!(apps_dir.join("encode-hash-lab.html").exists());
        assert!(apps_dir.join("jwt-inspector.html").exists());
        assert!(apps_dir.join("secret-credential-scanner.html").exists());
    }

    #[test]
    fn sync_default_apps_dir_refreshes_curated_app_html_when_it_drifted() {
        let temp_dir = TempDir::new().unwrap();
        let apps_dir = temp_dir.path();

        fs::write(
            apps_dir.join("ioc-toolbox.html"),
            "<html>stale ioc toolbox</html>",
        )
        .unwrap();

        sync_default_apps_dir(apps_dir).unwrap();

        let refreshed = fs::read_to_string(apps_dir.join("ioc-toolbox.html")).unwrap();
        assert!(refreshed.contains("<title>IOC Toolbox · IOC 工具箱</title>"));
        assert!(!refreshed.contains("stale ioc toolbox"));
    }
}
