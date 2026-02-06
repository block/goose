/// Tree-sitter query for extracting Kotlin code elements
pub const ELEMENT_QUERY: &str = r#"
    ; Functions
    (function_declaration name: (identifier) @func)

    ; Classes
    (class_declaration name: (identifier) @class)

    ; Objects (singleton classes)
    (object_declaration name: (identifier) @class)

    ; Imports
    (import) @import
"#;

/// Tree-sitter query for extracting Kotlin function calls
pub const CALL_QUERY: &str = r#"
    ; Simple function calls
    (call_expression
      (identifier) @function.call)

    ; Method calls with navigation (obj.method())
    (call_expression
      (navigation_expression
        (identifier) @method.call))
"#;
