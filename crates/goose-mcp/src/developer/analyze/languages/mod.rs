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
//! 5. **Add a single case to `get_language_info()`** - that's it!
//!
//! ## Optional Features
//!
//! Languages can opt into additional features by implementing:
//!
//! - **Reference tracking**: Define `REFERENCE_QUERY` to track type instantiation,
//!   field types, and method-to-type associations (see Go and Ruby)
//! - **Custom function naming**: Implement `extract_function_name_for_kind()` for
//!   special cases like Swift's init/deinit or Rust's impl blocks
//! - **Method receiver lookup**: Implement `find_method_for_receiver()` to associate
//!   methods with their containing types (see Go and Ruby)

pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod python;
pub mod ruby;
pub mod rust;
pub mod swift;

/// Language configuration containing all language-specific information
///
/// This struct serves as a single source of truth for language support.
/// All language-specific queries and handlers are defined here.
struct LanguageInfo {
    /// Tree-sitter query for extracting code elements (functions, classes, imports)
    element_query: &'static str,
    /// Tree-sitter query for extracting function calls
    call_query: &'static str,
    /// Tree-sitter query for extracting type references (optional)
    reference_query: &'static str,
    /// Node kinds that represent function-like constructs
    function_node_kinds: &'static [&'static str],
    /// Node kinds that represent function name identifiers
    function_name_kinds: &'static [&'static str],
    /// Optional handler for language-specific function name extraction
    extract_function_name_handler: Option<fn(&tree_sitter::Node, &str, &str) -> Option<String>>,
    /// Optional handler for finding method names from receiver nodes
    find_method_for_receiver_handler: Option<fn(&tree_sitter::Node, &str) -> Option<String>>,
}

/// Get language configuration - SINGLE SOURCE OF TRUTH
///
/// Add new languages here. This is the only place you need to update
/// when adding support for a new language.
fn get_language_info(language: &str) -> Option<LanguageInfo> {
    match language {
        "python" => Some(LanguageInfo {
            element_query: python::ELEMENT_QUERY,
            call_query: python::CALL_QUERY,
            reference_query: "",
            function_node_kinds: &["function_definition"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: None,
        }),
        "rust" => Some(LanguageInfo {
            element_query: rust::ELEMENT_QUERY,
            call_query: rust::CALL_QUERY,
            reference_query: "",
            function_node_kinds: &["function_item", "impl_item"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: Some(rust::extract_function_name_for_kind),
            find_method_for_receiver_handler: None,
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
        }),
        "go" => Some(LanguageInfo {
            element_query: go::ELEMENT_QUERY,
            call_query: go::CALL_QUERY,
            reference_query: go::REFERENCE_QUERY,
            function_node_kinds: &["function_declaration", "method_declaration"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: Some(go::find_method_for_receiver),
        }),
        "java" => Some(LanguageInfo {
            element_query: java::ELEMENT_QUERY,
            call_query: java::CALL_QUERY,
            reference_query: "",
            function_node_kinds: &["method_declaration", "constructor_declaration"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: None,
        }),
        "kotlin" => Some(LanguageInfo {
            element_query: kotlin::ELEMENT_QUERY,
            call_query: kotlin::CALL_QUERY,
            reference_query: "",
            function_node_kinds: &["function_declaration", "class_body"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: None,
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
        }),
        "ruby" => Some(LanguageInfo {
            element_query: ruby::ELEMENT_QUERY,
            call_query: ruby::CALL_QUERY,
            reference_query: ruby::REFERENCE_QUERY,
            function_node_kinds: &["method", "singleton_method"],
            function_name_kinds: &["identifier", "field_identifier", "property_identifier"],
            extract_function_name_handler: None,
            find_method_for_receiver_handler: Some(ruby::find_method_for_receiver),
        }),
        _ => None,
    }
}

/// Get the tree-sitter query for extracting code elements for a language
pub fn get_element_query(language: &str) -> &'static str {
    get_language_info(language)
        .map(|info| info.element_query)
        .unwrap_or("")
}

/// Get the tree-sitter query for extracting function calls for a language
pub fn get_call_query(language: &str) -> &'static str {
    get_language_info(language)
        .map(|info| info.call_query)
        .unwrap_or("")
}

/// Get the tree-sitter query for extracting type references for a language
pub fn get_reference_query(language: &str) -> &'static str {
    get_language_info(language)
        .map(|info| info.reference_query)
        .unwrap_or("")
}

/// Get the node kinds that represent function-like constructs for a language
pub fn get_function_node_kinds(language: &str) -> &'static [&'static str] {
    get_language_info(language)
        .map(|info| info.function_node_kinds)
        .unwrap_or(&[])
}

/// Get the node kinds that represent function name identifiers for a language
pub fn get_function_name_kinds(language: &str) -> &'static [&'static str] {
    get_language_info(language)
        .map(|info| info.function_name_kinds)
        .unwrap_or(&[])
}

/// Extract function name for language-specific node kinds
///
/// Some languages have special cases where the function name cannot be extracted
/// using standard child node traversal. This function delegates to language-specific
/// implementations for those cases.
pub fn extract_function_name_for_kind(
    node: &tree_sitter::Node,
    source: &str,
    language: &str,
    kind: &str,
) -> Option<String> {
    get_language_info(language)
        .and_then(|info| info.extract_function_name_handler)
        .and_then(|handler| handler(node, source, kind))
}

/// Find method name for a receiver node (for method-to-type associations)
///
/// Some languages need to associate methods with their containing types during
/// reference extraction. This delegates to language-specific implementations.
pub fn find_method_for_receiver(
    receiver_node: &tree_sitter::Node,
    source: &str,
    language: &str,
) -> Option<String> {
    get_language_info(language)
        .and_then(|info| info.find_method_for_receiver_handler)
        .and_then(|handler| handler(receiver_node, source))
}
