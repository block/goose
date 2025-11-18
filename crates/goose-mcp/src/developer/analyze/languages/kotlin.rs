/// Tree-sitter query for extracting Kotlin code elements
pub const ELEMENT_QUERY: &str = r#"
    ; Functions
    (function_declaration (simple_identifier) @func)

    ; Classes
    (class_declaration (type_identifier) @class)

    ; Objects (singleton classes)
    (object_declaration (type_identifier) @class)

    ; Imports
    (import_header) @import
"#;

/// Tree-sitter query for extracting Kotlin function calls
pub const CALL_QUERY: &str = r#"
    ; Simple function calls
    (call_expression
      (simple_identifier) @function.call)

    ; Method calls with navigation (obj.method())
    (call_expression
      (navigation_expression
        (navigation_suffix
          (simple_identifier) @method.call)))
"#;
