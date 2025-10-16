// Integration tests for the analyze module

use crate::developer::analyze::tests::fixtures::create_test_gitignore;
use crate::developer::analyze::{types::AnalyzeParams, CodeAnalyzer};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_analyze_python_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");
    fs::write(&file_path, "def main():\n    pass").unwrap();

    let analyzer = CodeAnalyzer::new();
    let params = AnalyzeParams {
        path: file_path.to_string_lossy().to_string(),
        focus: None,
        follow_depth: 2,
        max_depth: 3,
        ast_recursion_limit: None,
        force: false,
    };

    let ignore = create_test_gitignore();
    let result = analyzer.analyze(params, file_path, &ignore);

    assert!(result.is_ok());
    let result = result.unwrap();

    // Check that we got content back
    assert!(!result.content.is_empty());
}

#[test]
fn test_analyze_directory() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create test files
    fs::write(dir_path.join("test1.rs"), "fn main() {}").unwrap();
    fs::write(dir_path.join("test2.py"), "def test(): pass").unwrap();

    let analyzer = CodeAnalyzer::new();
    let params = AnalyzeParams {
        path: dir_path.to_string_lossy().to_string(),
        focus: None,
        follow_depth: 2,
        max_depth: 3,
        ast_recursion_limit: None,
        force: false,
    };

    let ignore = create_test_gitignore();
    let result = analyzer.analyze(params, dir_path.to_path_buf(), &ignore);

    assert!(result.is_ok());
    let result = result.unwrap();

    // Check that we got content back
    assert!(!result.content.is_empty());

    // Extract text content and verify it contains expected information
    if let Some(text_content) = result.content[0].as_text() {
        assert!(text_content.text.contains("SUMMARY:"));
        assert!(text_content.text.contains("test1.rs"));
        assert!(text_content.text.contains("test2.py"));
    }
}

#[test]
fn test_focused_analysis() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");
    fs::write(
        &file_path,
        "def main():\n    helper()\n\ndef helper():\n    pass",
    )
    .unwrap();

    let analyzer = CodeAnalyzer::new();
    let params = AnalyzeParams {
        path: file_path.to_string_lossy().to_string(),
        focus: Some("helper".to_string()),
        follow_depth: 1,
        max_depth: 3,
        ast_recursion_limit: None,
        force: false,
    };

    let ignore = create_test_gitignore();
    let result = analyzer.analyze(params, file_path, &ignore);

    assert!(result.is_ok());
    let result = result.unwrap();

    // Check that focused analysis output is generated
    if let Some(text_content) = result.content[0].as_text() {
        assert!(text_content.text.contains("FOCUSED ANALYSIS: helper"));
        assert!(text_content.text.contains("DEFINITIONS:"));
    }
}

#[test]
fn test_analyze_with_cache() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.rs");
    fs::write(&file_path, "fn main() {\n    println!(\"Hello\");\n}").unwrap();

    let analyzer = CodeAnalyzer::new();
    let params = AnalyzeParams {
        path: file_path.to_string_lossy().to_string(),
        focus: None,
        follow_depth: 2,
        max_depth: 3,
        ast_recursion_limit: None,
        force: false,
    };

    let ignore = create_test_gitignore();

    // First analysis - should cache
    let result1 = analyzer.analyze(params.clone(), file_path.clone(), &ignore);
    assert!(result1.is_ok());

    // Second analysis - should use cache
    let result2 = analyzer.analyze(params, file_path, &ignore);
    assert!(result2.is_ok());

    // Results should be identical
    let content1 = result1.unwrap().content[0].as_text().unwrap().text.clone();
    let content2 = result2.unwrap().content[0].as_text().unwrap().text.clone();
    assert_eq!(content1, content2);
}

#[test]
fn test_analyze_unsupported_file() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.txt");
    fs::write(&file_path, "This is not code").unwrap();

    let analyzer = CodeAnalyzer::new();
    let params = AnalyzeParams {
        path: file_path.to_string_lossy().to_string(),
        focus: None,
        follow_depth: 2,
        max_depth: 3,
        ast_recursion_limit: None,
        force: false,
    };

    let ignore = create_test_gitignore();
    let result = analyzer.analyze(params, file_path, &ignore);

    // Should succeed but return minimal information
    assert!(result.is_ok());
}

#[test]
fn test_analyze_nonexistent_path() {
    let analyzer = CodeAnalyzer::new();
    let params = AnalyzeParams {
        path: "/nonexistent/path".to_string(),
        focus: None,
        follow_depth: 2,
        max_depth: 3,
        ast_recursion_limit: None,
        force: false,
    };

    let ignore = create_test_gitignore();
    let result = analyzer.analyze(params, "/nonexistent/path".into(), &ignore);

    // Should return an error
    assert!(result.is_err());
}

#[test]
fn test_focused_without_symbol() {
    let temp_dir = TempDir::new().unwrap();
    let file_path = temp_dir.path().join("test.py");
    fs::write(&file_path, "def main(): pass").unwrap();

    let analyzer = CodeAnalyzer::new();

    // This should trigger focused mode due to having focus parameter
    let params = AnalyzeParams {
        path: file_path.to_string_lossy().to_string(),
        focus: Some("nonexistent_symbol".to_string()),
        follow_depth: 1,
        max_depth: 3,
        ast_recursion_limit: None,
        force: false,
    };

    let ignore = create_test_gitignore();
    let result = analyzer.analyze(params, file_path, &ignore);

    assert!(result.is_ok());
    let result = result.unwrap();

    // Should indicate symbol not found
    if let Some(text_content) = result.content[0].as_text() {
        assert!(text_content
            .text
            .contains("Symbol 'nonexistent_symbol' not found"));
    }
}

#[test]
fn test_nested_directory_analysis() {
    let temp_dir = TempDir::new().unwrap();
    let dir_path = temp_dir.path();

    // Create nested structure
    let src_dir = dir_path.join("src");
    fs::create_dir(&src_dir).unwrap();
    fs::write(src_dir.join("main.rs"), "fn main() {}").unwrap();

    let lib_dir = src_dir.join("lib");
    fs::create_dir(&lib_dir).unwrap();
    fs::write(lib_dir.join("utils.rs"), "pub fn util() {}").unwrap();

    let analyzer = CodeAnalyzer::new();
    let params = AnalyzeParams {
        path: dir_path.to_string_lossy().to_string(),
        focus: None,
        follow_depth: 2,
        max_depth: 3, // Increase max_depth to ensure we reach nested files
        ast_recursion_limit: None,
        force: false,
    };

    let ignore = create_test_gitignore();
    let result = analyzer.analyze(params, dir_path.to_path_buf(), &ignore);

    assert!(result.is_ok());
    let result = result.unwrap();

    if let Some(text_content) = result.content[0].as_text() {
        assert!(text_content.text.contains("main.rs"));
        // The directory structure analysis should show both files
        assert!(text_content.text.contains("src"));
    }
}
