use etcetera::{choose_app_strategy, AppStrategy};
use ignore::gitignore::Gitignore;
use once_cell::sync::Lazy;
use std::{collections::HashSet, path::{Path, PathBuf}};

pub const GOOSE_HINTS_FILENAME: &str = ".goosehints";

static FILE_REFERENCE_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"(?:^|\s)@([a-zA-Z0-9_\-./]+(?:\.[a-zA-Z0-9]+)+|[A-Z][a-zA-Z0-9_\-]*|[a-zA-Z0-9_\-./]*[./][a-zA-Z0-9_\-./]*)")
        .expect("Invalid file reference regex pattern")
});

/// Sanitize and resolve a file reference path safely
///
/// This function prevents path traversal attacks by:
/// 1. Rejecting absolute paths
/// 2. Resolving the path canonically
/// 3. Ensuring the resolved path stays within the allowed base directory
fn sanitize_reference_path(reference: &Path, base_path: &Path) -> Result<PathBuf, std::io::Error> {
    if reference.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Absolute paths not allowed in file references",
        ));
    }

    let resolved = base_path.join(reference);
    let base_canonical = base_path.canonicalize().map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Base directory not found")
    })?;

    if let Ok(canonical) = resolved.canonicalize() {
        if !canonical.starts_with(&base_canonical) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                "Path traversal attempt detected",
            ));
        }
        Ok(canonical)
    } else {
        Ok(resolved) // File doesn't exist, but path structure is safe
    }
}

/// Parse file references (@-mentions) from content
fn parse_file_references(content: &str) -> Vec<PathBuf> {
    // Keep size limits for ReDoS protection - .goosehints should be reasonably sized
    const MAX_CONTENT_LENGTH: usize = 131_072; // 128KB limit

    if content.len() > MAX_CONTENT_LENGTH {
        tracing::warn!(
            "Content too large for file reference parsing: {} bytes (limit: {} bytes)",
            content.len(),
            MAX_CONTENT_LENGTH
        );
        return Vec::new();
    }

    FILE_REFERENCE_REGEX
        .captures_iter(content)
        .map(|cap| PathBuf::from(&cap[1]))
        .collect()
}

/// Read referenced files and expand their content
/// Check if a file reference should be processed
fn should_process_reference_v2(
    reference: &Path,
    visited: &HashSet<PathBuf>,
    base_path: &Path,
    ignore_patterns: &Gitignore,
) -> Option<PathBuf> {
    // Check if we've already visited this file (circular reference protection)
    if visited.contains(reference) {
        return None;
    }

    // Sanitize the path
    let safe_path = match sanitize_reference_path(reference, base_path) {
        Ok(path) => path,
        Err(_) => {
            tracing::warn!("Skipping unsafe file reference: {:?}", reference);
            return None;
        }
    };

    // Check if the file should be ignored
    if ignore_patterns.matched(&safe_path, false).is_ignore() {
        tracing::debug!("Skipping ignored file reference: {:?}", safe_path);
        return None;
    }

    // Check if file exists
    if !safe_path.is_file() {
        return None;
    }

    Some(safe_path)
}

/// Process a single file reference and return the replacement content
fn process_file_reference_v2(
    reference: &Path,
    safe_path: &Path,
    visited: &mut HashSet<PathBuf>,
    base_path: &Path,
    depth: usize,
    ignore_patterns: &Gitignore,
) -> Option<(String, String)> {
    match std::fs::read_to_string(safe_path) {
        Ok(file_content) => {
            // Mark this file as visited
            visited.insert(reference.to_path_buf());

            // Recursively expand any references in the included file
            let expanded_content = read_referenced_files(
                &file_content,
                base_path,
                visited,
                depth + 1,
                ignore_patterns,
            );

            // Create the replacement content
            let reference_pattern = format!("@{}", reference.to_string_lossy());
            let replacement = format!(
                "--- Content from {} ---\n{}\n--- End of {} ---",
                reference.display(),
                expanded_content,
                reference.display()
            );

            // Remove from visited so it can be referenced again in different contexts
            visited.remove(reference);

            Some((reference_pattern, replacement))
        }
        Err(e) => {
            tracing::warn!("Could not read referenced file {:?}: {}", safe_path, e);
            None
        }
    }
}

fn read_referenced_files(
    content: &str,
    base_path: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    ignore_patterns: &Gitignore,
) -> String {
    const MAX_DEPTH: usize = 3;

    if depth >= MAX_DEPTH {
        tracing::warn!("Maximum reference depth {} exceeded", MAX_DEPTH);
        return content.to_string();
    }

    let references = parse_file_references(content);
    let mut result = content.to_string();

    for reference in references {
        let safe_path =
            match should_process_reference_v2(&reference, visited, base_path, ignore_patterns) {
                Some(path) => path,
                None => continue,
            };

        if let Some((pattern, replacement)) = process_file_reference_v2(
            &reference,
            &safe_path,
            visited,
            base_path,
            depth,
            ignore_patterns,
        ) {
            result = result.replace(&pattern, &replacement);
        }
    }

    result
}

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

pub fn load_hints(cwd: &Path, hints_filenames: &[String]) -> String {
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
                if let Ok(content) = std::fs::read_to_string(&hints_path) {
                    local_hints_contents.push(content);
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
    use serial_test::serial;
    use std::fs;
    use tempfile::TempDir;

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

        let hints = load_hints(dir.path(), &[GOOSE_HINTS_FILENAME.to_string()]);

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
        let hints = load_hints(dir.path(), &[GOOSE_HINTS_FILENAME.to_string()]);

        assert!(hints.contains("Test hint content"));
    }

    #[test]
    #[serial]
    fn test_goosehints_when_missing() {
        let dir = TempDir::new().unwrap();
        std::env::set_current_dir(dir.path()).unwrap();

        let hints = load_hints(dir.path(), &[GOOSE_HINTS_FILENAME.to_string()]);

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

        let hints = load_hints(
            dir.path(),
            &["CLAUDE.md".to_string(), GOOSE_HINTS_FILENAME.to_string()],
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
        let hints = load_hints(dir.path(), &["CLAUDE.md".to_string()]);

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

        let hints = load_hints(&current_dir, &[GOOSE_HINTS_FILENAME.to_string()]);

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

        let hints = load_hints(&current_dir, &[GOOSE_HINTS_FILENAME.to_string()]);

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

        let hints = load_hints(
            &current_dir,
            &["CLAUDE.md".to_string(), GOOSE_HINTS_FILENAME.to_string()],
        );

        assert!(hints.contains("Root CLAUDE.md content"));
        assert!(hints.contains("Subdir .goosehints content"));

        std::env::remove_var("NESTED_GOOSE_HINTS");
    }
}
