/// Tree-sitter query for extracting Swift code elements
pub const ELEMENT_QUERY: &str = r#"
    ; Functions
    (function_declaration name: (simple_identifier) @func)

    ; Classes
    (class_declaration name: (type_identifier) @class)

    ; Protocols (interfaces)
    (protocol_declaration name: (type_identifier) @class)

    ; Imports
    (import_declaration) @import
"#;

/// Tree-sitter query for extracting Swift function calls
pub const CALL_QUERY: &str = r#"
    ; Function calls
    (call_expression
      (simple_identifier) @function.call)

    ; Method calls with navigation
    (call_expression
      (navigation_expression
        target: (_)
        suffix: (navigation_suffix
          suffix: (simple_identifier) @method.call)))

    ; Constructor calls
    (constructor_expression
      (user_type
        (type_identifier) @constructor.call))

    ; Async function calls
    (await_expression
      (call_expression
        (simple_identifier) @function.call))

    ; Async method calls
    (await_expression
      (call_expression
        (navigation_expression
          suffix: (navigation_suffix
            suffix: (simple_identifier) @method.call))))

    ; Static method calls (Type.method())
    (call_expression
      (navigation_expression
        target: (user_type)
        suffix: (navigation_suffix
          suffix: (simple_identifier) @scoped.call)))

    ; Closure calls
    (call_expression
      (navigation_expression) @function.call)
"#;
