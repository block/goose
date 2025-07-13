use std::time::Instant;
use ignore::WalkBuilder;
use serde_json::json;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::io;
use tree_sitter::{Language, Parser};
// Import the language crates
use tree_sitter_rust as ts_rust;
use tree_sitter_python as ts_python;
use tree_sitter_javascript as ts_javascript;
use tree_sitter_cpp as ts_cpp;
use tree_sitter_java as ts_java;
use tree_sitter_typescript as ts_typescript;
use tree_sitter_c_sharp as ts_c_sharp;
use tree_sitter_swift as ts_swift;
use tree_sitter_go as ts_go;

/// Recursively extract function call names from a node (for any language)
fn extract_function_calls(node: &tree_sitter::Node, source: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut stack = vec![*node];
    while let Some(n) = stack.pop() {
        let kind = n.kind();
        // Heuristics for function call nodes in various languages
        if kind == "call_expression" || kind == "function_call" || kind == "invocation_expression" {
            // Try to get the function name
            let name = n.child_by_field_name("function")
                .or_else(|| n.child_by_field_name("function_name"))
                .or_else(|| n.child_by_field_name("name"))
                .or_else(|| n.child(0));
            if let Some(name_node) = name {
                let text = name_node.utf8_text(source.as_bytes()).unwrap_or("");
                // Only push non-empty names
                if !text.is_empty() {
                    calls.push(text.to_string());
                }
            }
        }
        // Recurse into children
        for child in n.children(&mut n.walk()) {
            stack.push(child);
        }
    }
    calls
}

// Provide functions to get the language from each crate
fn tree_sitter_rust() -> Language {
    ts_rust::language()
}
fn tree_sitter_python() -> Language {
    ts_python::language()
}
fn tree_sitter_javascript() -> Language {
    ts_javascript::language()
}
fn tree_sitter_cpp() -> Language {
    ts_cpp::language()
}
fn tree_sitter_java() -> Language {
    ts_java::language()
}
fn tree_sitter_typescript() -> Language {
    ts_typescript::language_typescript()
}
fn tree_sitter_c_sharp() -> Language {
    ts_c_sharp::language()
}
fn tree_sitter_swift() -> Language {
    ts_swift::language()
}
fn tree_sitter_go() -> Language {
    ts_go::language()
}

fn make_entity_json(
    file: &str,
    language: &str,
    entity_type: &str,
    name: &str,
    signature: &str,
    starting_line: usize,
    ending_line: usize,
    parent: Option<&str>,
    doc: Option<&str>,
    calls: Option<&[String]>,
) -> serde_json::Value {
    let mut obj = serde_json::json!({
        "file": file,
        "language": language,
        "type": entity_type,
        "name": name,
        "signature": signature,
        "startingLineNumber": starting_line,
        "endingLineNumber": ending_line,
    });
    if let Some(parent) = parent {
        obj["parent"] = serde_json::json!(parent);
    }
    if let Some(doc) = doc {
        obj["doc"] = serde_json::json!(doc);
    }
    if let Some(calls) = calls {
        obj["calls"] = serde_json::json!(calls);
    }
    obj
}

fn detect_language(path: &Path) -> Option<&'static str> {
    match path.extension().and_then(|e| e.to_str()) {
        Some("rs") => Some("rust"),
        Some("py") => Some("python"),
        Some("js") => Some("javascript"),
        Some("ts") | Some("tsx") => Some("typescript"),
        Some("cpp") | Some("cc") | Some("cxx") | Some("hpp") | Some("h") => Some("cpp"),
        Some("java") => Some("java"),
        Some("cs") => Some("c_sharp"),
        Some("swift") => Some("swift"),
        Some("go") => Some("go"),
        _ => None,
    }
}

fn get_language(lang: &str) -> Option<Language> {
    match lang {
        "rust" => Some(tree_sitter_rust()),
        "python" => Some(tree_sitter_python()),
        "javascript" => Some(tree_sitter_javascript()),
        "typescript" => Some(tree_sitter_typescript()),
        "cpp" => Some(tree_sitter_cpp()),
        "java" => Some(tree_sitter_java()),
        "c_sharp" => Some(tree_sitter_c_sharp()),
        "swift" => Some(tree_sitter_swift()),
        "go" => Some(tree_sitter_go()),
        _ => None,
    }
}
fn extract_swift_entities<W: Write>(
    tree: &tree_sitter::Tree,
    source: &str,
    file: &str,
    out: &mut W,
) {
    // Swift: extract classes, structs, enums, protocols, functions
    // use tree_sitter::{Node, TreeCursor};
    let cursor = tree.root_node().walk();
    let mut stack = vec![(cursor.node(), None::<String>)];
    while let Some((node, parent)) = stack.pop() {
        let kind = node.kind();
        if kind == "class_declaration"
            || kind == "struct_declaration"
            || kind == "enum_declaration"
            || kind == "protocol_declaration"
        {
            let name_node = node.child_by_field_name("name");
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let entity_type = match kind {
                "class_declaration" => "class",
                "struct_declaration" => "struct",
                "enum_declaration" => "enum",
                "protocol_declaration" => "protocol",
                _ => kind,
            };
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let info = make_entity_json(
                file,
                "swift",
                entity_type,
                name,
                signature,
                starting_line,
                ending_line,
                None,
                None,
                None,
            );
            let _ = writeln!(out, "{}", info.to_string());
            for child in node.children(&mut node.walk()) {
                stack.push((child, Some(name.to_string())));
            }
        } else if kind == "function_declaration" {
            let name_node = node.child_by_field_name("name");
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let parent = parent.as_deref();
            let calls = extract_function_calls(&node, source);
            let info = make_entity_json(
                file,
                "swift",
                "function",
                name,
                signature,
                starting_line,
                ending_line,
                parent,
                None,
                Some(&calls),
            );
            let _ = writeln!(out, "{}", info.to_string());
        } else {
            for child in node.children(&mut node.walk()) {
                stack.push((child, parent.clone()));
            }
        }
    }
}

fn extract_go_entities<W: Write>(tree: &tree_sitter::Tree, source: &str, file: &str, out: &mut W) {
    // Go: extract functions, types, fields, variables, imports, enums, generics
    // use tree_sitter::{Node, TreeCursor};
    let cursor = tree.root_node().walk();
    let mut stack = vec![(cursor.node(), None::<String>)];
    while let Some((node, parent)) = stack.pop() {
        let kind = node.kind();
        if kind == "import_spec" {
            // import_spec: child 'path'
            let path_node = node.child_by_field_name("path");
            let name = path_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let info = make_entity_json(
                file, "go", "import", name, signature, starting_line, ending_line, None, None, None,
            );
            let _ = writeln!(out, "{}", info.to_string());
        } else if kind == "type_spec" {
            // type_spec: child 'name', child 'type' (struct_type, interface_type, etc), child 'type_parameters' (generics)
            let name_node = node.child_by_field_name("name");
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let type_node = node.child_by_field_name("type");
            let type_kind = type_node.map(|n| n.kind()).unwrap_or("");
            let entity_type = match type_kind {
                "struct_type" => "struct",
                "interface_type" => "interface",
                "enum_type" => "enum",
                _ => "type",
            };
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let generics = node
                .child_by_field_name("type_parameters")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok());
            let mut info = make_entity_json(
                file,
                "go",
                entity_type,
                name,
                signature,
                starting_line,
                ending_line,
                None,
                None,
                None,
            );
            if let Some(g) = generics {
                info["generics"] = serde_json::json!(g);
            }
            let _ = writeln!(out, "{}", info.to_string());
            // Extract fields for structs/interfaces/enums
            if let Some(type_node) = type_node {
                for child in type_node.children(&mut type_node.walk()) {
                    if child.kind() == "field_declaration" {
                        let field_name = child
                            .child_by_field_name("name")
                            .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                            .unwrap_or("");
                        let starting_line = child.start_position().row + 1;
                        let ending_line = child.end_position().row + 1;
                        let signature = source[child.byte_range()].lines().next().unwrap_or("");
                        let info = make_entity_json(
                            file,
                            "go",
                            "field",
                            field_name,
                            signature,
                            starting_line,
                            ending_line,
                            Some(name),
                            None,
                            None,
                        );
                        let _ = writeln!(out, "{}", info.to_string());
                    }
                }
            }
            for child in node.children(&mut node.walk()) {
                stack.push((child, Some(name.to_string())));
            }
        } else if kind == "function_declaration" {
            let name_node = node.child_by_field_name("name");
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let parent = parent.as_deref();
            let generics = node
                .child_by_field_name("type_parameters")
                .and_then(|n| n.utf8_text(source.as_bytes()).ok());
            let calls = extract_function_calls(&node, source);
            let mut info = make_entity_json(
                file,
                "go",
                "function",
                name,
                signature,
                starting_line,
                ending_line,
                parent,
                None,
                Some(&calls),
            );
            if let Some(g) = generics {
                info["generics"] = serde_json::json!(g);
            }
            let _ = writeln!(out, "{}", info.to_string());
        } else if kind == "var_spec" {
            // var_spec: child 'name', child 'type', child 'value'
            let name_node = node.child_by_field_name("name");
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let parent = parent.as_deref();
            let info = make_entity_json(
                file, "go", "variable", name, signature, starting_line, ending_line, parent, None, None,
            );
            let _ = writeln!(out, "{}", info.to_string());
        } else {
            for child in node.children(&mut node.walk()) {
                stack.push((child, parent.clone()));
            }
        }
    }
}
// DRY entity extraction for Python and Rust with doc extraction callback
fn extract_entities_with_doc<W: Write, DocFn>(
    tree: &tree_sitter::Tree,
    source: &str,
    file: &str,
    out: &mut W,
    language: &str,
    class_kind: &str,
    class_name_field: &str,
    function_kind: &str,
    function_name_field: &str,
    function_type: &str,
    doc_extractor: DocFn,
) where
    W: Write,
    DocFn: Fn(&tree_sitter::Node, &str) -> String,
{
    // use tree_sitter::{Node, TreeCursor};
    let cursor = tree.root_node().walk();
    let mut stack = vec![(cursor.node(), None::<String>)];
    while let Some((node, parent)) = stack.pop() {
        let kind = node.kind();
        if kind == class_kind {
            let name_node = node.child_by_field_name(class_name_field);
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let doc = doc_extractor(&node, source);
            let info = make_entity_json(
                file,
                language,
                "class",
                name,
                signature,
                starting_line,
                ending_line,
                None,
                Some(&doc),
                None,
            );
            let _ = writeln!(out, "{}", info.to_string());
            for child in node.children(&mut node.walk()) {
                stack.push((child, Some(name.to_string())));
            }
        } else if kind == function_kind {
            let name_node = node.child_by_field_name(function_name_field);
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let doc = doc_extractor(&node, source);
            let parent = parent.as_deref();
            let calls = extract_function_calls(&node, source);
            let info = make_entity_json(
                file,
                language,
                function_type,
                name,
                signature,
                starting_line,
                ending_line,
                parent,
                Some(&doc),
                Some(&calls),
            );
            let _ = writeln!(out, "{}", info.to_string());
        } else {
            for child in node.children(&mut node.walk()) {
                stack.push((child, parent.clone()));
            }
        }
    }
}

fn extract_csharp_entities<W: Write>(
    tree: &tree_sitter::Tree,
    source: &str,
    file: &str,
    out: &mut W,
) {
    extract_entities_generic(
        tree,
        source,
        file,
        out,
        "c_sharp",
        "class_declaration",
        "name",
        "method_declaration",
        "name",
        "function",
    );
}

fn extract_python_entities<W: Write>(
    tree: &tree_sitter::Tree,
    source: &str,
    file: &str,
    out: &mut W,
) {
    // Extract decorators for classes and functions
    // use tree_sitter::{Node, TreeCursor};
    let cursor = tree.root_node().walk();
    let mut stack = vec![(cursor.node(), None::<String>)];
    while let Some((node, parent)) = stack.pop() {
        let kind = node.kind();
        if kind == "class_definition" || kind == "function_definition" {
            for child in node.children(&mut node.walk()) {
                if child.kind() == "decorator" {
                    let name_node = child.child_by_field_name("name");
                    let name = name_node
                        .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                        .unwrap_or("");
                    let signature = source[child.byte_range()].lines().next().unwrap_or("");
                    let parent = if kind == "class_definition" {
                        node.child_by_field_name("name")
                            .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                    } else {
                        parent.as_deref()
                    };
                    let starting_line = child.start_position().row + 1;
                    let ending_line = child.end_position().row + 1;
                    let info = make_entity_json(
                        file,
                        "python",
                        "decorator",
                        name,
                        signature,
                        starting_line,
                        ending_line,
                        parent,
                        None,
                        None,
                    );
                    let _ = writeln!(out, "{}", info.to_string());
                }
            }
        }
        for child in node.children(&mut node.walk()) {
            stack.push((child, parent.clone()));
        }
    }
    extract_entities_with_doc(
        tree,
        source,
        file,
        out,
        "python",
        "class_definition",
        "name",
        "function_definition",
        "name",
        "function",
        extract_python_docstring,
    );
}

fn extract_rust_entities<W: Write>(
    tree: &tree_sitter::Tree,
    source: &str,
    file: &str,
    out: &mut W,
) {
    extract_entities_with_doc(
        tree,
        source,
        file,
        out,
        "rust",
        "struct_item",
        "name",
        "function_item",
        "name",
        "function",
        extract_rust_doc_comment,
    );
    extract_entities_with_doc(
        tree,
        source,
        file,
        out,
        "rust",
        "enum_item",
        "name",
        "function_item",
        "name",
        "function",
        extract_rust_doc_comment,
    );
    extract_entities_with_doc(
        tree,
        source,
        file,
        out,
        "rust",
        "trait_item",
        "name",
        "function_item",
        "name",
        "function",
        extract_rust_doc_comment,
    );
    // impl_item is handled separately for parent tracking
    // use tree_sitter::{Node, TreeCursor};
    let cursor = tree.root_node().walk();
    let mut stack = vec![(cursor.node(), None::<String>)];
    while let Some((node, parent)) = stack.pop() {
        let kind = node.kind();
        if kind == "impl_item" {
            let type_node = node.child_by_field_name("type");
            let parent = type_node
                .and_then(|n| n.utf8_text(source.as_bytes()).ok())
                .map(|s| s.to_string());
            for child in node.children(&mut node.walk()) {
                stack.push((child, parent.clone()));
            }
            continue;
        } else if kind == "function_item" {
            let name_node = node.child_by_field_name("name");
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let doc = extract_rust_doc_comment(&node, source);
            let parent = parent.as_deref();
            let calls = extract_function_calls(&node, source);
            let info = make_entity_json(
                file,
                "rust",
                "function",
                name,
                signature,
                starting_line,
                ending_line,
                parent,
                Some(&doc),
                Some(&calls),
            );
            let _ = writeln!(out, "{}", info.to_string());
        } else {
            for child in node.children(&mut node.walk()) {
                stack.push((child, parent.clone()));
            }
        }
    }
}

fn extract_python_docstring(node: &tree_sitter::Node, source: &str) -> String {
    // Look for the first expression_statement string literal after the colon
    for child in node.children(&mut node.walk()) {
        if child.kind() == "block" {
            for grandchild in child.children(&mut child.walk()) {
                if grandchild.kind() == "expression_statement" {
                    let expr = grandchild.child(0);
                    if let Some(expr) = expr {
                        if expr.kind() == "string" {
                            return expr
                                .utf8_text(source.as_bytes())
                                .unwrap_or("")
                                .trim_matches('"')
                                .to_string();
                        }
                    }
                }
            }
        }
    }
    String::new()
}

fn extract_rust_doc_comment(node: &tree_sitter::Node, source: &str) -> String {
    // Look for preceding line comments (/// or /** ... */)
    let mut doc_lines = Vec::new();
    let mut cur = *node;
    while let Some(prev) = cur.prev_sibling() {
        if prev.kind() == "line_comment" {
            let text = prev.utf8_text(source.as_bytes()).unwrap_or("");
            if text.trim_start().starts_with("///") {
                doc_lines.push(text.trim_start_matches("///").trim());
            }
        } else if prev.kind() == "block_comment" {
            let text = prev.utf8_text(source.as_bytes()).unwrap_or("");
            if text.trim_start().starts_with("/**") {
                doc_lines.push(text.trim_start_matches("/**").trim_end_matches("*/").trim());
            }
        } else if prev.kind().starts_with("comment") {
            // skip other comments
        } else {
            break;
        }
        cur = prev;
    }
    doc_lines.reverse();
    doc_lines.join("\n")
}

fn extract_cpp_entities<W: Write>(tree: &tree_sitter::Tree, source: &str, file: &str, out: &mut W) {
    // use tree_sitter::{Node, TreeCursor};
    let cursor = tree.root_node().walk();
    let mut stack = vec![(cursor.node(), None::<String>)];
    while let Some((node, parent)) = stack.pop() {
        match node.kind() {
            "template_declaration" => {
                // C++ template declaration
                let name = "template";
                let starting_line = node.start_position().row + 1;
                let ending_line = node.end_position().row + 1;
                let signature = source[node.byte_range()].lines().next().unwrap_or("");
                let info = make_entity_json(
                    file,
                    "cpp",
                    "template",
                    name,
                    signature,
                    starting_line,
                    ending_line,
                    parent.as_deref(),
                    None,
                    None,
                );
                let _ = writeln!(out, "{}", info.to_string());
                for child in node.children(&mut node.walk()) {
                    stack.push((child, parent.as_ref().map(|s| s.as_str().to_string())));
                }
            }
            "class_specifier" => {
                // Use generic helper for class
                let name_node = node.child_by_field_name("name");
                let name = name_node
                    .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                    .unwrap_or("");
                let starting_line = node.start_position().row + 1;
                let ending_line = node.end_position().row + 1;
                let signature = source[node.byte_range()].lines().next().unwrap_or("");
                let info = make_entity_json(
                    file, "cpp", "class", name, signature, starting_line, ending_line, None, None, None,
                );
                let _ = writeln!(out, "{}", info.to_string());
                for child in node.children(&mut node.walk()) {
                    stack.push((child, Some(name.to_string())));
                }
            }
            "function_definition" => {
                // Custom function name extraction
                let decl_node = node.child_by_field_name("declarator");
                let name = decl_node
                    .and_then(|n| n.child_by_field_name("declarator"))
                    .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                    .unwrap_or("");
                let starting_line = node.start_position().row + 1;
                let ending_line = node.end_position().row + 1;
                let signature = source[node.byte_range()].lines().next().unwrap_or("");
                let parent_ref = parent.as_deref();
                let calls = extract_function_calls(&node, source);
                let info = make_entity_json(
                    file, "cpp", "function", name, signature, starting_line, ending_line, parent_ref, None, Some(&calls),
                );
                let _ = writeln!(out, "{}", info.to_string());
                for child in node.children(&mut node.walk()) {
                    stack.push((child, parent.as_ref().map(|s| s.as_str().to_string())));
                }
            }
            _ => {
                for child in node.children(&mut node.walk()) {
                    stack.push((child, parent.as_ref().map(|s| s.as_str().to_string())));
                }
            }
        }
    }
}

fn extract_entities_generic<W: Write>(
    tree: &tree_sitter::Tree,
    source: &str,
    file: &str,
    out: &mut W,
    language: &str,
    class_kind: &str,
    class_name_field: &str,
    function_kind: &str,
    function_name_field: &str,
    function_type: &str,
) {
    // use tree_sitter::{Node, TreeCursor};
    let cursor = tree.root_node().walk();
    let mut stack = vec![(cursor.node(), None::<String>)];
    while let Some((node, parent)) = stack.pop() {
        let kind = node.kind();
        if kind == class_kind {
            let name_node = node.child_by_field_name(class_name_field);
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let info = make_entity_json(
                file, language, "class", name, signature, starting_line, ending_line, None, None, None,
            );
            let _ = writeln!(out, "{}", info.to_string());
            for child in node.children(&mut node.walk()) {
                stack.push((child, Some(name.to_string())));
            }
        } else if kind == function_kind {
            let name_node = node.child_by_field_name(function_name_field);
            let name = name_node
                .map(|n| n.utf8_text(source.as_bytes()).unwrap_or(""))
                .unwrap_or("");
            let starting_line = node.start_position().row + 1;
            let ending_line = node.end_position().row + 1;
            let signature = source[node.byte_range()].lines().next().unwrap_or("");
            let parent = parent.as_deref();
            let calls = extract_function_calls(&node, source);
            let info = make_entity_json(
                file,
                language,
                function_type,
                name,
                signature,
                starting_line,
                ending_line,
                parent,
                None,
                Some(&calls),
            );
            let _ = writeln!(out, "{}", info.to_string());
        } else {
            for child in node.children(&mut node.walk()) {
                stack.push((child, parent.clone()));
            }
        }
    }
}

// Refactored extractors to use the generic helper
fn extract_java_entities<W: Write>(
    tree: &tree_sitter::Tree,
    source: &str,
    file: &str,
    out: &mut W,
) {
    extract_entities_generic(
        tree,
        source,
        file,
        out,
        "java",
        "class_declaration",
        "name",
        "method_declaration",
        "name",
        "function",
    );
}

fn extract_js_ts_entities<W: Write>(
    tree: &tree_sitter::Tree,
    source: &str,
    file: &str,
    out: &mut W,
    language: &str,
) {
    extract_entities_generic(
        tree,
        source,
        file,
        out,
        language,
        "class_declaration",
        "name",
        "function_declaration",
        "name",
        "function",
    );
}

pub fn index_repository_with_args(root_path: &str, output_file: &str) -> Result<(), String> {
    println!("Indexing repository with Tree-sitter at '{root_path}'...");
    let start_time = Instant::now();
    let mut out = fs::File::create(output_file)
        .map_err(|e| format!("[goose repo] Failed to create index file '{output_file}': {e}"))?;

    let mut files_indexed = 0usize;
    let mut entities_indexed = 0usize;
    let mut errors = Vec::new();

    let walker = WalkBuilder::new(root_path)
        .standard_filters(true)
        .add_custom_ignore_filename(".gitignore")
        .build();
    for result in walker {
        let entry = match result {
            Ok(e) => e,
            Err(e) => {
                errors.push(format!("[walkdir] {e}"));
                continue;
            }
        };
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let lang = match detect_language(path) {
            Some(l) => l,
            None => continue,
        };
        let language = match get_language(lang) {
            Some(l) => l,
            None => {
                errors.push(format!(
                    "[lang] No Tree-sitter language for {lang} in file {}",
                    path.display()
                ));
                continue;
            }
        };
        let source = match fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                errors.push(format!("[read] {}: {e}", path.display()));
                continue;
            }
        };
        let mut parser = Parser::new();
        if let Err(e) = parser.set_language(language) {
            errors.push(format!("[parser] {}: {e}", path.display()));
            continue;
        }
        let tree = match parser.parse(&source, None) {
            Some(t) => t,
            None => {
                errors.push(format!("[parse] {}: failed to parse", path.display()));
                continue;
            }
        };
        let file_str = path.display().to_string();
        let _before = entities_indexed;
        match lang {
            "swift" => {
                let mut counting_out = CountingWriter::new(&mut out, &mut entities_indexed);
                extract_swift_entities(&tree, &source, &file_str, &mut counting_out);
            }
            "python" => {
                let mut counting_out = CountingWriter::new(&mut out, &mut entities_indexed);
                extract_python_entities(&tree, &source, &file_str, &mut counting_out);
            }
            "rust" => {
                let mut counting_out = CountingWriter::new(&mut out, &mut entities_indexed);
                extract_rust_entities(&tree, &source, &file_str, &mut counting_out);
            }
            "cpp" => {
                let mut counting_out = CountingWriter::new(&mut out, &mut entities_indexed);
                extract_cpp_entities(&tree, &source, &file_str, &mut counting_out);
            }
            "java" => {
                let mut counting_out = CountingWriter::new(&mut out, &mut entities_indexed);
                extract_java_entities(&tree, &source, &file_str, &mut counting_out);
            }
            "go" => {
                let mut counting_out = CountingWriter::new(&mut out, &mut entities_indexed);
                extract_go_entities(&tree, &source, &file_str, &mut counting_out);
            }
            "javascript" => {
                let mut counting_out = CountingWriter::new(&mut out, &mut entities_indexed);
                extract_js_ts_entities(&tree, &source, &file_str, &mut counting_out, "javascript");
            }
            "typescript" => {
                let mut counting_out = CountingWriter::new(&mut out, &mut entities_indexed);
                extract_js_ts_entities(&tree, &source, &file_str, &mut counting_out, "typescript");
            }
            "c_sharp" => {
                let mut counting_out = CountingWriter::new(&mut out, &mut entities_indexed);
                extract_csharp_entities(&tree, &source, &file_str, &mut counting_out);
            }
            _ => {
                let info = json!({
                    "file": file_str,
                    "language": lang,
                });
                if let Err(e) = writeln!(out, "{}", info.to_string()) {
                    errors.push(format!("[write] {}: {e}", file_str));
                }
                entities_indexed += 1;
            }
        }
        files_indexed += 1;
    }
    let elapsed = start_time.elapsed();
    println!("Indexing complete. Indexed {files_indexed} files, {entities_indexed} entities in {:.2?}.", elapsed);
    if !errors.is_empty() {
        eprintln!("Encountered {} errors during indexing:", errors.len());
        for err in &errors {
            eprintln!("  {err}");
        }
        return Err(format!("Encountered {} errors during indexing.", errors.len()));
    }
    Ok(())
}

/// A writer that increments a counter for each line written (i.e., each entity output)
pub struct CountingWriter<'a, W: Write> {
    inner: &'a mut W,
    counter: &'a mut usize,
}

impl<'a, W: Write> CountingWriter<'a, W> {
    pub fn new(inner: &'a mut W, counter: &'a mut usize) -> Self {
        CountingWriter { inner, counter }
    }
}

impl<'a, W: Write> Write for CountingWriter<'a, W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let s = std::str::from_utf8(buf).unwrap_or("");
        let lines = s.matches('\n').count();
        *self.counter += lines;
        self.inner.write(buf)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.inner.flush()
    }
}
