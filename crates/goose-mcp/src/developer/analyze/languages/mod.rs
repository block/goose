pub mod go;
pub mod java;
pub mod javascript;
pub mod kotlin;
pub mod python;
pub mod rust;
pub mod swift;

/// Get the tree-sitter query for extracting code elements for a language
pub fn get_element_query(language: &str) -> &'static str {
    match language {
        "python" => python::ELEMENT_QUERY,
        "rust" => rust::ELEMENT_QUERY,
        "javascript" | "typescript" => javascript::ELEMENT_QUERY,
        "go" => go::ELEMENT_QUERY,
        "java" => java::ELEMENT_QUERY,
        "kotlin" => kotlin::ELEMENT_QUERY,
        "swift" => swift::ELEMENT_QUERY,
        _ => "",
    }
}

/// Get the tree-sitter query for extracting function calls for a language
pub fn get_call_query(language: &str) -> &'static str {
    match language {
        "python" => python::CALL_QUERY,
        "rust" => rust::CALL_QUERY,
        "javascript" | "typescript" => javascript::CALL_QUERY,
        "go" => go::CALL_QUERY,
        "java" => java::CALL_QUERY,
        "kotlin" => kotlin::CALL_QUERY,
        "swift" => swift::CALL_QUERY,
        _ => "",
    }
}
