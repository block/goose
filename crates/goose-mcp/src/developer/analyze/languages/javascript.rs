/// Tree-sitter query for extracting JavaScript/TypeScript code elements
pub const ELEMENT_QUERY: &str = r#"
    (function_declaration name: (identifier) @func)
    (class_declaration name: (identifier) @class)
    (import_statement) @import
"#;

/// Tree-sitter query for extracting JavaScript/TypeScript function calls
pub const CALL_QUERY: &str = r#"
    ; Function calls
    (call_expression
      function: (identifier) @function.call)
    
    ; Method calls
    (call_expression
      function: (member_expression
        property: (property_identifier) @method.call))
    
    ; Constructor calls
    (new_expression
      constructor: (identifier) @constructor.call)
"#;
