// Tests for the traversal module

use crate::developer::analyze::tests::fixtures::create_test_gitignore;
use crate::developer::analyze::traversal::FileTraverser;
use ignore::gitignore::Gitignore;
use std::fs;
use std::path::Path;
use tempfile::TempDir;

#[test]
fn test_is_ignored() {
    // Create a temporary directory for testing
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create actual files and directories to test
    fs::write(dir_path.join("test.log"), "log content").unwrap();
    fs::write(dir_path.join("test.rs"), "fn main() {}").unwrap();

    // Create gitignore that ignores .log files
    let mut builder = ignore::gitignore::GitignoreBuilder::new(dir_path);
    builder.add_line(None, "*.log").unwrap();
    let ignore = builder.build().unwrap();

    let traverser = FileTraverser::new(&ignore);

    // Test that .log files are ignored and .rs files are not
    assert!(traverser.is_ignored(&dir_path.join("test.log")));
    assert!(!traverser.is_ignored(&dir_path.join("test.rs")));
}

#[test]
fn test_validate_path() {
    let ignore = create_test_gitignore();
    let traverser = FileTraverser::new(&ignore);

    // Test non-existent path
    assert!(traverser
        .validate_path(Path::new("/nonexistent/path"))
        .is_err());

    // Test ignored path
    assert!(traverser.validate_path(Path::new("test.log")).is_err());
}

#[test]
fn test_collect_files() {
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

    let files = traverser.collect_files_for_focused(dir_path, 0).unwrap();

    // Should find .rs and .py files but not .txt
    assert_eq!(files.len(), 3);
    assert!(files.iter().any(|p| p.ends_with("test.rs")));
    assert!(files.iter().any(|p| p.ends_with("test.py")));
    assert!(files.iter().any(|p| p.ends_with("lib.rs")));
}

#[test]
fn test_max_depth() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create nested structure
    fs::write(dir_path.join("root.rs"), "").unwrap();

    let level1 = dir_path.join("level1");
    fs::create_dir(&level1).unwrap();
    fs::write(level1.join("file1.rs"), "").unwrap();

    let level2 = level1.join("level2");
    fs::create_dir(&level2).unwrap();
    fs::write(level2.join("file2.rs"), "").unwrap();

    let level3 = level2.join("level3");
    fs::create_dir(&level3).unwrap();
    fs::write(level3.join("file3.rs"), "").unwrap();

    let ignore = Gitignore::empty();
    let traverser = FileTraverser::new(&ignore);

    // Test that limiting depth works - exact counts may vary based on implementation
    // The important thing is that deeper files are excluded with lower max_depth

    // With a small max_depth, we should find fewer files
    let files_limited = traverser.collect_files_for_focused(dir_path, 2).unwrap();

    // With unlimited depth, we should find all files
    let files_unlimited = traverser.collect_files_for_focused(dir_path, 0).unwrap();

    // The unlimited search should find more files than the limited one
    assert!(
        files_unlimited.len() > files_limited.len(),
        "Unlimited depth should find more files than limited depth"
    );

    // Should always find the root file
    assert!(files_unlimited.iter().any(|p| p.ends_with("root.rs")));

    // With unlimited, should find all 4 files
    assert_eq!(
        files_unlimited.len(),
        4,
        "Should find all 4 files with unlimited depth"
    );
}

#[test]
fn test_symlink_handling() {
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
        let _ = symlink(dir_path.join("target.rs"), dir_path.join("link.rs"));
        let _ = symlink(&target_dir, dir_path.join("link_dir"));
    }

    let ignore = Gitignore::empty();
    let traverser = FileTraverser::new(&ignore);

    // Collect files - symlinks should be handled appropriately
    let files = traverser.collect_files_for_focused(dir_path, 0).unwrap();

    // Should find the actual files
    assert!(files.iter().any(|p| p.ends_with("target.rs")));
    assert!(files.iter().any(|p| p.ends_with("inner.rs")));
}

#[test]
fn test_empty_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    let ignore = Gitignore::empty();
    let traverser = FileTraverser::new(&ignore);

    let files = traverser.collect_files_for_focused(dir_path, 0).unwrap();

    assert_eq!(files.len(), 0);
}

#[test]
fn test_gitignore_patterns() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create files
    fs::write(dir_path.join("test.log"), "log").unwrap();
    fs::write(dir_path.join("debug.log"), "debug").unwrap();
    fs::write(dir_path.join("test.rs"), "fn main() {}").unwrap();
    fs::write(dir_path.join("main.py"), "def main(): pass").unwrap();

    // Create gitignore that only ignores .log files
    let mut builder = ignore::gitignore::GitignoreBuilder::new(dir_path);
    builder.add_line(None, "*.log").unwrap();
    let ignore = builder.build().unwrap();

    let traverser = FileTraverser::new(&ignore);

    let files = traverser.collect_files_for_focused(dir_path, 0).unwrap();

    // Should find .rs and .py files, but not .log files
    assert_eq!(files.len(), 2, "Should find 2 non-log files");
    assert!(files.iter().any(|p| p.ends_with("test.rs")));
    assert!(files.iter().any(|p| p.ends_with("main.py")));
    assert!(!files.iter().any(|p| p.ends_with(".log")));
}
