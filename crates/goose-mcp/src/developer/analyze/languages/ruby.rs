/// Tree-sitter query for extracting Ruby code elements.
///
/// This query captures:
/// - Method definitions (def)
/// - Class and module definitions
/// - Constants
/// - Common attr_* declarations (attr_accessor, attr_reader, attr_writer)
/// - Import statements (require, require_relative, load)
pub const ELEMENT_QUERY: &str = r#"
    ; Method definitions
    (method name: (identifier) @func)
    
    ; Class and module definitions
    (class name: (constant) @class)
    (module name: (constant) @class)

    ; Constant assignments
    (assignment left: (constant) @const)

    ; Attr declarations as functions
    (call method: (identifier) @func (#eq? @func "attr_accessor"))
    (call method: (identifier) @func (#eq? @func "attr_reader"))
    (call method: (identifier) @func (#eq? @func "attr_writer"))
    
    ; Require statements
    (call method: (identifier) @import (#eq? @import "require"))
    (call method: (identifier) @import (#eq? @import "require_relative"))
    (call method: (identifier) @import (#eq? @import "load"))
"#;

/// Tree-sitter query for extracting Ruby function calls.
///
/// This query captures:
/// - Direct method calls
/// - Method calls with receivers (object.method)
/// - Calls to constants (typically constructors like ClassName.new)
/// - Identifier and constant references in various expression contexts
pub const CALL_QUERY: &str = r#"
    ; Method calls
    (call method: (identifier) @method.call)

    ; Method calls with receiver
    (call receiver: (_) method: (identifier) @method.call)

    ; Calls to constants (typically constructors)
    (call receiver: (constant) @function.call)

    ; Identifier and constant references in argument lists
    (argument_list (identifier) @identifier.reference)
    (argument_list (constant) @identifier.reference)

    ; Binary expressions
    (binary left: (identifier) @identifier.reference)
    (binary right: (identifier) @identifier.reference)
    (binary left: (constant) @identifier.reference)
    (binary right: (constant) @identifier.reference)

    ; Assignment expressions
    (assignment right: (identifier) @identifier.reference)
    (assignment right: (constant) @identifier.reference)
"#;

/// Tree-sitter query for extracting Ruby type references and usage patterns.
///
/// This query captures:
/// - Method-to-class associations (instance and class methods)
/// - Class instantiation (ClassName.new)
/// - Type references in various contexts
pub const REFERENCE_QUERY: &str = r#"
    ; Instance methods within a class - capture class name, will find method via receiver lookup
    (class
      name: (constant) @method.receiver
      (body_statement (method)))

    ; Class instantiation (ClassName.new)
    (call
      receiver: (constant) @struct.literal
      method: (identifier) @method.name (#eq? @method.name "new"))

    ; Constant references as receivers (type usage)
    (call
      receiver: (constant) @field.type
      method: (identifier))
"#;

/// Find the method name for a method receiver node in Ruby
///
/// For Ruby, the receiver_node is the class constant. This finds methods
/// within that class node, used for associating methods with their classes.
pub fn find_method_for_receiver(
    receiver_node: &tree_sitter::Node,
    source: &str,
    ast_recursion_limit: Option<usize>,
) -> Option<String> {
    let max_depth = ast_recursion_limit.unwrap_or(10);

    // For Ruby, receiver_node is the class constant
    if receiver_node.kind() == "constant" {
        let mut current = *receiver_node;
        while let Some(parent) = current.parent() {
            if parent.kind() == "class" {
                return find_first_method_in_class(&parent, source, max_depth);
            }
            current = parent;
        }
    }
    None
}

/// Find the first method name within a Ruby class node
fn find_first_method_in_class(
    class_node: &tree_sitter::Node,
    source: &str,
    max_depth: usize,
) -> Option<String> {
    for i in 0..class_node.child_count() {
        if let Some(child) = class_node.child(i) {
            if child.kind() == "body_statement" {
                return find_method_in_body_with_depth(&child, source, 0, max_depth);
            }
        }
    }
    None
}

/// Recursively find a method within a body_statement node with depth limit
fn find_method_in_body_with_depth(
    node: &tree_sitter::Node,
    source: &str,
    depth: usize,
    max_depth: usize,
) -> Option<String> {
    if depth >= max_depth {
        return None;
    }

    for i in 0..node.child_count() {
        if let Some(child) = node.child(i) {
            if child.kind() == "method" {
                for j in 0..child.child_count() {
                    if let Some(name_node) = child.child(j) {
                        if name_node.kind() == "identifier" {
                            return source.get(name_node.byte_range()).map(|s| s.to_string());
                        }
                    }
                }
            }
        }
    }
    None
}
