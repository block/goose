//! Language-specific analysis implementations
//!
//! This module contains language-specific parsing logic and tree-sitter queries
//! for the analyze tool. Each language has its own submodule with query definitions
//! and optional helper functions.
//!
//! ## Adding a New Language
//!
//! To add support for a new language:
//!
//! 1. Create a new file `languages/yourlang.rs`
//! 2. Define `ELEMENT_QUERY` and `CALL_QUERY` constants
//! 3. Optionally define `REFERENCE_QUERY` for advanced type tracking
//! 4. Add `pub mod yourlang;` below
//! 5. Add language configuration to registry in `get_language_info()`
//!
//! ## Optional Features
//!
//! Languages can opt into additional features by implementing:
//!
//! - Reference tracking: Define `REFERENCE_QUERY` to track type instantiation,
//!   field types, and method-to-type associations (see Go and Ruby)
//! - Custom function naming: Implement `extract_function_name_for_kind()` for
//!   special cases like Swift's init/deinit or Rust's impl blocks
//! - Method receiver lookup: Implement `find_method_for_receiver()` to associate
//!   methods with their containing types (see Go and Ruby)

pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod swift;

/// Handler for extracting function names from special node kinds
type ExtractFunctionNameHandler = fn(&tree_sitter::Node, &str, &str) -> Option<String>;

/// Handler for finding method names from receiver nodes
/// Takes: (receiver_node, source, ast_recursion_limit)
type FindMethodForReceiverHandler = fn(&tree_sitter::Node, &str, Option<usize>) -> Option<String>;

/// Handler for finding the receiver type from a receiver node
/// Takes: (receiver_node, source)
type FindReceiverTypeHandler = fn(&tree_sitter::Node, &str) -> Option<String>;

/// Language configuration containing all language-specific information
///
/// This struct serves as a single source of truth for language support.
/// All language-specific queries and handlers are defined here.
#[derive(Copy, Clone)]
pub struct LanguageInfo {
    /// Tree-sitter query for extracting code elements (functions, classes, imports)
    pub element_query: &'static str,
    /// Tree-sitter query for extracting function calls
    pub call_query: &'static str,
    /// Tree-sitter query for extracting type references (optional)
    pub reference_query: &'static str,
    /// Node kinds that represent function-like constructs
    pub function_node_kinds: &'static [&'static str],
    /// Node kinds that represent function name identifiers
    pub function_name_kinds: &'static [&'static str],
    /// Optional handler for language-specific function name extraction
    pub extract_function_name_handler: Option<ExtractFunctionNameHandler>,
    /// Optional handler for finding method names from receiver nodes
    pub find_method_for_receiver_handler: Option<FindMethodForReceiverHandler>,
    /// Optional handler for finding receiver type from receiver nodes
    pub find_receiver_type_handler: Option<FindReceiverTypeHandler>,
}

/// Get language configuration for a given language
///
/// Returns `Some(LanguageInfo)` if the language is supported, `None` otherwise.
pub fn get_language_info(language: &str) -> Option<LanguageInfo> {
    match language {
        "python" => Some(LanguageInfo {
            element_query: python::ELEMENT_QUERY,
            call_query: python::CALL_QUERY,
            reference_query: "",
            function_node_kinds: &["function_definition"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: None,
            find_receiver_type_handler: None,
        }),
        "rust" => Some(LanguageInfo {
            element_query: rust::ELEMENT_QUERY,
            call_query: rust::CALL_QUERY,
            reference_query: rust::REFERENCE_QUERY,
            function_node_kinds: &["function_item", "impl_item"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: Some(rust::extract_function_name_for_kind),
            find_method_for_receiver_handler: Some(rust::find_method_for_receiver),
            find_receiver_type_handler: Some(rust::find_receiver_type),
        }),
        "javascript" | "typescript" => Some(LanguageInfo {
            element_query: javascript::ELEMENT_QUERY,
            call_query: javascript::CALL_QUERY,
            reference_query: "",
            function_node_kinds: &[
                "function_declaration",
                "method_definition",
                "arrow_function",
            ],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: None,
            find_receiver_type_handler: None,
        }),
        "go" => Some(LanguageInfo {
            element_query: go::ELEMENT_QUERY,
            call_query: go::CALL_QUERY,
            reference_query: go::REFERENCE_QUERY,
            function_node_kinds: &["function_declaration", "method_declaration"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: Some(go::find_method_for_receiver),
            find_receiver_type_handler: None,
        }),
        "java" => Some(LanguageInfo {
            element_query: java::ELEMENT_QUERY,
            call_query: java::CALL_QUERY,
            reference_query: "",
            function_node_kinds: &["method_declaration", "constructor_declaration"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: None,
            find_receiver_type_handler: None,
        }),
        "kotlin" => Some(LanguageInfo {
            element_query: kotlin::ELEMENT_QUERY,
            call_query: kotlin::CALL_QUERY,
            reference_query: "",
            function_node_kinds: &["function_declaration", "class_body"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: None,
            find_receiver_type_handler: None,
        }),
        "swift" => Some(LanguageInfo {
            element_query: swift::ELEMENT_QUERY,
            call_query: swift::CALL_QUERY,
            reference_query: "",
            function_node_kinds: &[
                "function_declaration",
                "init_declaration",
                "deinit_declaration",
                "subscript_declaration",
            ],
            function_name_kinds: &["simple_identifier"],
            extract_function_name_handler: Some(swift::extract_function_name_for_kind),
            find_method_for_receiver_handler: None,
            find_receiver_type_handler: None,
        }),
        "ruby" => Some(LanguageInfo {
            element_query: ruby::ELEMENT_QUERY,
            call_query: ruby::CALL_QUERY,
            reference_query: ruby::REFERENCE_QUERY,
            function_node_kinds: &["method", "singleton_method"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: Some(ruby::find_method_for_receiver),
            find_receiver_type_handler: None,
        }),
        _ => None,
    }
}
