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
                    return Some(format!("impl {}", &source[child.byte_range()]));
                }
            }
        }
    }
    None
}
