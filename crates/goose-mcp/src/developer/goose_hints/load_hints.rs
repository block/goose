use etcetera::{choose_app_strategy, AppStrategy};
use ignore::gitignore::Gitignore;
use std::{collections::HashSet, path::{Path, PathBuf}};

use crate::developer::goose_hints::import_files::read_referenced_files;

pub const GOOSE_HINTS_FILENAME: &str = ".goosehints";


fn traverse_directories_upward(start_dir: &Path) -> Vec<PathBuf> {
    let mut directories = Vec::new();
    let mut current_dir = start_dir;

    loop {
        directories.push(current_dir.to_path_buf());
        if current_dir.join(".git").exists() {
            break;
        }
        if let Some(parent) = current_dir.parent() {
            current_dir = parent;
        } else {
            break;
        }
    }
    directories.reverse();
    directories
}

fn is_nested_enabled() -> bool {
    std::env::var("NESTED_GOOSE_HINTS")
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(false)
}


pub fn load_hint_files(cwd: &Path, hints_filenames: &[String], ignore_patterns: &Gitignore) -> String {
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
            let mut visited = HashSet::new();
            let hints_dir = global_hints_path.parent().unwrap();
            let expanded_content = read_referenced_files(
                &global_hints_path,
                hints_dir,
                &mut visited,
                0,
                &ignore_patterns,
            );
            if !expanded_content.is_empty() {
                global_hints_contents.push(expanded_content);
            }
        }
    }

    let local_directories = if is_nested_enabled() {
        traverse_directories_upward(cwd)
    } else {
        vec![cwd.to_path_buf()]
    };

    for directory in &local_directories {
        for hints_filename in hints_filenames {
            let hints_path = directory.join(hints_filename);
            if hints_path.is_file() {
                let mut visited = HashSet::new();
                let expanded_content = read_referenced_files(
                    &hints_path,
                    cwd,
                    &mut visited,
                    0,
                    &ignore_patterns,
                );
                if !expanded_content.is_empty() {
                    local_hints_contents.push(expanded_content);
                }
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
    use ignore::gitignore::GitignoreBuilder;
    use serial_test::serial;
    use std::fs::{self};
    use tempfile::TempDir;

    fn create_dummy_gitignore() -> Gitignore {
        let temp_dir = tempfile::tempdir().expect("failed to create tempdir");
        let builder = GitignoreBuilder::new(temp_dir.path());
        builder.build().expect("failed to build gitignore")
    }

    #[test]
    #[serial]
    fn test_global_goosehints() {
        // if ~/.config/goose/.goosehints exists, it should be included in the instructions
        // copy the existing global hints file to a .bak file
        let global_hints_path = PathBuf::from(
            shellexpand::tilde(format!("~/.config/goose/{}", GOOSE_HINTS_FILENAME).as_str())
                .to_string(),
        );
        let global_hints_bak_path = PathBuf::from(
            shellexpand::tilde(format!("~/.config/goose/{}.bak", GOOSE_HINTS_FILENAME).as_str())
                .to_string(),
        );
        let mut globalhints_existed = false;

        if global_hints_path.is_file() {
            globalhints_existed = true;
            fs::copy(&global_hints_path, &global_hints_bak_path).unwrap();
        }

        fs::write(&global_hints_path, "These are my global goose hints.").unwrap();

        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(dir.path(), &[GOOSE_HINTS_FILENAME.to_string()], &gitignore);

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

        fs::write(dir.path().join(GOOSE_HINTS_FILENAME), "Test hint content").unwrap();
        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(dir.path(), &[GOOSE_HINTS_FILENAME.to_string()], &gitignore);

        assert!(hints.contains("Test hint content"));
    }

    #[test]
    #[serial]
    fn test_goosehints_when_missing() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(dir.path(), &[GOOSE_HINTS_FILENAME.to_string()], &gitignore);

        assert!(!hints.contains("Project Hints"));
    }

    #[test]
    #[serial]
    fn test_goosehints_multiple_filenames() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(
            dir.path().join("CLAUDE.md"),
            "Custom hints file content from CLAUDE.md",
        )
        .unwrap();
        fs::write(
            dir.path().join(GOOSE_HINTS_FILENAME),
            "Custom hints file content from .goosehints",
        )
        .unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(
            dir.path(),
            &["CLAUDE.md".to_string(), GOOSE_HINTS_FILENAME.to_string()],
            &gitignore,
        );

        assert!(hints.contains("Custom hints file content from CLAUDE.md"));
        assert!(hints.contains("Custom hints file content from .goosehints"));
    }

    #[test]
    #[serial]
    fn test_goosehints_configurable_filename() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "Custom hints file content").unwrap();
        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(dir.path(), &["CLAUDE.md".to_string()], &gitignore);

        assert!(hints.contains("Custom hints file content"));
        assert!(!hints.contains(".goosehints")); // Make sure it's not loading the default
    }

    #[test]
    #[serial]
    fn test_nested_goosehints_with_git_root() {
        std::env::set_var("NESTED_GOOSE_HINTS", "true");

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        fs::create_dir(project_root.join(".git")).unwrap();
        fs::write(
            project_root.join(GOOSE_HINTS_FILENAME),
            "Root hints content",
        )
        .unwrap();

        let subdir = project_root.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join(GOOSE_HINTS_FILENAME), "Subdir hints content").unwrap();
        let current_dir = subdir.join("current_dir");
        fs::create_dir(&current_dir).unwrap();
        fs::write(
            current_dir.join(GOOSE_HINTS_FILENAME),
            "current_dir hints content",
        )
        .unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(&current_dir, &[GOOSE_HINTS_FILENAME.to_string()], &gitignore);

        assert!(
            hints.contains("Root hints content\nSubdir hints content\ncurrent_dir hints content")
        );

        std::env::remove_var("NESTED_GOOSE_HINTS");
    }

    #[test]
    #[serial]
    fn test_nested_goosehints_without_git_root() {
        std::env::set_var("NESTED_GOOSE_HINTS", "true");

        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        fs::write(base_dir.join(GOOSE_HINTS_FILENAME), "Base hints content").unwrap();

        let subdir = base_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join(GOOSE_HINTS_FILENAME), "Subdir hints content").unwrap();

        let current_dir = subdir.join("current_dir");
        fs::create_dir(&current_dir).unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(&current_dir, &[GOOSE_HINTS_FILENAME.to_string()], &gitignore);

        assert!(hints.contains("Base hints content"));
        assert!(hints.contains("Subdir hints content"));

        std::env::remove_var("NESTED_GOOSE_HINTS");
    }

    #[test]
    #[serial]
    fn test_nested_goosehints_mixed_filenames() {
        std::env::set_var("NESTED_GOOSE_HINTS", "true");

        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        fs::create_dir(project_root.join(".git")).unwrap();
        fs::write(project_root.join("CLAUDE.md"), "Root CLAUDE.md content").unwrap();

        let subdir = project_root.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(
            subdir.join(GOOSE_HINTS_FILENAME),
            "Subdir .goosehints content",
        )
        .unwrap();

        let current_dir = subdir.join("current_dir");
        fs::create_dir(&current_dir).unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(
            &current_dir,
            &["CLAUDE.md".to_string(), GOOSE_HINTS_FILENAME.to_string()],
            &gitignore,
        );

        assert!(hints.contains("Root CLAUDE.md content"));
        assert!(hints.contains("Subdir .goosehints content"));

        std::env::remove_var("NESTED_GOOSE_HINTS");
    }
}
