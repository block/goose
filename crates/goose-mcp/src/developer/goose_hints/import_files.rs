use ignore::gitignore::Gitignore;
use once_cell::sync::Lazy;
use std::{collections::HashSet, path::{Path, PathBuf}};

static FILE_REFERENCE_REGEX: Lazy<regex::Regex> = Lazy::new(|| {
    regex::Regex::new(r"(?:^|\s)@([a-zA-Z0-9_\-./]+(?:\.[a-zA-Z0-9]+)+|[A-Z][a-zA-Z0-9_\-]*|[a-zA-Z0-9_\-./]*[./][a-zA-Z0-9_\-./]*)")
        .expect("Invalid file reference regex pattern")
});

const MAX_DEPTH: usize = 3;

fn sanitize_reference_path(reference: &Path, including_file_path: &Path, base_path: &Path) -> Result<PathBuf, std::io::Error> {
    if reference.is_absolute() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "Absolute paths not allowed in file references",
        ));
    }
    let resolved = including_file_path.join(reference);
    let base_canonical = base_path.canonicalize().map_err(|_| {
        std::io::Error::new(std::io::ErrorKind::NotFound, "Base directory not found")
    })?;

    if let Ok(canonical) = resolved.canonicalize() {
        if !canonical.starts_with(&base_canonical) {
            return Err(std::io::Error::new(
                std::io::ErrorKind::PermissionDenied,
                format!(
                    "Include: '{}' is outside the project root '{}'",
                    resolved.display(),
                    base_path.display()
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
    base_path: &Path,
    visited: &HashSet<PathBuf>,
    ignore_patterns: &Gitignore,
) -> Option<PathBuf> {
    if visited.contains(reference) {
        return None;
    }
    let safe_path = match sanitize_reference_path(reference, including_file_path, base_path) {
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
    base_path: &Path,
    depth: usize,
    ignore_patterns: &Gitignore,
) -> Option<(String, String)> {
    visited.insert(reference.to_path_buf());

    let expanded_content = read_referenced_files(
        safe_path,
        base_path,
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
    base_path: &Path,
    visited: &mut HashSet<PathBuf>,
    depth: usize,
    ignore_patterns: &Gitignore,
) -> String {
    if depth >= MAX_DEPTH {
        tracing::warn!("Maximum reference depth {} exceeded", MAX_DEPTH);
        return String::new();
    }

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
        let safe_path =
            match should_process_reference(&reference, including_file_path, base_path, visited, ignore_patterns) {
                Some(path) => path,
                None => continue,
            };

        if let Some((pattern, replacement)) = process_file_reference(
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

// #[cfg(test)]
// mod tests {
//     use super::*;

//     #[test]
//     // Tests for @-mention file reference functionality
//     #[test]
//     fn test_parse_file_references() {
//         let content = r#"
//         Basic file references: @README.md @./docs/guide.md @../shared/config.json @/absolute/path/file.txt
//         Inline references: @file1.txt and @file2.py
//         Files with extensions: @component.tsx @file.test.js @config.local.json
//         Files without extensions: @Makefile @LICENSE @Dockerfile @CHANGELOG
//         Complex paths: @src/utils/helper.js @docs/api/endpoints.md
        
//         Should not match:
//         - Email addresses: user@example.com admin@company.org
//         - Social handles: @username @user123
//         - URLs: https://example.com/@user
//         "#;

//         let references = parse_file_references(content);

//         // Should match basic file references
//         assert!(references.contains(&PathBuf::from("README.md")));
//         assert!(references.contains(&PathBuf::from("./docs/guide.md")));
//         assert!(references.contains(&PathBuf::from("../shared/config.json")));
//         assert!(references.contains(&PathBuf::from("/absolute/path/file.txt")));
//         assert!(references.contains(&PathBuf::from("file1.txt")));
//         assert!(references.contains(&PathBuf::from("file2.py")));

//         // Should match files with extensions (including multiple dots)
//         assert!(references.contains(&PathBuf::from("component.tsx")));
//         assert!(references.contains(&PathBuf::from("file.test.js")));
//         assert!(references.contains(&PathBuf::from("config.local.json")));

//         // Should match files without extensions
//         assert!(references.contains(&PathBuf::from("Makefile")));
//         assert!(references.contains(&PathBuf::from("LICENSE")));
//         assert!(references.contains(&PathBuf::from("Dockerfile")));
//         assert!(references.contains(&PathBuf::from("CHANGELOG")));

//         // Should match complex paths
//         assert!(references.contains(&PathBuf::from("src/utils/helper.js")));
//         assert!(references.contains(&PathBuf::from("docs/api/endpoints.md")));

//         // Should not match email addresses or social handles
//         assert!(!references
//             .iter()
//             .any(|p| p.to_str().unwrap().contains("example.com")));
//         assert!(!references
//             .iter()
//             .any(|p| p.to_str().unwrap().contains("company.org")));
//         assert!(!references.iter().any(|p| p.to_str().unwrap() == "username"));
//         assert!(!references.iter().any(|p| p.to_str().unwrap() == "user123"));
//     }

//     #[test]
//     #[serial]
//     fn test_file_expansion_normal_cases() {
//         let temp_dir = tempfile::tempdir().unwrap();
//         let base_path = temp_dir.path();

//         // Test 1: Basic file reference
//         let basic_file = base_path.join("basic.md");
//         std::fs::write(&basic_file, "This is basic content").unwrap();

//         let builder = GitignoreBuilder::new(base_path);
//         let ignore_patterns = builder.build().unwrap();

//         let mut visited = HashSet::new();
//         let basic_content = "Main content\n@basic.md\nMore content";
//         let expanded =
//             read_referenced_files(basic_content, base_path, &mut visited, 0, &ignore_patterns);

//         assert!(expanded.contains("Main content"));
//         assert!(expanded.contains("--- Content from"));
//         assert!(expanded.contains("This is basic content"));
//         assert!(expanded.contains("--- End of"));
//         assert!(expanded.contains("More content"));

//         // Test 2: Nested file references
//         let ref_file1 = base_path.join("level1.md");
//         std::fs::write(&ref_file1, "Level 1 content\n@level2.md").unwrap();

//         let ref_file2 = base_path.join("level2.md");
//         std::fs::write(&ref_file2, "Level 2 content").unwrap();

//         visited.clear();
//         let nested_content = "Main content\n@level1.md";
//         let expanded =
//             read_referenced_files(nested_content, base_path, &mut visited, 0, &ignore_patterns);

//         assert!(expanded.contains("Main content"));
//         assert!(expanded.contains("Level 1 content"));
//         assert!(expanded.contains("Level 2 content"));

//         temp_dir.close().unwrap();
//     }

//     #[test]
//     #[serial]
//     fn test_file_expansion_edge_cases() {
//         let temp_dir = tempfile::tempdir().unwrap();
//         let base_path = temp_dir.path();
//         let builder = GitignoreBuilder::new(base_path);
//         let ignore_patterns = builder.build().unwrap();

//         // Test 1: Circular references
//         let ref_file1 = base_path.join("file1.md");
//         std::fs::write(&ref_file1, "File 1\n@file2.md").unwrap();
//         let ref_file2 = base_path.join("file2.md");
//         std::fs::write(&ref_file2, "File 2\n@file1.md").unwrap();

//         let mut visited = HashSet::new();
//         let circular_content = "Main\n@file1.md";
//         let expanded = read_referenced_files(
//             circular_content,
//             base_path,
//             &mut visited,
//             0,
//             &ignore_patterns,
//         );

//         assert!(expanded.contains("File 1"));
//         assert!(expanded.contains("File 2"));
//         // Should only appear once due to circular reference protection
//         let file1_count = expanded.matches("File 1").count();
//         assert_eq!(file1_count, 1);

//         // Test 2: Max depth limit
//         for i in 1..=5 {
//             let content = if i < 5 {
//                 format!("Level {} content\n@level{}.md", i, i + 1)
//             } else {
//                 format!("Level {} content", i)
//             };
//             let ref_file = base_path.join(format!("level{}.md", i));
//             std::fs::write(&ref_file, content).unwrap();
//         }

//         visited.clear();
//         let depth_content = "Main\n@level1.md";
//         let expanded =
//             read_referenced_files(depth_content, base_path, &mut visited, 0, &ignore_patterns);

//         // Should contain up to level 3 (MAX_DEPTH = 3)
//         assert!(expanded.contains("Level 1 content"));
//         assert!(expanded.contains("Level 2 content"));
//         assert!(expanded.contains("Level 3 content"));
//         // Should not contain level 4 or 5 due to depth limit
//         assert!(!expanded.contains("Level 4 content"));
//         assert!(!expanded.contains("Level 5 content"));

//         // Test 3: Missing file
//         visited.clear();
//         let missing_content = "Main\n@missing.md\nMore content";
//         let expanded = read_referenced_files(
//             missing_content,
//             base_path,
//             &mut visited,
//             0,
//             &ignore_patterns,
//         );

//         // Should keep the original reference unchanged
//         assert!(expanded.contains("@missing.md"));
//         assert!(!expanded.contains("--- Content from"));

//         temp_dir.close().unwrap();
//     }

//     #[test]
//     #[serial]
//     fn test_read_referenced_files_respects_ignore() {
//         let temp_dir = tempfile::tempdir().unwrap();
//         let base_path = temp_dir.path();

//         // Create referenced files
//         let allowed_file = base_path.join("allowed.md");
//         std::fs::write(&allowed_file, "Allowed content").unwrap();

//         let ignored_file = base_path.join("secret.md");
//         std::fs::write(&ignored_file, "Secret content").unwrap();

//         // Create main content with references
//         let content = "Main\n@allowed.md\n@secret.md";

//         // Create ignore patterns
//         let mut builder = GitignoreBuilder::new(base_path);
//         builder.add_line(None, "secret.md").unwrap();
//         let ignore_patterns = builder.build().unwrap();

//         let mut visited = HashSet::new();
//         let expanded = read_referenced_files(content, base_path, &mut visited, 0, &ignore_patterns);

//         // Should contain allowed content but not ignored content
//         assert!(expanded.contains("Allowed content"));
//         assert!(!expanded.contains("Secret content"));

//         // The @secret.md reference should remain unchanged
//         assert!(expanded.contains("@secret.md"));

//         temp_dir.close().unwrap();
//     }

//     #[test]
//     #[serial]
//     fn test_goosehints_with_file_references() {
//         let temp_dir = tempfile::tempdir().unwrap();
//         std::env::set_current_dir(&temp_dir).unwrap();

//         // Create referenced files
//         let readme_path = temp_dir.path().join("README.md");
//         std::fs::write(
//             &readme_path,
//             "# Project README\n\nThis is the project documentation.",
//         )
//         .unwrap();

//         let guide_path = temp_dir.path().join("guide.md");
//         std::fs::write(&guide_path, "# Development Guide\n\nFollow these steps...").unwrap();

//         // Create .goosehints with references
//         let hints_content = r#"# Project Information

// Please refer to:
// @README.md
// @guide.md

// Additional instructions here.
// "#;
//         let hints_path = temp_dir.path().join(".goosehints");
//         std::fs::write(&hints_path, hints_content).unwrap();

//         // Create router and check instructions
//         let router = DeveloperRouter::new();
//         let instructions = router.instructions();

//         // Should contain the .goosehints content
//         assert!(instructions.contains("Project Information"));
//         assert!(instructions.contains("Additional instructions here"));

//         // Should contain the referenced files' content
//         assert!(instructions.contains("# Project README"));
//         assert!(instructions.contains("This is the project documentation"));
//         assert!(instructions.contains("# Development Guide"));
//         assert!(instructions.contains("Follow these steps"));

//         // Should have attribution markers
//         assert!(instructions.contains("--- Content from"));
//         assert!(instructions.contains("--- End of"));

//         temp_dir.close().unwrap();
//     }

//     #[test]
//     #[serial]
//     fn test_parse_file_references_redos_protection() {
//         // Test very large input to ensure ReDoS protection
//         let large_content = "@".repeat(2_000_000); // 2MB of @ symbols
//         let references = parse_file_references(&large_content);
//         // Should return empty due to size limit, not hang
//         assert!(references.is_empty());

//         // Test normal size content still works
//         let normal_content = "Check out @README.md for details";
//         let references = parse_file_references(&normal_content);
//         assert_eq!(references.len(), 1);
//         assert_eq!(references[0], PathBuf::from("README.md"));
//     }

//     #[test]
//     #[serial]
//     fn test_security_integration_with_file_expansion() {
//         let temp_dir = tempfile::tempdir().unwrap();
//         let base_path = temp_dir.path();

//         // Create a config file attempting path traversal
//         let malicious_content = r#"
//         Normal content here.
//         @../../../etc/passwd
//         @/absolute/path/file.txt
//         @legitimate_file.md
//         "#;

//         // Create a legitimate file
//         let legit_file = base_path.join("legitimate_file.md");
//         std::fs::write(&legit_file, "This is safe content").unwrap();

//         // Create ignore patterns
//         let builder = GitignoreBuilder::new(base_path);
//         let ignore_patterns = builder.build().unwrap();

//         let mut visited = HashSet::new();
//         let expanded = read_referenced_files(
//             malicious_content,
//             base_path,
//             &mut visited,
//             0,
//             &ignore_patterns,
//         );

//         // Should contain the legitimate file but not the malicious attempts
//         assert!(expanded.contains("This is safe content"));
//         assert!(!expanded.contains("root:")); // Common content in /etc/passwd

//         // The malicious references should still be present (not expanded)
//         assert!(expanded.contains("@../../../etc/passwd"));
//         assert!(expanded.contains("@/absolute/path/file.txt"));

//         temp_dir.close().unwrap();
//     }
// }