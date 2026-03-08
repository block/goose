// Tests for the parser module

use crate::developer::analyze::parser::{ElementExtractor, ParserManager};
use std::sync::Arc;

#[test]
fn test_parser_initialization() {
    let manager = ParserManager::new();
    assert!(manager.get_or_create_parser("python").is_ok());
    assert!(manager.get_or_create_parser("rust").is_ok());
    assert!(manager.get_or_create_parser("unknown").is_err());
}

#[test]
fn test_parser_caching() {
    let manager = ParserManager::new();

    // First call creates parser
    let parser1 = manager.get_or_create_parser("python").unwrap();

    // Second call should return cached parser
    let parser2 = manager.get_or_create_parser("python").unwrap();

    // They should be the same Arc
    assert!(Arc::ptr_eq(&parser1, &parser2));
}

#[test]
fn test_parse_python() {
    let manager = ParserManager::new();
    let content = "def hello():\n    pass";

    let tree = manager.parse(content, "python").unwrap();
    assert!(tree.root_node().child_count() > 0);
}

#[test]
fn test_parse_rust() {
    let manager = ParserManager::new();
    let content = "fn main() {\n    println!(\"Hello\");\n}";

    let tree = manager.parse(content, "rust").unwrap();
    assert!(tree.root_node().child_count() > 0);
}

#[test]
fn test_parse_javascript() {
    let manager = ParserManager::new();
    let content = "function hello() {\n    console.log('Hello');\n}";

    let tree = manager.parse(content, "javascript").unwrap();
    assert!(tree.root_node().child_count() > 0);
}

#[test]
fn test_extract_python_elements() {
    let manager = ParserManager::new();
    let content = r#"
import os

class MyClass:
    def method(self):
        pass

def main():
    print("hello")
"#;

    let tree = manager.parse(content, "python").unwrap();
    let result = ElementExtractor::extract_elements(&tree, content, "python").unwrap();

    assert_eq!(result.function_count, 2); // main and method
    assert_eq!(result.class_count, 1); // MyClass
    assert_eq!(result.import_count, 1); // import os
    assert!(result.main_line.is_some());
}

#[test]
fn test_extract_rust_elements() {
    let manager = ParserManager::new();
    let content = r#"
use std::fs;

struct MyStruct {
    field: i32,
}

impl MyStruct {
    fn new() -> Self {
        Self { field: 0 }
    }
}

fn main() {
    let s = MyStruct::new();
}
"#;

    let tree = manager.parse(content, "rust").unwrap();
    let result = ElementExtractor::extract_elements(&tree, content, "rust").unwrap();

    assert_eq!(result.function_count, 2); // main and new
    assert_eq!(result.class_count, 2); // MyStruct (struct) and MyStruct (impl)
    assert_eq!(result.import_count, 1); // use std::fs
    assert!(result.main_line.is_some());
}

#[test]
fn test_extract_with_depth_structure() {
    let manager = ParserManager::new();
    let content = r#"
def func1():
    pass

def func2():
    func1()
"#;

    let tree = manager.parse(content, "python").unwrap();
    let result =
        ElementExtractor::extract_with_depth(&tree, content, "python", "structure", None).unwrap();

    // In structure mode, detailed vectors should be empty but counts preserved
    assert_eq!(result.function_count, 2);
    assert!(result.functions.is_empty());
    assert!(result.calls.is_empty());
}

#[test]
fn test_extract_with_depth_semantic() {
    let manager = ParserManager::new();
    let content = r#"
def func1():
    pass

def func2():
    func1()
"#;

    let tree = manager.parse(content, "python").unwrap();
    let result =
        ElementExtractor::extract_with_depth(&tree, content, "python", "semantic", None).unwrap();

    // In semantic mode, should have both elements and calls
    assert_eq!(result.function_count, 2);
    assert_eq!(result.functions.len(), 2);
    assert!(!result.calls.is_empty());
    assert_eq!(result.calls[0].callee_name, "func1");
}

#[test]
fn test_parse_invalid_syntax() {
    let manager = ParserManager::new();
    let content = "def invalid syntax here";

    // Should still parse (tree-sitter is error-tolerant)
    let tree = manager.parse(content, "python");
    assert!(tree.is_ok());
}

#[test]
fn test_multiple_languages() {
    let manager = ParserManager::new();

    // Test that we can handle multiple languages in the same manager
    assert!(manager.get_or_create_parser("python").is_ok());
    assert!(manager.get_or_create_parser("rust").is_ok());
    assert!(manager.get_or_create_parser("javascript").is_ok());
    assert!(manager.get_or_create_parser("go").is_ok());
    assert!(manager.get_or_create_parser("java").is_ok());
    assert!(manager.get_or_create_parser("kotlin").is_ok());
}

#[test]
fn test_parse_kotlin() {
    let manager = ParserManager::new();
    let content = r#"
package com.example

import kotlin.math.*

class Example(val name: String) {
    fun greet() {
        println("Hello, $name")
    }
}

fun main() {
    val example = Example("World")
    example.greet()
}
"#;

    let tree = manager.parse(content, "kotlin").unwrap();
    assert!(tree.root_node().child_count() > 0);
}

#[test]
fn test_extract_kotlin_elements() {
    let manager = ParserManager::new();
    let content = r#"
package com.example

import kotlin.math.*

class MyClass {
    fun method() {
        println("method")
    }
}

fun main() {
    println("hello")
}

fun helper() {
    main()
}
"#;

    let tree = manager.parse(content, "kotlin").unwrap();
    let result = ElementExtractor::extract_elements(&tree, content, "kotlin").unwrap();

    assert_eq!(result.function_count, 3); // main, helper, method
    assert_eq!(result.class_count, 1); // MyClass
    assert!(result.import_count > 0); // import statements
    assert!(result.main_line.is_some());
}

#[test]
fn test_language_registry() {
    use crate::developer::analyze::languages;

    let supported = vec![
        "python",
        "rust",
        "javascript",
        "typescript",
        "go",
        "java",
        "kotlin",
        "swift",
        "ruby",
    ];

    for lang in supported {
        let info = languages::get_language_info(lang);
        assert!(info.is_some(), "Language {} should be supported", lang);

        let info = info.unwrap();
        assert!(
            !info.element_query.is_empty(),
            "{} missing element_query",
            lang
        );
        assert!(!info.call_query.is_empty(), "{} missing call_query", lang);
        assert!(
            !info.function_node_kinds.is_empty(),
            "{} missing function_node_kinds",
            lang
        );
        assert!(
            !info.function_name_kinds.is_empty(),
            "{} missing function_name_kinds",
            lang
        );
    }

    let js = languages::get_language_info("javascript").unwrap();
    let ts = languages::get_language_info("typescript").unwrap();
    assert_eq!(
        js.element_query, ts.element_query,
        "JS/TS should share config"
    );

    let go = languages::get_language_info("go").unwrap();
    assert!(
        !go.reference_query.is_empty(),
        "Go should have reference tracking"
    );
    assert!(go.find_method_for_receiver_handler.is_some());

    let ruby = languages::get_language_info("ruby").unwrap();
    assert!(
        !ruby.reference_query.is_empty(),
        "Ruby should have reference tracking"
    );
    assert!(ruby.find_method_for_receiver_handler.is_some());

    let rust = languages::get_language_info("rust").unwrap();
    assert!(
        rust.extract_function_name_handler.is_some(),
        "Rust should have custom handler"
    );

    let swift = languages::get_language_info("swift").unwrap();
    assert!(
        swift.extract_function_name_handler.is_some(),
        "Swift should have custom handler"
    );

    assert!(languages::get_language_info("unsupported").is_none());
    assert!(languages::get_language_info("").is_none());
    assert!(languages::get_language_info("C++").is_none());
}
