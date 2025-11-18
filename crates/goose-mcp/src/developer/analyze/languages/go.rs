/// Tree-sitter query for extracting Go code elements
pub const ELEMENT_QUERY: &str = r#"
    (function_declaration name: (identifier) @func)
    (method_declaration name: (field_identifier) @func)
    (type_declaration (type_spec name: (type_identifier) @struct))
    (const_declaration (const_spec name: (identifier) @const))
    (import_declaration) @import
"#;

/// Tree-sitter query for extracting Go function calls and identifier references
pub const CALL_QUERY: &str = r#"
    ; Function calls
    (call_expression
      function: (identifier) @function.call)

    ; Method calls
    (call_expression
      function: (selector_expression
        field: (field_identifier) @method.call))

    ; Identifier references in various expression contexts
    ; This captures constants/variables used in arguments, comparisons, returns, assignments, etc.
    (argument_list (identifier) @identifier.reference)
    (binary_expression left: (identifier) @identifier.reference)
    (binary_expression right: (identifier) @identifier.reference)
    (unary_expression operand: (identifier) @identifier.reference)
    (return_statement (expression_list (identifier) @identifier.reference))
    (assignment_statement right: (expression_list (identifier) @identifier.reference))
"#;

/// Tree-sitter query for extracting Go struct references and usage patterns
pub const REFERENCE_QUERY: &str = r#"
    ; Method receivers - pointer type
    (method_declaration
      receiver: (parameter_list
        (parameter_declaration
          type: (pointer_type (type_identifier) @method.receiver))))

    ; Method receivers - value type
    (method_declaration
      receiver: (parameter_list
        (parameter_declaration
          type: (type_identifier) @method.receiver)))

    ; Struct literals - simple
    (composite_literal
      type: (type_identifier) @struct.literal)

    ; Struct literals - qualified (package.Type)
    (composite_literal
      type: (qualified_type
        name: (type_identifier) @struct.literal))

    ; Field declarations in structs - simple type
    (field_declaration
      type: (type_identifier) @field.type)

    ; Field declarations - pointer type
    (field_declaration
      type: (pointer_type
        (type_identifier) @field.type))

    ; Field declarations - qualified type (package.Type)
    (field_declaration
      type: (qualified_type
        name: (type_identifier) @field.type))

    ; Field declarations - pointer to qualified type
    (field_declaration
      type: (pointer_type
        (qualified_type
          name: (type_identifier) @field.type)))
"#;

/// Find the method name for a method receiver node in Go
///
/// This walks up the tree to find the method_declaration parent and extracts
/// the method name, used for associating methods with their receiver types.
pub fn find_method_for_receiver(
    receiver_node: &tree_sitter::Node,
    source: &str,
    _ast_recursion_limit: Option<usize>,
) -> Option<String> {
    let mut current = *receiver_node;
    while let Some(parent) = current.parent() {
        if parent.kind() == "method_declaration" {
            for i in 0..parent.child_count() {
                if let Some(child) = parent.child(i) {
                    if child.kind() == "field_identifier" {
                        return source.get(child.byte_range()).map(|s| s.to_string());
                    }
                }
            }
        }
        current = parent;
    }
    None
}
