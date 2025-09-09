/// Tree-sitter query for extracting Kotlin code elements
pub const ELEMENT_QUERY: &str = r#"
    (function_declaration (identifier) @func)
    (class_declaration (identifier) @class)
    (object_declaration (identifier) @class)
    (import) @import
"#;

/// Tree-sitter query for extracting Kotlin function calls
pub const CALL_QUERY: &str = r#"
    ; Function calls (simple identifier)
    (call_expression
      (identifier) @function.call)
    
    ; Method calls (navigation expression like obj.method)
    (call_expression
      (navigation_expression) @method.call)
"#;
