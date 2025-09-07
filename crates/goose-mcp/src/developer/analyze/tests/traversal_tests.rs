// Tests for the traversal module

use crate::developer::analyze::tests::fixtures::{create_test_gitignore, create_test_gitignore_at};
use crate::developer::analyze::traversal::FileTraverser;
use ignore::gitignore::Gitignore;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[tokio::test]
async fn test_is_ignored() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create actual files and directories to test
    fs::write(dir_path.join("test.log"), "log content").unwrap();
    fs::create_dir(dir_path.join("node_modules")).unwrap();
    fs::create_dir(dir_path.join("src")).unwrap();
    fs::write(dir_path.join("src").join("main.rs"), "fn main() {}").unwrap();

    // Create gitignore relative to temp dir
    let ignore = create_test_gitignore_at(dir_path);
    let traverser = FileTraverser::new(&ignore);

    // Test with actual paths relative to the gitignore base
    assert!(traverser.is_ignored(&dir_path.join("test.log")));
    assert!(traverser.is_ignored(&dir_path.join("node_modules")));
    assert!(!traverser.is_ignored(&dir_path.join("src").join("main.rs")));
}

#[tokio::test]
async fn test_validate_path() {
    let ignore = create_test_gitignore();
    let traverser = FileTraverser::new(&ignore);

    // Test non-existent path
    assert!(traverser
        .validate_path(Path::new("/nonexistent/path"))
        .is_err());

    // Test ignored path
    assert!(traverser.validate_path(Path::new("test.log")).is_err());
}

#[tokio::test]
async fn test_collect_files() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create test files
    fs::write(dir_path.join("test.rs"), "fn main() {}").unwrap();
    fs::write(dir_path.join("test.py"), "def main(): pass").unwrap();
    fs::write(dir_path.join("test.txt"), "not code").unwrap();

    // Create subdirectory with file
    let sub_dir = dir_path.join("src");
    fs::create_dir(&sub_dir).unwrap();
    fs::write(sub_dir.join("lib.rs"), "pub fn test() {}").unwrap();

    let ignore = Gitignore::empty();
    let traverser = FileTraverser::new(&ignore);

    let files = traverser
        .collect_files_for_focused(dir_path, 0)
        .await
        .unwrap();

    // Should find .rs and .py files but not .txt
    assert_eq!(files.len(), 3);
    assert!(files.iter().any(|p| p.ends_with("test.rs")));
    assert!(files.iter().any(|p| p.ends_with("test.py")));
    assert!(files.iter().any(|p| p.ends_with("lib.rs")));
}

#[tokio::test]
async fn test_max_depth() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create nested structure
    // Root level (depth 0)
    fs::write(dir_path.join("root.rs"), "").unwrap();

    // Level 1 (depth 1)
    let level1 = dir_path.join("level1");
    fs::create_dir(&level1).unwrap();
    fs::write(level1.join("file1.rs"), "").unwrap();

    // Level 2 (depth 2)
    let level2 = level1.join("level2");
    fs::create_dir(&level2).unwrap();
    fs::write(level2.join("file2.rs"), "").unwrap();

    let ignore = Gitignore::empty();
    let traverser = FileTraverser::new(&ignore);

    // With max_depth=1, should find root.rs and file1.rs (stops before level2)
    let files = traverser
        .collect_files_for_focused(dir_path, 1)
        .await
        .unwrap();
    assert_eq!(files.len(), 2, "max_depth=1 should find 2 files");
    assert!(files.iter().any(|p| p.ends_with("root.rs")));
    assert!(files.iter().any(|p| p.ends_with("file1.rs")));

    // With max_depth=2, should find all three files
    let files = traverser
        .collect_files_for_focused(dir_path, 2)
        .await
        .unwrap();
    assert_eq!(files.len(), 3, "max_depth=2 should find all 3 files");

    // With max_depth=0 (unlimited), should also find all three files
    let files = traverser
        .collect_files_for_focused(dir_path, 0)
        .await
        .unwrap();
    assert_eq!(
        files.len(),
        3,
        "max_depth=0 (unlimited) should find all 3 files"
    );
}

#[tokio::test]
async fn test_symlink_handling() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create a file and directory
    fs::write(dir_path.join("target.rs"), "fn main() {}").unwrap();
    let target_dir = dir_path.join("target_dir");
    fs::create_dir(&target_dir).unwrap();
    fs::write(target_dir.join("inner.rs"), "fn test() {}").unwrap();

    // Create symlinks (if supported by the OS)
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;
        let _ = symlink(&dir_path.join("target.rs"), dir_path.join("link.rs"));
        let _ = symlink(&target_dir, dir_path.join("link_dir"));
    }

    let ignore = Gitignore::empty();
    let traverser = FileTraverser::new(&ignore);

    // Collect files - symlinks should be handled appropriately
    let files = traverser
        .collect_files_for_focused(dir_path, 0)
        .await
        .unwrap();

    // Should find the actual files
    assert!(files.iter().any(|p| p.ends_with("target.rs")));
    assert!(files.iter().any(|p| p.ends_with("inner.rs")));
}

#[tokio::test]
async fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    let ignore = Gitignore::empty();
    let traverser = FileTraverser::new(&ignore);

    let files = traverser
        .collect_files_for_focused(dir_path, 0)
        .await
        .unwrap();

    assert_eq!(files.len(), 0);
}

#[tokio::test]
async fn test_gitignore_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create files that should be ignored
    fs::write(dir_path.join("test.log"), "log").unwrap();
    fs::write(dir_path.join("debug.log"), "debug").unwrap();
    fs::write(dir_path.join("test.rs"), "fn main() {}").unwrap();

    // Create node_modules directory with files
    let node_modules = dir_path.join("node_modules");
    fs::create_dir(&node_modules).unwrap();
    fs::write(node_modules.join("package.json"), "{}").unwrap();

    let ignore = create_test_gitignore_at(dir_path);
    let traverser = FileTraverser::new(&ignore);

    let files = traverser
        .collect_files_for_focused(dir_path, 0)
        .await
        .unwrap();

    // Should only find test.rs, not the .log files or node_modules content
    assert_eq!(files.len(), 1);
    assert!(files[0].ends_with("test.rs"));
}
