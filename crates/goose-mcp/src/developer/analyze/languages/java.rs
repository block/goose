/// Tree-sitter query for extracting Java code elements
pub const ELEMENT_QUERY: &str = r#"
    (method_declaration name: (identifier) @func)
    (class_declaration name: (identifier) @class)
    (import_declaration) @import
"#;

/// Tree-sitter query for extracting Java function calls
pub const CALL_QUERY: &str = r#"
    ; Method invocations
    (method_invocation
      name: (identifier) @method.call)
    
    ; Constructor calls
    (object_creation_expression
      type: (type_identifier) @constructor.call)
"#;
