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
        ElementExtractor::extract_with_depth(&tree, content, "python", "structure").unwrap();

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
        ElementExtractor::extract_with_depth(&tree, content, "python", "semantic").unwrap();

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
