use etcetera::{choose_app_strategy, AppStrategy};
use std::path::{Path, PathBuf};

pub fn load_and_format_hints(cwd: &Path, hints_filenames: &[String]) -> String {
    let mut global_hints_contents = Vec::with_capacity(hints_filenames.len());
    let mut local_hints_contents = Vec::with_capacity(hints_filenames.len());

    for hints_filename in hints_filenames {
        // Global hints
        // choose_app_strategy().config_dir()
        // - macOS/Linux: ~/.config/goose/
        // - Windows:     ~\AppData\Roaming\Block\goose\config\
        // keep previous behavior of expanding ~/.config in case this fails
        let global_hints_path = choose_app_strategy(crate::APP_STRATEGY.clone())
            .map(|strategy| strategy.in_config_dir(hints_filename))
            .unwrap_or_else(|_| {
                let path_str = format!("~/.config/goose/{}", hints_filename);
                PathBuf::from(shellexpand::tilde(&path_str).to_string())
            });

        if let Some(parent) = global_hints_path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        if global_hints_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&global_hints_path) {
                global_hints_contents.push(content);
            }
        }

        let local_hints_path = cwd.join(hints_filename);
        if local_hints_path.is_file() {
            if let Ok(content) = std::fs::read_to_string(&local_hints_path) {
                local_hints_contents.push(content);
            }
        }
    }

    let mut hints = String::new();
    if !global_hints_contents.is_empty() {
        hints.push_str("\n### Global Hints\nThe developer extension includes some global hints that apply to all projects & directories.\n");
        hints.push_str(&global_hints_contents.join("\n"));
    }

    if !local_hints_contents.is_empty() {
        if !hints.is_empty() {
            hints.push_str("\n\n");
        }
        hints.push_str("### Project Hints\nThe developer extension includes some hints for working on the project in this directory.\n");
        hints.push_str(&local_hints_contents.join("\n"));
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

    #[test]
    #[serial]
    fn test_global_goosehints() {
        // if ~/.config/goose/.goosehints exists, it should be included in the instructions
        // copy the existing global hints file to a .bak file
        let global_hints_path =
            PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints").to_string());
        let global_hints_bak_path =
            PathBuf::from(shellexpand::tilde("~/.config/goose/.goosehints.bak").to_string());
        let mut globalhints_existed = false;

        if global_hints_path.is_file() {
            globalhints_existed = true;
            fs::copy(&global_hints_path, &global_hints_bak_path).unwrap();
        }

        fs::write(&global_hints_path, "These are my global goose hints.").unwrap();

        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let hints = load_and_format_hints(dir.path(), &[".goosehints".to_string()]);

        assert!(hints.contains("### Global Hints"));
        assert!(hints.contains("my global goose hints."));

        // restore backup if globalhints previously existed
        if globalhints_existed {
            fs::copy(&global_hints_bak_path, &global_hints_path).unwrap();
            fs::remove_file(&global_hints_bak_path).unwrap();
        } else {
            // Clean up the test file we created
            let _ = fs::remove_file(&global_hints_path);
        }
    }

    #[test]
    #[serial]
    fn test_goosehints_when_present() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(dir.path().join(".goosehints"), "Test hint content").unwrap();
        let hints = load_and_format_hints(dir.path(), &[".goosehints".to_string()]);

        assert!(hints.contains("Test hint content"));
    }

    #[test]
    #[serial]
    fn test_goosehints_when_missing() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let hints = load_and_format_hints(dir.path(), &[".goosehints".to_string()]);

        assert!(!hints.contains("Project Hints"));
    }

    #[test]
    #[serial]
    fn test_goosehints_multiple_filenames() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "Custom hints file content from CLAUDE.md").unwrap();
        fs::write(dir.path().join(".goosehints"), "Custom hints file content from .goosehints").unwrap();
        
        let hints = load_and_format_hints(dir.path(), &["CLAUDE.md".to_string(), ".goosehints".to_string()]);

        assert!(hints.contains("Custom hints file content from CLAUDE.md"));
        assert!(hints.contains("Custom hints file content from .goosehints"));
    }

    #[test]
    #[serial]
    fn test_goosehints_configurable_filename() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "Custom hints file content").unwrap();
        let hints = load_and_format_hints(dir.path(), &["CLAUDE.md".to_string()]);

        assert!(hints.contains("Custom hints file content"));
        assert!(!hints.contains(".goosehints")); // Make sure it's not loading the default
    }
}