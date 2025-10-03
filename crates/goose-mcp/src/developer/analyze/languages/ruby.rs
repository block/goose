/// Tree-sitter query for extracting Ruby code elements.
///
/// This query captures:
/// - Method definitions (def)
/// - Class and module definitions
/// - Common attr_* declarations (attr_accessor, attr_reader, attr_writer)
/// - Import statements (require, require_relative, load)
pub const ELEMENT_QUERY: &str = r#"
    ; Method definitions
    (method name: (identifier) @func)
    
    ; Class and module definitions
    (class name: (constant) @class)
    (module name: (constant) @class)
    
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
pub const CALL_QUERY: &str = r#"
    ; Method calls
    (call method: (identifier) @method.call)
    
    ; Method calls with receiver
    (call receiver: (_) method: (identifier) @method.call)
    
    ; Calls to constants (typically constructors)
    (call receiver: (constant) @function.call)
"#;
