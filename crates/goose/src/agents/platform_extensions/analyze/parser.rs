use std::path::{Path, PathBuf};
use tree_sitter::{Language, Parser as TsParser, Query, QueryCursor, StreamingIterator};

use super::languages::{LangInfo, lang_for_ext};

// ── Types ──────────────────────────────────────────────────────────────

pub struct FileAnalysis {
    pub path: PathBuf,
    pub language: &'static str,
    pub loc: usize,
    pub functions: Vec<Symbol>,
    pub classes: Vec<Symbol>,
    pub imports: Vec<Import>,
    pub calls: Vec<Call>,
}

pub struct Symbol {
    pub name: String,
    pub line: usize,
    pub parent: Option<String>,
    pub detail: Option<String>,
}

pub struct Import {
    pub module: String,
    pub count: usize,
}

pub struct Call {
    pub caller: String,
    pub callee: String,
    pub line: usize,
}

// ── Parser ─────────────────────────────────────────────────────────────

#[derive(Default)]
pub struct Parser;

impl Parser {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze_file(&self, path: &Path, source: &str) -> Option<FileAnalysis> {
        let ext = path.extension()?.to_str()?;
        let info = lang_for_ext(ext)?;
        let lang = (info.language)();

        let mut parser = TsParser::new();
        parser.set_language(&lang).ok()?;
        let tree = parser.parse(source, None)?;
        let root = tree.root_node();

        let loc = source.lines().count();
        let functions = extract_functions(&lang, info, root, source);
        let classes = extract_classes(&lang, info, root, source);
        let imports = extract_imports(&lang, info.queries.imports, root, source);
        let calls = extract_calls(&lang, info.queries.calls, root, source, info);

        Some(FileAnalysis {
            path: path.to_path_buf(),
            language: info.name,
            loc,
            functions,
            classes,
            imports,
            calls,
        })
    }
}

// ── Query Runners ──────────────────────────────────────────────────────

fn extract_functions(
    lang: &Language,
    info: &LangInfo,
    root: tree_sitter::Node,
    source: &str,
) -> Vec<Symbol> {
    let Ok(query) = Query::new(lang, info.queries.functions) else {
        return vec![];
    };
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, root, source.as_bytes());
    let mut symbols = Vec::new();

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if query.capture_names()[cap.index as usize] == "name" {
                let name = node_text(source, &cap.node).to_string();
                let line = cap.node.start_position().row + 1;
                let parent = find_enclosing_class(cap.node, source, info);
                let detail = extract_fn_signature(cap.node, source);
                symbols.push(Symbol { name, line, parent, detail });
            }
        }
    }
    symbols
}

fn extract_classes(
    lang: &Language,
    info: &LangInfo,
    root: tree_sitter::Node,
    source: &str,
) -> Vec<Symbol> {
    let Ok(query) = Query::new(lang, info.queries.classes) else {
        return vec![];
    };
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, root, source.as_bytes());
    let mut symbols = Vec::new();

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if query.capture_names()[cap.index as usize] == "name" {
                let name_text = node_text(source, &cap.node).to_string();
                let line = cap.node.start_position().row + 1;

                // Capture superclass / inheritance info per language
                let mut name = name_text;
                if let Some(parent_node) = cap.node.parent() {
                    let inheritance = extract_inheritance(info.name, &parent_node, source);
                    if !inheritance.is_empty() {
                        name = format!("{}({})", name, inheritance);
                    }
                }

                // Extract field summary from the class/struct body
                let detail = extract_class_detail(cap.node, source, info);
                symbols.push(Symbol { name, line, parent: None, detail });
            }
        }
    }
    symbols
}

/// Extract the superclass / extends / implements target from a class declaration node.
/// Returns an empty string if no inheritance is detected.
/// The `class_node` is the parent of the name node (i.e. the full class declaration).
fn extract_inheritance(lang_name: &str, class_node: &tree_sitter::Node, source: &str) -> String {
    match lang_name {
        // Python: class Foo(Bar, Baz) → argument_list sibling
        "python" => {
            if let Some(supers) = find_child_by_kind(class_node, "argument_list") {
                let super_text = node_text(source, &supers).trim();
                let inner = super_text
                    .strip_prefix('(')
                    .and_then(|s| s.strip_suffix(')'))
                    .unwrap_or("");
                if !inner.is_empty() {
                    return inner.to_string();
                }
            }
            String::new()
        }

        // TypeScript / TSX: class Foo extends Bar implements Baz { ... }
        // class_declaration → class_heritage → extends_clause → type_identifier
        // interface_declaration → extends_type_clause → type_identifier
        "typescript" | "tsx" => {
            // class extends
            if let Some(heritage) = find_child_by_kind(class_node, "class_heritage") {
                if let Some(extends_clause) = find_child_by_kind(&heritage, "extends_clause") {
                    if let Some(ti) = find_descendant_by_kind(&extends_clause, "type_identifier")
                        .or_else(|| find_descendant_by_kind(&extends_clause, "identifier"))
                    {
                        return node_text(source, &ti).to_string();
                    }
                }
            }
            // interface extends
            if let Some(extends_clause) = find_child_by_kind(class_node, "extends_type_clause") {
                if let Some(ti) = find_descendant_by_kind(&extends_clause, "type_identifier")
                    .or_else(|| find_descendant_by_kind(&extends_clause, "identifier"))
                {
                    return node_text(source, &ti).to_string();
                }
            }
            String::new()
        }

        // JavaScript: class Foo extends Bar { ... }
        // class_declaration → class_heritage → identifier (expression directly, no extends_clause wrapper)
        "javascript" => {
            if let Some(heritage) = find_child_by_kind(class_node, "class_heritage") {
                // JS heritage contains the expression directly (identifier, member_expression, etc.)
                if let Some(id) = find_descendant_by_kind(&heritage, "identifier") {
                    return node_text(source, &id).to_string();
                }
            }
            String::new()
        }

        // Java: class Foo extends Bar implements Baz { ... }
        // class_declaration → superclass → type_identifier
        "java" => {
            if let Some(superclass) = find_child_by_kind(class_node, "superclass") {
                if let Some(ti) = find_descendant_by_kind(&superclass, "type_identifier")
                    .or_else(|| find_descendant_by_kind(&superclass, "identifier"))
                {
                    return node_text(source, &ti).to_string();
                }
            }
            // interface extends
            if let Some(extends) = find_child_by_kind(class_node, "extends_interfaces") {
                if let Some(ti) = find_descendant_by_kind(&extends, "type_identifier")
                    .or_else(|| find_descendant_by_kind(&extends, "identifier"))
                {
                    return node_text(source, &ti).to_string();
                }
            }
            String::new()
        }

        // Kotlin: class Foo : Bar(), Baz { ... }
        // class_declaration → delegation_specifiers → delegation_specifier → user_type → type_identifier
        "kotlin" => {
            if let Some(specs) = find_child_by_kind(class_node, "delegation_specifiers") {
                if let Some(spec) = find_child_by_kind(&specs, "delegation_specifier") {
                    // Try user_type → type_identifier first
                    if let Some(ut) = find_child_by_kind(&spec, "user_type") {
                        if let Some(ti) = find_descendant_by_kind(&ut, "type_identifier")
                            .or_else(|| find_descendant_by_kind(&ut, "identifier"))
                        {
                            return node_text(source, &ti).to_string();
                        }
                    }
                    // Fallback: constructor_invocation → user_type
                    if let Some(ci) = find_child_by_kind(&spec, "constructor_invocation") {
                        if let Some(ut) = find_child_by_kind(&ci, "user_type") {
                            if let Some(ti) = find_descendant_by_kind(&ut, "type_identifier")
                                .or_else(|| find_descendant_by_kind(&ut, "identifier"))
                            {
                                return node_text(source, &ti).to_string();
                            }
                        }
                    }
                }
            }
            String::new()
        }

        // Ruby: class Foo < Bar
        // class → superclass → constant | scope_resolution
        "ruby" => {
            if let Some(superclass) = find_child_by_kind(class_node, "superclass") {
                if let Some(c) = find_child_by_kind(&superclass, "scope_resolution") {
                    return node_text(source, &c).to_string();
                }
                if let Some(c) = find_child_by_kind(&superclass, "constant") {
                    return node_text(source, &c).to_string();
                }
            }
            String::new()
        }

        // Swift: class Foo: Bar, Protocol { ... }
        // class_declaration → inheritance_specifier → type_identifier
        "swift" => {
            if let Some(inh) = find_child_by_kind(class_node, "inheritance_specifier") {
                if let Some(ti) = find_descendant_by_kind(&inh, "user_type") {
                    if let Some(id) = find_descendant_by_kind(&ti, "type_identifier") {
                        return node_text(source, &id).to_string();
                    }
                }
                if let Some(ti) = find_descendant_by_kind(&inh, "type_identifier") {
                    return node_text(source, &ti).to_string();
                }
            }
            String::new()
        }

        // Rust: impl Display for MyType → "MyType(impl Display)"
        // impl_item with "for" keyword: trait is the first type_identifier, type is after "for"
        "rust" => {
            // Only for impl_item nodes
            if class_node.kind() != "impl_item" {
                return String::new();
            }
            // Check if there's a "for" keyword child — indicates trait impl
            let mut has_for = false;
            for i in 0..class_node.child_count() as u32 {
                if let Some(child) = class_node.child(i) {
                    if node_text(source, &child) == "for" {
                        has_for = true;
                        break;
                    }
                }
            }
            if !has_for {
                return String::new();
            }
            // Find the trait name: it's the type before "for"
            // In tree-sitter-rust, impl_item has: "impl" <trait_type> "for" <type>
            // The trait is typically a type_identifier or scoped_type_identifier before "for"
            let mut trait_name = String::new();
            let mut found_for = false;
            for i in 0..class_node.child_count() as u32 {
                if let Some(child) = class_node.child(i) {
                    if node_text(source, &child) == "for" {
                        found_for = true;
                    } else if !found_for && (child.kind() == "type_identifier"
                        || child.kind() == "scoped_type_identifier"
                        || child.kind() == "generic_type")
                    {
                        trait_name = node_text(source, &child).to_string();
                    }
                }
            }
            if !trait_name.is_empty() {
                return format!("impl {}", trait_name);
            }
            String::new()
        }

        _ => String::new(),
    }
}

/// Walk up from a function node to find the nearest enclosing class-like container.
fn find_enclosing_class(
    node: tree_sitter::Node,
    source: &str,
    info: &LangInfo,
) -> Option<String> {
    let mut cur = node;
    while let Some(parent) = cur.parent() {
        if info.class_kinds.contains(&parent.kind()) {
            // For Rust impl_item: get the type name
            if parent.kind() == "impl_item" {
                return find_child_by_kind(&parent, "type_identifier")
                    .map(|n| node_text(source, &n).to_string());
            }
            // For Go type_declaration, look inside type_spec
            if parent.kind() == "type_declaration" {
                for i in 0..parent.child_count() as u32 {
                    if let Some(child) = parent.child(i) {
                        if child.kind() == "type_spec" {
                            return find_child_by_kind(&child, "type_identifier")
                                .map(|n| node_text(source, &n).to_string());
                        }
                    }
                }
                return None;
            }
            // Generic: find the name child (identifier, type_identifier, constant, etc.)
            let name_kinds = &["identifier", "type_identifier", "constant", "simple_identifier"];
            for kind in name_kinds {
                if let Some(n) = find_child_by_kind(&parent, kind) {
                    return Some(node_text(source, &n).to_string());
                }
            }
            return None;
        }
        cur = parent;
    }
    None
}

/// Extract a compact function signature: "(params) -> ReturnType"
fn extract_fn_signature(name_node: tree_sitter::Node, source: &str) -> Option<String> {
    // The name_node is the identifier inside the function node; go to the function node
    let fn_node = name_node.parent()?;

    let mut parts = String::new();

    // Find parameter list child
    let param_kinds = &[
        "parameters", "formal_parameters", "parameter_list",
        "function_value_parameters", "method_parameters",
        "lambda_parameters",
    ];
    let params_node = param_kinds.iter()
        .find_map(|kind| find_child_by_kind(&fn_node, kind));

    if let Some(pn) = params_node {
        let raw = node_text(source, &pn);
        if raw.len() <= 60 {
            parts.push_str(raw);
        } else {
            // Count commas to estimate arg count
            let count = raw.matches(',').count() + 1;
            parts.push_str(&format!("({} args)", count));
        }
    } else {
        parts.push_str("()");
    }

    // Find return type annotation
    let ret_kinds = &[
        "type", "return_type", "type_annotation",
    ];
    // For Rust: look for a child that is "->" followed by a type
    // For Python: look for "return_type" or "type" child
    // Generic approach: scan children for return type indicators
    for i in 0..fn_node.child_count() as u32 {
        if let Some(child) = fn_node.child(i) {
            if ret_kinds.contains(&child.kind()) {
                let ret_text = node_text(source, &child).trim().to_string();
                if !ret_text.is_empty() {
                    let ret_text = ret_text.trim_start_matches("->").trim_start_matches(':').trim();
                    if !ret_text.is_empty() {
                        parts.push_str("->");
                        parts.push_str(&truncate(ret_text, 30));
                    }
                }
                break;
            }
            // Rust uses "->" as a literal anonymous child, then a type child follows
            if node_text(source, &child) == "->" {
                if let Some(type_child) = fn_node.child(i + 1) {
                    let ret_text = node_text(source, &type_child).trim();
                    if !ret_text.is_empty() {
                        parts.push_str("->");
                        parts.push_str(&truncate(ret_text, 30));
                    }
                }
                break;
            }
        }
    }

    if parts == "()" {
        // No useful info extracted
        return None;
    }
    Some(parts)
}

/// Extract a compact detail for a class/struct: field names or count.
fn extract_class_detail(
    name_node: tree_sitter::Node,
    source: &str,
    info: &LangInfo,
) -> Option<String> {
    let class_node = name_node.parent()?;

    // Determine field node kinds based on language
    let (body_kinds, field_kinds): (&[&str], &[&str]) = match info.name {
        "rust" => (&["field_declaration_list"], &["field_declaration"]),
        "go" => (&["field_declaration_list", "struct_type"], &["field_declaration"]),
        "java" | "kotlin" => (&["class_body"], &["field_declaration"]),
        _ => return None, // Skip Python (hard), JS/TS/Ruby/Swift for now
    };

    // Find the body node
    let body = body_kinds.iter()
        .find_map(|kind| find_descendant_by_kind(&class_node, kind))?;

    // Collect field names
    let mut fields: Vec<String> = Vec::new();
    collect_field_names(&body, field_kinds, source, &mut fields);

    if fields.is_empty() {
        return None;
    }

    if fields.len() <= 5 {
        Some(format!("{{{}}}", fields.join(",")))
    } else {
        Some(format!("{{{}f}}", fields.len()))
    }
}

/// Collect field names from a body node.
fn collect_field_names(
    node: &tree_sitter::Node,
    field_kinds: &[&str],
    source: &str,
    out: &mut Vec<String>,
) {
    for i in 0..node.child_count() as u32 {
        if let Some(child) = node.child(i) {
            if field_kinds.contains(&child.kind()) {
                // Find the field name: first identifier-like child
                let name_kinds = &["field_identifier", "identifier", "type_identifier"];
                for nk in name_kinds {
                    if let Some(n) = find_child_by_kind(&child, nk) {
                        out.push(node_text(source, &n).to_string());
                        break;
                    }
                }
            }
        }
    }
}

fn find_child_by_kind<'a>(
    node: &tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    (0..node.child_count() as u32)
        .filter_map(|i| node.child(i))
        .find(|c| c.kind() == kind)
}

fn find_descendant_by_kind<'a>(
    node: &tree_sitter::Node<'a>,
    kind: &str,
) -> Option<tree_sitter::Node<'a>> {
    if node.kind() == kind {
        return Some(*node);
    }
    for i in 0..node.child_count() as u32 {
        if let Some(child) = node.child(i) {
            if let Some(found) = find_descendant_by_kind(&child, kind) {
                return Some(found);
            }
        }
    }
    None
}

fn extract_imports(
    lang: &Language,
    query_src: &str,
    root: tree_sitter::Node,
    source: &str,
) -> Vec<Import> {
    let Ok(query) = Query::new(lang, query_src) else {
        return vec![];
    };
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, root, source.as_bytes());
    let mut imports: Vec<Import> = Vec::new();

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if query.capture_names()[cap.index as usize] != "path" {
                continue;
            }
            let raw = node_text(source, &cap.node);
            let module = normalize_import(raw.trim());
            if let Some(existing) = imports.iter_mut().find(|i| i.module == module) {
                existing.count += 1;
            } else {
                imports.push(Import { module, count: 1 });
            }
        }
    }
    imports
}

fn normalize_import(s: &str) -> String {
    let s = s
        .trim_start_matches("use ")
        .trim_start_matches("import ")
        .trim_start_matches("from ")
        .trim_start_matches("require_relative ")
        .trim_start_matches("require ")
        .trim_start_matches("load ")
        .trim_end_matches(';')
        .trim()
        .trim_matches(|c| c == '\'' || c == '"');
    // Handle Python "from X import Y" → keep just "X"
    let s = s.split(" import ").next().unwrap_or(s);
    match s.find("::{") {
        Some(i) => s.get(..i).unwrap_or(s).to_string(),
        None => s.to_string(),
    }
}

fn extract_calls(
    lang: &Language,
    query_src: &str,
    root: tree_sitter::Node,
    source: &str,
    info: &LangInfo,
) -> Vec<Call> {
    let Ok(query) = Query::new(lang, query_src) else {
        return vec![];
    };
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, root, source.as_bytes());
    let mut calls = Vec::new();

    while let Some(m) = matches.next() {
        for cap in m.captures {
            if query.capture_names()[cap.index as usize] != "name" {
                continue;
            }
            let callee = node_text(source, &cap.node).to_string();
            let line = cap.node.start_position().row + 1;
            let caller = find_enclosing_fn(cap.node, source, info)
                .unwrap_or_else(|| "<module>".to_string());
            calls.push(Call {
                caller,
                callee,
                line,
            });
        }
    }
    calls
}

fn find_enclosing_fn(
    node: tree_sitter::Node,
    source: &str,
    info: &LangInfo,
) -> Option<String> {
    let mut cur = node;
    while let Some(parent) = cur.parent() {
        if info.fn_kinds.contains(&parent.kind()) {
            // Special case: Swift init/deinit
            if parent.kind() == "init_declaration" {
                return Some("init".into());
            }
            if parent.kind() == "deinit_declaration" {
                return Some("deinit".into());
            }
            return find_child_text(&parent, info.fn_name_kinds, source);
        }
        cur = parent;
    }
    None
}

fn find_child_text(
    node: &tree_sitter::Node,
    kinds: &[&str],
    source: &str,
) -> Option<String> {
    (0..node.child_count() as u32)
        .filter_map(|i| node.child(i))
        .find(|c| kinds.contains(&c.kind()))
        .map(|c| node_text(source, &c).to_string())
}

/// Truncate a string to at most `max` chars, appending "..." if truncated.
fn truncate(s: &str, max: usize) -> String {
    if s.len() <= max {
        return s.to_string();
    }
    let limit = max.saturating_sub(3);
    // Walk back to a valid char boundary
    let end = (0..=limit).rev().find(|&i| s.is_char_boundary(i)).unwrap_or(0);
    let prefix = s.get(..end).unwrap_or("");
    format!("{}...", prefix)
}

fn node_text<'a>(source: &'a str, node: &tree_sitter::Node) -> &'a str {
    source.get(node.byte_range()).unwrap_or("")
}
