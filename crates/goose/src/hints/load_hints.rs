use ignore::gitignore::Gitignore;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::config::paths::Paths;
use crate::hints::import_files::read_referenced_files;

pub const GOOSE_HINTS_FILENAME: &str = ".goosehints";
pub const AGENTS_MD_FILENAME: &str = "AGENTS.md";

fn find_git_root(start_dir: &Path) -> Option<&Path> {
    let mut check_dir = start_dir;

    loop {
        if check_dir.join(".git").exists() {
            return Some(check_dir);
        }
        if let Some(parent) = check_dir.parent() {
            check_dir = parent;
        } else {
            break;
        }
    }

    None
}

fn get_local_directories(git_root: Option<&Path>, cwd: &Path) -> Vec<PathBuf> {
    match git_root {
        Some(git_root) => {
            let mut directories = Vec::new();
            let mut current_dir = cwd;

            loop {
                directories.push(current_dir.to_path_buf());
                if current_dir == git_root {
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
        None => vec![cwd.to_path_buf()],
    }
}

pub fn load_hint_files(
    cwd: &Path,
    hints_filenames: &[String],
    ignore_patterns: &Gitignore,
) -> String {
    let mut global_hints_contents = Vec::with_capacity(hints_filenames.len());
    let mut local_hints_contents = Vec::with_capacity(hints_filenames.len());

    for hints_filename in hints_filenames {
        let global_hints_path = Paths::in_config_dir(hints_filename);
        if global_hints_path.is_file() {
            let mut visited = HashSet::new();
            let hints_dir = global_hints_path.parent().unwrap();
            let expanded_content = read_referenced_files(
                &global_hints_path,
                hints_dir,
                &mut visited,
                0,
                ignore_patterns,
            );
            if !expanded_content.is_empty() {
                global_hints_contents.push(expanded_content);
            }
        }
    }
    let git_root = find_git_root(cwd);
    let local_directories = get_local_directories(git_root, cwd);

    let import_boundary = git_root.unwrap_or(cwd);

    for directory in &local_directories {
        for hints_filename in hints_filenames {
            let hints_path = directory.join(hints_filename);
            if hints_path.is_file() {
                let mut visited = HashSet::new();
                let expanded_content = read_referenced_files(
                    &hints_path,
                    import_boundary,
                    &mut visited,
                    0,
                    ignore_patterns,
                );
                if !expanded_content.is_empty() {
                    local_hints_contents.push(expanded_content);
                }
            }
        }
    }

    let mut hints = String::new();
    if !global_hints_contents.is_empty() {
        hints.push_str("\n### Global Hints\nThese are my global goose hints.\n");
        hints.push_str(&global_hints_contents.join("\n"));
    }

    if !local_hints_contents.is_empty() {
        if !hints.is_empty() {
            hints.push_str("\n\n");
        }
        hints.push_str(
            "### Project Hints\nThese are hints for working on the project in this directory.\n",
        );
        hints.push_str(&local_hints_contents.join("\n"));
    }

    hints
}

#[cfg(test)]
mod tests {
    use super::*;
    use ignore::gitignore::GitignoreBuilder;
    use std::fs::{self};
    use tempfile::TempDir;

    fn create_dummy_gitignore() -> Gitignore {
        let temp_dir = tempfile::tempdir().expect("failed to create tempdir");
        let builder = GitignoreBuilder::new(temp_dir.path());
        builder.build().expect("failed to build gitignore")
    }

    #[test]
    fn test_goosehints_when_present() {
        let dir = TempDir::new().unwrap();

        fs::write(dir.path().join(GOOSE_HINTS_FILENAME), "Test hint content").unwrap();
        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(dir.path(), &[GOOSE_HINTS_FILENAME.to_string()], &gitignore);

        assert!(hints.contains("Test hint content"));
    }

    #[test]
    fn test_goosehints_when_missing() {
        let dir = TempDir::new().unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(dir.path(), &[GOOSE_HINTS_FILENAME.to_string()], &gitignore);

        assert!(!hints.contains("Project Hints"));
    }

    #[test]
    fn test_goosehints_multiple_filenames() {
        let dir = TempDir::new().unwrap();

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
    fn test_goosehints_configurable_filename() {
        let dir = TempDir::new().unwrap();

        fs::write(dir.path().join("CLAUDE.md"), "Custom hints file content").unwrap();
        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(dir.path(), &["CLAUDE.md".to_string()], &gitignore);

        assert!(hints.contains("Custom hints file content"));
        assert!(!hints.contains(".goosehints")); // Make sure it's not loading the default
    }

    #[test]
    fn test_nested_goosehints_with_git_root() {
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
        let hints = load_hint_files(
            &current_dir,
            &[GOOSE_HINTS_FILENAME.to_string()],
            &gitignore,
        );

        assert!(
            hints.contains("Root hints content\nSubdir hints content\ncurrent_dir hints content")
        );
    }

    #[test]
    fn test_nested_goosehints_without_git_root() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        fs::write(base_dir.join(GOOSE_HINTS_FILENAME), "Base hints content").unwrap();

        let subdir = base_dir.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join(GOOSE_HINTS_FILENAME), "Subdir hints content").unwrap();

        let current_dir = subdir.join("current_dir");
        fs::create_dir(&current_dir).unwrap();
        fs::write(
            current_dir.join(GOOSE_HINTS_FILENAME),
            "Current dir hints content",
        )
        .unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(
            &current_dir,
            &[GOOSE_HINTS_FILENAME.to_string()],
            &gitignore,
        );

        // Without .git, should only find hints in current directory
        assert!(hints.contains("Current dir hints content"));
        assert!(!hints.contains("Base hints content"));
        assert!(!hints.contains("Subdir hints content"));
    }

    #[test]
    fn test_nested_goosehints_mixed_filenames() {
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
    }

    #[test]
    fn test_hints_with_basic_imports() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        fs::create_dir(project_root.join(".git")).unwrap();

        fs::write(project_root.join("README.md"), "# Project README").unwrap();
        fs::write(project_root.join("config.md"), "Configuration details").unwrap();

        let hints_content = r#"Project hints content
@README.md
@config.md
Additional instructions here."#;
        fs::write(project_root.join(GOOSE_HINTS_FILENAME), hints_content).unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(
            project_root,
            &[GOOSE_HINTS_FILENAME.to_string()],
            &gitignore,
        );

        assert!(hints.contains("Project hints content"));
        assert!(hints.contains("Additional instructions here"));

        assert!(hints.contains("--- Content from README.md ---"));
        assert!(hints.contains("# Project README"));
        assert!(hints.contains("--- End of README.md ---"));

        assert!(hints.contains("--- Content from config.md ---"));
        assert!(hints.contains("Configuration details"));
        assert!(hints.contains("--- End of config.md ---"));
    }

    #[test]
    fn test_hints_with_git_import_boundary() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();

        fs::create_dir(project_root.join(".git")).unwrap();

        fs::write(project_root.join("root_file.md"), "Root file content").unwrap();
        fs::write(
            project_root.join("shared_docs.md"),
            "Shared documentation content",
        )
        .unwrap();

        let docs_dir = project_root.join("docs");
        fs::create_dir_all(&docs_dir).unwrap();
        fs::write(docs_dir.join("api.md"), "API documentation content").unwrap();

        let utils_dir = project_root.join("src").join("utils");
        fs::create_dir_all(&utils_dir).unwrap();
        fs::write(
            utils_dir.join("helpers.md"),
            "Helper utilities content @../../shared_docs.md",
        )
        .unwrap();

        let components_dir = project_root.join("src").join("components");
        fs::create_dir_all(&components_dir).unwrap();
        fs::write(components_dir.join("local_file.md"), "Local file content").unwrap();

        let outside_dir = temp_dir.path().parent().unwrap();
        fs::write(outside_dir.join("forbidden.md"), "Forbidden content").unwrap();

        let root_hints_content = r#"Project root hints
@docs/api.md
Root level instructions"#;
        fs::write(project_root.join(GOOSE_HINTS_FILENAME), root_hints_content).unwrap();

        let nested_hints_content = r#"Nested directory hints
@local_file.md
@../utils/helpers.md
@../../docs/api.md
@../../root_file.md
@../../../forbidden.md
End of nested hints"#;
        fs::write(
            components_dir.join(GOOSE_HINTS_FILENAME),
            nested_hints_content,
        )
        .unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(
            &components_dir,
            &[GOOSE_HINTS_FILENAME.to_string()],
            &gitignore,
        );
        println!("======{}", hints);
        assert!(hints.contains("Project root hints"));
        assert!(hints.contains("Root level instructions"));

        assert!(hints.contains("API documentation content"));
        assert!(hints.contains("--- Content from docs/api.md ---"));

        assert!(hints.contains("Nested directory hints"));
        assert!(hints.contains("End of nested hints"));

        assert!(hints.contains("Local file content"));
        assert!(hints.contains("--- Content from local_file.md ---"));

        assert!(hints.contains("Helper utilities content"));
        assert!(hints.contains("--- Content from ../utils/helpers.md ---"));
        assert!(hints.contains("Shared documentation content"));
        assert!(hints.contains("--- Content from ../../shared_docs.md ---"));

        let api_content_count = hints.matches("API documentation content").count();
        assert_eq!(
            api_content_count, 2,
            "API content should appear twice - from root and nested hints"
        );

        assert!(hints.contains("Root file content"));
        assert!(hints.contains("--- Content from ../../root_file.md ---"));

        assert!(!hints.contains("Forbidden content"));
        assert!(hints.contains("@../../../forbidden.md"));
    }

    #[test]
    fn test_hints_without_git_import_boundary() {
        let temp_dir = TempDir::new().unwrap();
        let base_dir = temp_dir.path();

        let current_dir = base_dir.join("current");
        fs::create_dir(&current_dir).unwrap();
        fs::write(current_dir.join("local.md"), "Local content").unwrap();

        fs::write(base_dir.join("parent.md"), "Parent content").unwrap();

        let hints_content = r#"Current directory hints
@local.md
@../parent.md
End of hints"#;
        fs::write(current_dir.join(GOOSE_HINTS_FILENAME), hints_content).unwrap();

        let gitignore = create_dummy_gitignore();
        let hints = load_hint_files(
            &current_dir,
            &[GOOSE_HINTS_FILENAME.to_string()],
            &gitignore,
        );

        assert!(hints.contains("Local content"));
        assert!(hints.contains("--- Content from local.md ---"));

        assert!(!hints.contains("Parent content"));
        assert!(hints.contains("@../parent.md"));
    }

    #[test]
    fn test_import_boundary_respects_nested_setting() {
        let temp_dir = TempDir::new().unwrap();
        let project_root = temp_dir.path();
        fs::create_dir(project_root.join(".git")).unwrap();
        fs::write(project_root.join("root_file.md"), "Root file content").unwrap();
        let subdir = project_root.join("subdir");
        fs::create_dir(&subdir).unwrap();
        fs::write(subdir.join("local_file.md"), "Local file content").unwrap();
        let hints_content = r#"Subdir hints
@local_file.md
@../root_file.md
End of hints"#;
        fs::write(subdir.join(GOOSE_HINTS_FILENAME), hints_content).unwrap();
        let gitignore = create_dummy_gitignore();

        let hints = load_hint_files(&subdir, &[GOOSE_HINTS_FILENAME.to_string()], &gitignore);

        assert!(hints.contains("Local file content"));
        assert!(hints.contains("--- Content from local_file.md ---"));

        assert!(hints.contains("Root file content"));
        assert!(hints.contains("--- Content from ../root_file.md ---"));
    }
}
