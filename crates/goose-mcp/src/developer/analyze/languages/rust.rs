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
    ; Now captures the full Type::method instead of just method
    (call_expression
      function: (scoped_identifier) @scoped.call)
    
    ; Macro calls (often contain function-like behavior)
    (macro_invocation
      macro: (identifier) @macro.call)
"#;

/// Tree-sitter query for extracting Rust type references and usage patterns
pub const REFERENCE_QUERY: &str = r#"
    ; Method receivers - capture self parameters to associate methods with impl types
    (self_parameter) @method.receiver

    ; Struct instantiation - struct literals
    (struct_expression
      name: (type_identifier) @struct.literal)

    ; Field type declarations in structs
    (field_declaration
      type: (type_identifier) @field.type)

    ; Field with reference type
    (field_declaration
      type: (reference_type
        (type_identifier) @field.type))

    ; Field with generic type
    (field_declaration
      type: (generic_type
        type: (type_identifier) @field.type))

    ; Variable type annotations
    (let_declaration
      type: (type_identifier) @var.type)

    ; Variable with reference type
    (let_declaration
      type: (reference_type
        (type_identifier) @var.type))

    ; Function parameter types
    (parameter
      type: (type_identifier) @param.type)

    ; Parameter with reference type
    (parameter
      type: (reference_type
        (type_identifier) @param.type))
"#;

/// Extract function name for Rust-specific node kinds
///
/// Rust has special cases like impl_item blocks that should be
/// formatted as "impl TypeName" instead of extracting a simple name.
pub fn extract_function_name_for_kind(
    node: &tree_sitter::Node,
    source: &str,
    kind: &str,
) -> Option<String> {
    if kind == "impl_item" {
        // For impl blocks, find the type being implemented
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                if child.kind() == "type_identifier" {
                    return source
                        .get(child.byte_range())
                        .map(|s| format!("impl {}", s));
                }
            }
        }
    }
    None
}

/// Find the method name for a method receiver node in Rust
///
/// The receiver_node is a self_parameter. This walks up to find the
/// containing function_item and returns the method name.
pub fn find_method_for_receiver(
    receiver_node: &tree_sitter::Node,
    source: &str,
    _ast_recursion_limit: Option<usize>,
) -> Option<String> {
    // Walk up to find the function_item that contains this self_parameter
    let mut current = *receiver_node;

    while let Some(parent) = current.parent() {
        if parent.kind() == "function_item" {
            // Found the function, get its name
            for i in 0..parent.child_count() {
                if let Some(child) = parent.child(i) {
                    if child.kind() == "identifier" {
                        return source.get(child.byte_range()).map(|s| s.to_string());
                    }
                }
            }
        }
        current = parent;
    }
    None
}

/// Find the receiver type for a self parameter in Rust
///
/// In Rust, self parameters are special - they don't explicitly state their type.
/// This function walks up from a self_parameter node to find the impl block
/// and extracts the type being implemented.
pub fn find_receiver_type(node: &tree_sitter::Node, source: &str) -> Option<String> {
    // Walk up from self_parameter to find the impl_item
    let mut current = *node;
    while let Some(parent) = current.parent() {
        if parent.kind() == "impl_item" {
            // Find the type_identifier in the impl block
            for i in 0..parent.child_count() {
                if let Some(child) = parent.child(i) {
                    if child.kind() == "type_identifier" {
                        return source.get(child.byte_range()).map(|s| s.to_string());
                    }
                }
            }
        }
        current = parent;
    }
    None
}
