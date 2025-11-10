/// Tree-sitter query for extracting Go code elements
pub const ELEMENT_QUERY: &str = r#"
    (function_declaration name: (identifier) @func)
    (method_declaration name: (field_identifier) @func)
    (type_declaration (type_spec name: (type_identifier) @struct))
    (import_declaration) @import
"#;

/// Tree-sitter query for extracting Go function calls
pub const CALL_QUERY: &str = r#"
    ; Function calls
    (call_expression
      function: (identifier) @function.call)
    
    ; Method calls
    (call_expression
      function: (selector_expression
        field: (field_identifier) @method.call))
"#;
