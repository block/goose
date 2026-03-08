// Shared test fixtures and utilities

use crate::developer::analyze::types::{AnalysisResult, CallInfo, ClassInfo, FunctionInfo};
use ignore::gitignore::Gitignore;

/// Create a test AnalysisResult with sample data
pub fn create_test_result() -> AnalysisResult {
    AnalysisResult {
        functions: vec![
            FunctionInfo {
                name: "main".to_string(),
                line: 10,
                params: vec![],
            },
            FunctionInfo {
                name: "helper".to_string(),
                line: 20,
                params: vec![],
            },
        ],
        classes: vec![ClassInfo {
            name: "TestClass".to_string(),
            line: 5,
            methods: vec![],
        }],
        imports: vec!["use std::fs".to_string()],
        calls: vec![],
        references: vec![],
        function_count: 2,
        class_count: 1,
        line_count: 100,
        import_count: 1,
        main_line: Some(10),
    }
}

/// Create a test result with specific functions and call relationships
pub fn create_test_result_with_calls(
    functions: Vec<&str>,
    calls: Vec<(&str, &str)>,
) -> AnalysisResult {
    AnalysisResult {
        functions: functions
            .into_iter()
            .map(|name| FunctionInfo {
                name: name.to_string(),
                line: 1,
                params: vec![],
            })
            .collect(),
        classes: vec![],
        imports: vec![],
        calls: calls
            .into_iter()
            .map(|(caller, callee)| CallInfo {
                caller_name: Some(caller.to_string()),
                callee_name: callee.to_string(),
                line: 1,
                column: 0,
                context: String::new(),
            })
            .collect(),
        references: vec![],
        function_count: 0,
        class_count: 0,
        line_count: 0,
        import_count: 0,
        main_line: None,
    }
}

/// Create a simple test gitignore
pub fn create_test_gitignore() -> Gitignore {
    let mut builder = ignore::gitignore::GitignoreBuilder::new(".");
    builder.add_line(None, "*.log").unwrap();
    builder.add_line(None, "node_modules/").unwrap();
    builder.build().unwrap()
}

/// Create a test gitignore with custom base path
#[allow(dead_code)]
pub fn create_test_gitignore_at(base_path: &std::path::Path) -> Gitignore {
    let mut builder = ignore::gitignore::GitignoreBuilder::new(base_path);
    builder.add_line(None, "*.log").unwrap();
    builder.add_line(None, "node_modules/").unwrap();
    builder.build().unwrap()
}
