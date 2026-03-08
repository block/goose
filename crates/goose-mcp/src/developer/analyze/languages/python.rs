/// Tree-sitter query for extracting Python code elements
pub const ELEMENT_QUERY: &str = r#"
    (function_definition name: (identifier) @func)
    (class_definition name: (identifier) @class)
    (import_statement) @import
    (import_from_statement) @import
    (aliased_import) @import
    (assignment left: (identifier) @class)
"#;

/// Tree-sitter query for extracting Python function calls
pub const CALL_QUERY: &str = r#"
    ; Function calls
    (call
      function: (identifier) @function.call)
    
    ; Method calls
    (call
      function: (attribute
        attribute: (identifier) @method.call))

    ; Decorator applications
    (decorator (identifier) @function.call)
    (decorator (attribute attribute: (identifier) @method.call))
"#;
