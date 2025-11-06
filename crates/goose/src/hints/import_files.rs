use ignore::gitignore::Gitignore;
use once_cell::sync::Lazy;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

static FILE_REFERENCE_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"(?:^|\s)@([a-zA-Z0-9_\-./]+(?:\.[a-zA-Z0-9]+)+|[A-Z][a-zA-Z0-9_\-]*|[a-zA-Z0-9_\-./]*[./][a-zA-Z0-9_\-./]*)")
        .expect("Invalid file reference regex pattern")
});

const MAX_DEPTH: usize = 3;

fn sanitize_reference_path(
    reference: &Path,
    including_file_path: &Path,
    import_boundary: &Path,
) -> Result<PathBuf, std::io::Error> {
    if reference.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Absolute paths not allowed in file references",
        ));
    }
    let resolved = including_file_path.join(reference);
    let boundary_canonical = import_boundary.canonicalize().map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "Import boundary directory not found",
        )
    })?;

    if let Ok(canonical) = resolved.canonicalize() {
        if !canonical.starts_with(&boundary_canonical) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "Include: '{}' is outside the import boundary '{}'",
                    resolved.display(),
                    import_boundary.display()
                ),
            ));
        }
        Ok(canonical)
    } else {
        Ok(resolved) // File doesn't exist, but path structure is safe
    }
}

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

fn should_process_reference(
    reference: &Path,
    including_file_path: &Path,
    import_boundary: &Path,
    visited: &HashSet<PathBuf>,
    ignore_patterns: &Gitignore,
) -> Option<PathBuf> {
    if visited.contains(reference) {
        return None;
    }
    let safe_path = match sanitize_reference_path(reference, including_file_path, import_boundary) {
        Ok(path) => path,
        Err(_) => {
            tracing::warn!("Skipping unsafe file reference: {:?}", reference);
            return None;
        }
    };

    if ignore_patterns.matched(&safe_path, false).is_ignore() {
        tracing::debug!("Skipping ignored file reference: {:?}", safe_path);
        return None;
    }

    if !safe_path.is_file() {
        return None;
    }

    Some(safe_path)
}

fn process_file_reference(
    reference: &Path,
    safe_path: &Path,
    visited: &mut HashSet<PathBuf>,
    import_boundary: &Path,
    depth: usize,
    ignore_patterns: &Gitignore,
) -> Option<(String, String)> {
    if depth >= MAX_DEPTH {
        tracing::warn!("Maximum reference depth {} exceeded", MAX_DEPTH);
        return None;
    }

    visited.insert(reference.to_path_buf());

    let expanded_content = read_referenced_files(
        safe_path,
        import_boundary,
        visited,
        depth + 1,
        ignore_patterns,
    );

    let reference_pattern = format!("@{}", reference.to_string_lossy());
    let replacement = format!(
        "--- Content from {} ---\n{}\n--- End of {} ---",
        reference.display(),
        expanded_content,
        reference.display()
    );

    visited.remove(reference);

    Some((reference_pattern, replacement))
}

pub fn read_referenced_files(
    file_path: &Path,
    import_boundary: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    ignore_patterns: &Gitignore,
) -> String {
    let content = match std::fs::read_to_string(file_path) {
        Ok(content) => content,
        Err(e) => {
            tracing::warn!("Could not read file {:?}: {}", file_path, e);
            return String::new();
        }
    };

    let including_file_path = file_path.parent().unwrap_or(file_path);

    let references = parse_file_references(&content);
    let mut result = content.to_string();

    for reference in references {
        let safe_path = match should_process_reference(
            &reference,
            including_file_path,
            import_boundary,
            visited,
            ignore_patterns,
        ) {
            Some(path) => path,
            None => continue,
        };

        if let Some((pattern, replacement)) = process_file_reference(
            &reference,
            &safe_path,
            visited,
            import_boundary,
            depth,
            ignore_patterns,
        ) {
            result = result.replace(&pattern, &replacement);
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use ignore::gitignore::GitignoreBuilder;

    use super::*;

    #[test]
    fn test_parse_file_references() {
        let content = r#"
        Basic file references: @README.md @./docs/guide.md @../shared/config.json @/absolute/path/file.txt
        Inline references: @file1.txt and @file2.py
        Files with extensions: @component.tsx @file.test.js @config.local.json
        Files without extensions: @Makefile @LICENSE @Dockerfile @CHANGELOG
        Complex paths: @src/utils/helper.js @docs/api/endpoints.md
        
        Should not match:
        - Email addresses: user@example.com admin@company.org
        - Social handles: @username @user123
        - URLs: https://example.com/@user
        "#;

        let references = parse_file_references(content);

        // Should match expected file references
        let expected_files = [
            "README.md",
            "./docs/guide.md",
            "../shared/config.json",
            "/absolute/path/file.txt",
            "file1.txt",
            "file2.py",
            "component.tsx",
            "file.test.js",
            "config.local.json",
            "Makefile",
            "LICENSE",
            "Dockerfile",
            "CHANGELOG",
            "src/utils/helper.js",
            "docs/api/endpoints.md",
        ];

        for expected in expected_files {
            assert!(
                references.contains(&PathBuf::from(expected)),
                "Expected to find reference: {}",
                expected
            );
        }

        // Should not match email addresses or social handles
        assert!(!references
            .iter()
            .any(|p| p.to_str().unwrap().contains("example.com")));
        assert!(!references
            .iter()
            .any(|p| p.to_str().unwrap().contains("company.org")));
        assert!(!references.iter().any(|p| p.to_str().unwrap() == "username"));
        assert!(!references.iter().any(|p| p.to_str().unwrap() == "user123"));
    }

    mod read_referenced_files {
        use super::*;

        fn create_ignore_patterns(import_boundary: &Path) -> Gitignore {
            let builder = GitignoreBuilder::new(import_boundary);
            builder.build().unwrap()
        }

        fn create_file(import_boundary: &Path, file_name: &str, content: &str) -> PathBuf {
            let file_path = import_boundary.join(file_name);
            std::fs::write(&file_path, content).unwrap();
            file_path
        }

        #[test]
        fn test_direct_reference() {
            let temp_dir = tempfile::tempdir().unwrap();
            let import_boundary = temp_dir.path();

            create_file(
                import_boundary,
                "basic_included_file.md",
                "This is basic content",
            );

            let ignore_patterns = create_ignore_patterns(import_boundary);

            let mut visited = HashSet::new();
            let main_file = create_file(
                import_boundary,
                "main.md",
                "Main content\n@basic_included_file.md\nMore content",
            );

            let expanded = read_referenced_files(
                &main_file,
                import_boundary,
                &mut visited,
                0,
                &ignore_patterns,
            );

            assert!(expanded.contains("Main content"));
            assert!(expanded.contains("--- Content from"));
            assert!(expanded.contains("This is basic content"));
            assert!(expanded.contains("--- End of"));
            assert!(expanded.contains("More content"));
        }

        #[test]
        fn test_nested_reference() {
            let temp_dir = tempfile::tempdir().unwrap();
            let import_boundary = temp_dir.path();

            create_file(import_boundary, "level1.md", "Level 1 content\n@level2.md");
            create_file(import_boundary, "level2.md", "Level 2 content");

            let mut visited = HashSet::new();
            let main_file = create_file(import_boundary, "main.md", "Main content\n@level1.md");

            let ignore_patterns = create_ignore_patterns(import_boundary);
            let expanded = read_referenced_files(
                &main_file,
                import_boundary,
                &mut visited,
                0,
                &ignore_patterns,
            );

            assert!(expanded.contains("Main content"));
            assert!(expanded.contains("Level 1 content"));
            assert!(expanded.contains("Level 2 content"));
        }

        #[test]
        fn test_circular_reference() {
            let temp_dir = tempfile::tempdir().unwrap();
            let import_boundary = temp_dir.path();

            let ignore_patterns = create_ignore_patterns(import_boundary);
            create_file(import_boundary, "file1.md", "File 1\n@file2.md");
            create_file(import_boundary, "file2.md", "File 2\n@file1.md");
            let main_file = create_file(import_boundary, "main.md", "Main\n@file1.md");

            let mut visited = HashSet::new();
            let expanded = read_referenced_files(
                &main_file,
                import_boundary,
                &mut visited,
                0,
                &ignore_patterns,
            );

            assert!(expanded.contains("File 1"));
            assert!(expanded.contains("File 2"));
            // Should only appear once due to circular reference protection
            let file1_count = expanded.matches("File 1").count();
            assert_eq!(file1_count, 1);
        }

        #[test]
        fn test_max_depth_limit() {
            let temp_dir = tempfile::tempdir().unwrap();
            let import_boundary = temp_dir.path();
            let ignore_patterns = create_ignore_patterns(import_boundary);
            let mut visited = HashSet::new();
            for i in 1..=5 {
                let content = if i < 5 {
                    format!("Level {} content\n@level{}.md", i, i + 1)
                } else {
                    format!("Level {} content", i)
                };
                create_file(import_boundary, &format!("level{}.md", i), &content);
            }
            let main_file = create_file(import_boundary, "main.md", "Main\n@level1.md");
            let expanded = read_referenced_files(
                &main_file,
                import_boundary,
                &mut visited,
                0,
                &ignore_patterns,
            );
            // Should contain up to level 3 (MAX_DEPTH = 3)
            assert!(expanded.contains("Level 1 content"));
            assert!(expanded.contains("Level 2 content"));
            assert!(expanded.contains("Level 3 content"));
            // Should not contain level 4 or 5 due to depth limit
            assert!(!expanded.contains("Level 4 content"));
            assert!(!expanded.contains("Level 5 content"));
        }

        #[test]
        fn test_missing_file() {
            let temp_dir = tempfile::tempdir().unwrap();
            let import_boundary = temp_dir.path();
            let ignore_patterns = create_ignore_patterns(import_boundary);
            let mut visited = HashSet::new();
            let main_file = create_file(
                import_boundary,
                "main.md",
                "Main\n@missing.md\nMore content",
            );

            let expanded = read_referenced_files(
                &main_file,
                import_boundary,
                &mut visited,
                0,
                &ignore_patterns,
            );

            assert!(expanded.contains("@missing.md"));
            assert!(!expanded.contains("--- Content from"));
        }

        #[test]
        fn test_read_referenced_files_respects_ignore() {
            let temp_dir = tempfile::tempdir().unwrap();
            let import_boundary = temp_dir.path();

            create_file(import_boundary, "allowed.md", "Allowed content");
            create_file(import_boundary, "secret.md", "Secret content");

            let mut builder = GitignoreBuilder::new(import_boundary);
            builder.add_line(None, "secret.md").unwrap();
            let ignore_patterns = builder.build().unwrap();

            let mut visited = HashSet::new();
            // Create main content with references
            let content = "Main\n@allowed.md\n@secret.md";
            let main_file = create_file(import_boundary, "main.md", content);
            let expanded = read_referenced_files(
                &main_file,
                import_boundary,
                &mut visited,
                0,
                &ignore_patterns,
            );

            // Should contain allowed content but not ignored content
            assert!(expanded.contains("Allowed content"));
            assert!(!expanded.contains("Secret content"));

            // The @secret.md reference should remain unchanged
            assert!(expanded.contains("@secret.md"));

            temp_dir.close().unwrap();
        }

        #[test]
        fn test_security_integration_with_file_expansion() {
            let temp_dir = tempfile::tempdir().unwrap();
            let import_boundary = temp_dir.path();
            let ignore_patterns = create_ignore_patterns(import_boundary);

            // Create a legitimate file
            create_file(
                import_boundary,
                "legitimate_file.md",
                "This is safe content",
            );

            let absolute_path_file = create_file(
                import_boundary,
                "used_with_absolute_path.md",
                "Absolute path content",
            );
            let absolute_path_file_path = absolute_path_file
                .canonicalize()
                .unwrap()
                .to_string_lossy()
                .into_owned();

            // Create a config file attempting path traversal
            let malicious_content = format!(
                r#"
            Normal content here.
            @../etc/passwd
            @{}
            @legitimate_file.md
            "#,
                absolute_path_file_path
            );
            create_file(import_boundary, "main.md", &malicious_content);

            let mut visited = HashSet::new();
            let expanded = read_referenced_files(
                &import_boundary.join("main.md"),
                import_boundary,
                &mut visited,
                0,
                &ignore_patterns,
            );

            // Should contain the legitimate file but not the malicious attempts
            assert!(expanded.contains("This is safe content"));
            assert!(!expanded.contains("root:")); // Common content in /etc/passwd
            assert!(!expanded.contains("Absolute path content"));

            // The malicious references should still be present (not expanded)
            assert!(expanded.contains("@../etc/passwd"));
            assert!(expanded.contains(absolute_path_file_path.as_str()));
        }
    }
}
