/// Tree-sitter query for extracting Rust code elements
pub const ELEMENT_QUERY: &str = r#"
    (function_item name: (identifier) @func)
    (impl_item type: (type_identifier) @class)
    (struct_item name: (type_identifier) @struct)
    (use_declaration) @import
"#;

/// Tree-sitter query for extracting Rust function calls
pub const CALL_QUERY: &str = r#"
    ; Function calls
    (call_expression
      function: (identifier) @function.call)
    
    ; Method calls
    (call_expression
      function: (field_expression
        field: (field_identifier) @method.call))
    
    ; Associated function calls (e.g., Type::method())
    (call_expression
      function: (scoped_identifier
        name: (identifier) @scoped.call))
    
    ; Macro calls (often contain function-like behavior)
    (macro_invocation
      macro: (identifier) @macro.call)
"#;
