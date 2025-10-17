use super::fixtures::create_test_gitignore;
use crate::developer::analyze::{types::AnalyzeParams, CodeAnalyzer};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_large_output_warning() {
    let analyzer = CodeAnalyzer::new();
    let gitignore = create_test_gitignore();

    // Create a temp directory with many files to trigger the warning
    let temp_dir = TempDir::new().unwrap();

    // Create many Python files with lots of functions to ensure we exceed 1000 lines
    // Each file generates about 1 line in structure mode, so we need 1000+ files
    for i in 0..1100 {
        let file_path = temp_dir.path().join(format!("file{}.py", i));
        // Each file will have multiple functions to generate more output
        let mut content = String::new();
        for j in 0..10 {
            content.push_str(&format!("def function_{}_{}():\n    pass\n\n", i, j));
        }
        for j in 0..5 {
            content.push_str(&format!(
                "class Class_{}_{}:\n    def method(self):\n        pass\n\n",
                i, j
            ));
        }
        fs::write(&file_path, content).unwrap();
    }

    let params = AnalyzeParams {
        path: temp_dir.path().to_str().unwrap().to_string(),
        focus: None,
        follow_depth: 2,
        max_depth: 3,
        ast_recursion_limit: None,
        force: false, // Should trigger warning
    };

    let result = analyzer
        .analyze(params, temp_dir.path().to_path_buf(), &gitignore)
        .unwrap();

    // Check that we got a warning, not the actual analysis
    assert_eq!(result.content.len(), 1);
    if let Some(text_content) = result.content[0].as_text() {
        assert!(text_content.text.contains("LARGE OUTPUT WARNING"));
        assert!(text_content.text.contains("force=true"));
        assert!(text_content.text.contains("exceed"));
    } else {
        panic!("Expected text content");
    }
}

#[test]
fn test_force_flag_bypasses_warning() {
    let analyzer = CodeAnalyzer::new();
    let gitignore = create_test_gitignore();

    // Create a temp directory with many files
    let temp_dir = TempDir::new().unwrap();

    // Create many Python files with lots of functions to ensure we exceed 1000 lines
    for i in 0..50 {
        let file_path = temp_dir.path().join(format!("file{}.py", i));
        // Each file will have multiple functions to generate more output
        let mut content = String::new();
        for j in 0..10 {
            content.push_str(&format!("def function_{}_{}():\n    pass\n\n", i, j));
        }
        for j in 0..5 {
            content.push_str(&format!(
                "class Class_{}_{}:\n    def method(self):\n        pass\n\n",
                i, j
            ));
        }
        fs::write(&file_path, content).unwrap();
    }

    let params = AnalyzeParams {
        path: temp_dir.path().to_str().unwrap().to_string(),
        focus: None,
        follow_depth: 2,
        max_depth: 3,
        ast_recursion_limit: None,
        force: true, // Should bypass warning
    };

    let result = analyzer
        .analyze(params, temp_dir.path().to_path_buf(), &gitignore)
        .unwrap();

    // Check that we got the actual analysis, not a warning
    if let Some(text_content) = result.content[0].as_text() {
        assert!(!text_content.text.contains("LARGE OUTPUT WARNING"));
        // Should contain actual file analysis
        assert!(text_content.text.contains("file0.py"));
        assert!(text_content.text.contains("file29.py"));
    } else {
        panic!("Expected text content");
    }
}

#[test]
fn test_small_output_no_warning() {
    let analyzer = CodeAnalyzer::new();
    let gitignore = create_test_gitignore();

    // Create a temp directory with just a few files
    let temp_dir = TempDir::new().unwrap();

    // Create only 2 Python files - should not trigger warning
    for i in 0..2 {
        let file_path = temp_dir.path().join(format!("file{}.py", i));
        fs::write(&file_path, format!("def function_{}():\n    pass\n", i)).unwrap();
    }

    let params = AnalyzeParams {
        path: temp_dir.path().to_str().unwrap().to_string(),
        focus: None,
        follow_depth: 2,
        max_depth: 3,
        ast_recursion_limit: None,
        force: false, // Shouldn't matter for small output
    };

    let result = analyzer
        .analyze(params, temp_dir.path().to_path_buf(), &gitignore)
        .unwrap();

    // Check that we got the actual analysis, not a warning
    if let Some(text_content) = result.content[0].as_text() {
        assert!(!text_content.text.contains("LARGE OUTPUT WARNING"));
        assert!(text_content.text.contains("file0.py"));
        assert!(text_content.text.contains("file1.py"));
    } else {
        panic!("Expected text content");
    }
}
