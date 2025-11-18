// Tests for the formatter module

use crate::developer::analyze::formatter::Formatter;
use crate::developer::analyze::tests::fixtures::create_test_result;
use crate::developer::analyze::types::{AnalysisMode, CallChain, EntryType, FocusedAnalysisData};
use std::path::{Path, PathBuf};

#[test]
fn test_format_structure_overview() {
    let result = create_test_result();
    let output = Formatter::format_structure_overview(Path::new("test.rs"), &result);

    assert!(output.contains("[100L, 2F, 1C]"));
    assert!(output.contains("main:10"));
}

#[test]
fn test_format_semantic_result() {
    let result = create_test_result();
    let output = Formatter::format_semantic_result(Path::new("test.rs"), &result);

    assert!(output.contains("FILE: test.rs"));
    assert!(output.contains("C: TestClass:5"));
    assert!(output.contains("F: main:10 helper:20"));
    assert!(output.contains("I: use std::fs"));
}

#[test]
fn test_filter_by_focus() {
    // The filter_by_focus function includes the whole section when it finds a match
    // This is the expected behavior - if a symbol is found in a file, show the whole file section
    let output = "## test.rs\nfunction main at line 10\nfunction helper at line 20\n## other.rs\nfunction foo at line 5\n";
    let filtered = Formatter::filter_by_focus(output, "main");

    assert!(filtered.contains("main"));
    // When we find 'main' in test.rs, we include the whole test.rs section including 'helper'
    assert!(filtered.contains("helper"));
    assert!(!filtered.contains("foo")); // But we don't include other.rs
}

#[test]
fn test_format_analysis_result_modes() {
    let result = create_test_result();
    let path = Path::new("test.rs");

    // Test structure mode
    let output = Formatter::format_analysis_result(path, &result, &AnalysisMode::Structure);
    assert!(output.contains("[100L, 2F, 1C]"));

    // Test semantic mode
    let output = Formatter::format_analysis_result(path, &result, &AnalysisMode::Semantic);
    assert!(output.contains("FILE: test.rs"));
    assert!(output.contains("C: TestClass:5"));

    // Test focused mode (should return empty string with warning)
    let output = Formatter::format_analysis_result(path, &result, &AnalysisMode::Focused);
    assert_eq!(output, "");
}

#[test]
fn test_format_directory_structure() {
    let base_path = Path::new("/test");
    let result1 = create_test_result();
    let mut result2 = create_test_result();
    result2.line_count = 200;

    let results = vec![
        (PathBuf::from("/test/file1.rs"), EntryType::File(result1)),
        (PathBuf::from("/test/dir"), EntryType::Directory),
        (
            PathBuf::from("/test/dir/file2.rs"),
            EntryType::File(result2),
        ),
    ];

    let output = Formatter::format_directory_structure(base_path, &results, 2);

    // Check summary
    assert!(output.contains("SUMMARY:"));
    assert!(output.contains("2 files, 300L, 4F, 2C"));
    assert!(output.contains("Languages: rust (100%)"));

    // Check file entries
    assert!(output.contains("file1.rs [100L, 2F, 1C]"));
    assert!(output.contains("file2.rs [200L, 2F, 1C]"));
}

#[test]
fn test_format_focused_output() {
    let focus_data = FocusedAnalysisData {
        focus_symbol: "test_func",
        definitions: &[(PathBuf::from("test.rs"), 10)],
        incoming_chains: &[CallChain {
            path: vec![(
                PathBuf::from("test.rs"),
                20,
                "caller".to_string(),
                "test_func".to_string(),
            )],
        }],
        outgoing_chains: &[CallChain {
            path: vec![(
                PathBuf::from("test.rs"),
                30,
                "test_func".to_string(),
                "callee".to_string(),
            )],
        }],
        files_analyzed: &[PathBuf::from("test.rs")],
        follow_depth: 2,
    };

    let output = Formatter::format_focused_output(&focus_data);

    assert!(output.contains("FOCUSED ANALYSIS: test_func"));
    assert!(output.contains("DEFINITIONS:"));
    assert!(output.contains("INCOMING CALL CHAINS"));
    assert!(output.contains("OUTGOING CALL CHAINS"));
    assert!(output.contains("STATISTICS:"));
}

#[test]
fn test_format_focused_output_empty() {
    let focus_data = FocusedAnalysisData {
        focus_symbol: "nonexistent",
        definitions: &[],
        incoming_chains: &[],
        outgoing_chains: &[],
        files_analyzed: &[PathBuf::from("test.rs")],
        follow_depth: 2,
    };

    let output = Formatter::format_focused_output(&focus_data);

    assert!(output.contains("Symbol 'nonexistent' not found"));
}

#[test]
fn test_format_results_wrapper() {
    let text = "Test output";
    let contents = Formatter::format_results(text.to_string());

    assert_eq!(contents.len(), 2);

    // Check that both assistant and user content are created
    let assistant_content = contents[0].as_text().unwrap();
    assert_eq!(assistant_content.text, "Test output");

    let user_content = contents[1].as_text().unwrap();
    assert_eq!(user_content.text, "Test output");
}
