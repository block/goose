use ignore::gitignore::Gitignore;
use std::path::{Path, PathBuf};

use crate::config::paths::Paths;
pub use goose_agent::prompt::build_gitignore;

pub const GOOSE_HINTS_FILENAME: &str = ".goosehints";
pub const AGENTS_MD_FILENAME: &str = "AGENTS.md";

pub fn get_context_filenames() -> Vec<String> {
    use crate::config::Config;

    Config::global()
        .get_param::<Vec<String>>("CONTEXT_FILE_NAMES")
        .unwrap_or_else(|_| {
            vec![
                GOOSE_HINTS_FILENAME.to_string(),
                AGENTS_MD_FILENAME.to_string(),
            ]
        })
}

pub fn load_hint_files(
    cwd: &Path,
    hints_filenames: &[String],
    ignore_patterns: &Gitignore,
) -> String {
    let mut global_hints_paths: Vec<PathBuf> = hints_filenames
        .iter()
        .map(|name| Paths::in_config_dir(name))
        .collect();
    if hints_filenames
        .iter()
        .any(|name| name == AGENTS_MD_FILENAME)
    {
        global_hints_paths.push(Paths::in_agents_home_dir(AGENTS_MD_FILENAME));
    }

    goose_agent::prompt::load_hint_files(cwd, hints_filenames, &global_hints_paths, ignore_patterns)
}

#[cfg(test)]
mod tests {
    use super::*;
    use ignore::gitignore::GitignoreBuilder;
    use std::fs;
    use tempfile::TempDir;

    fn create_dummy_gitignore() -> Gitignore {
        let temp_dir = tempfile::tempdir().expect("failed to create tempdir");
        let builder = GitignoreBuilder::new(temp_dir.path());
        builder.build().expect("failed to build gitignore")
    }

    #[test]
    #[serial_test::serial]
    fn test_global_agents_md_in_agents_home() {
        let root = TempDir::new().unwrap();
        std::env::set_var("GOOSE_PATH_ROOT", root.path());

        let agents_home = root.path().join(".agents");
        fs::create_dir_all(&agents_home).unwrap();
        fs::write(
            agents_home.join(AGENTS_MD_FILENAME),
            "Global agents home instructions",
        )
        .unwrap();

        let project = TempDir::new().unwrap();
        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(
            project.path(),
            &[
                GOOSE_HINTS_FILENAME.to_string(),
                AGENTS_MD_FILENAME.to_string(),
            ],
            &gitignore,
        );

        std::env::remove_var("GOOSE_PATH_ROOT");

        assert!(hints.contains("Global Hints"));
        assert!(hints.contains("Global agents home instructions"));
    }

    #[test]
    #[serial_test::serial]
    fn test_global_agents_md_imports_not_filtered_by_project_gitignore() {
        let root = TempDir::new().unwrap();
        std::env::set_var("GOOSE_PATH_ROOT", root.path());

        let agents_home = root.path().join(".agents");
        fs::create_dir_all(&agents_home).unwrap();
        fs::write(agents_home.join("policy.md"), "Imported policy content").unwrap();
        fs::write(
            agents_home.join(AGENTS_MD_FILENAME),
            "Global header\n@policy.md\n",
        )
        .unwrap();

        let project = TempDir::new().unwrap();
        let mut builder = GitignoreBuilder::new(project.path());
        builder.add_line(None, "*.md").unwrap();
        let gitignore = builder.build().unwrap();

        let hints = load_hint_files(
            project.path(),
            &[
                GOOSE_HINTS_FILENAME.to_string(),
                AGENTS_MD_FILENAME.to_string(),
            ],
            &gitignore,
        );

        std::env::remove_var("GOOSE_PATH_ROOT");

        assert!(hints.contains("Imported policy content"));
    }

    #[test]
    #[serial_test::serial]
    fn test_global_agents_md_skipped_when_not_in_context_file_names() {
        let root = TempDir::new().unwrap();
        std::env::set_var("GOOSE_PATH_ROOT", root.path());

        let agents_home = root.path().join(".agents");
        fs::create_dir_all(&agents_home).unwrap();
        fs::write(
            agents_home.join(AGENTS_MD_FILENAME),
            "Global agents home instructions",
        )
        .unwrap();

        let project = TempDir::new().unwrap();
        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(
            project.path(),
            &[GOOSE_HINTS_FILENAME.to_string()],
            &gitignore,
        );

        std::env::remove_var("GOOSE_PATH_ROOT");

        assert!(!hints.contains("Global agents home instructions"));
    }
}
