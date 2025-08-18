use anyhow::Result;
use ignore::WalkBuilder;
use serde::{Serialize, Deserialize};
use std::collections::HashSet;
use std::fs::File;
use std::io::{self, Write};
use std::path::Path;
use std::time::{Duration, Instant};
use tree_sitter::{Language, Parser, Node};

// Language crates (feature-level currently all under single feature)
use tree_sitter_c_sharp as ts_c_sharp;
use tree_sitter_cpp as ts_cpp;
use tree_sitter_go as ts_go;
use tree_sitter_java as ts_java;
use tree_sitter_javascript as ts_javascript;
use tree_sitter_python as ts_python;
use tree_sitter_rust as ts_rust;
use tree_sitter_swift as ts_swift;
use tree_sitter_typescript as ts_typescript;

#[derive(Debug, Clone, Default, Serialize)]
pub struct RepoIndexStats {
    pub files_indexed: usize,
    pub entities_indexed: usize,
    pub duration: Duration,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Entity<'a> {
    pub file: String,
    pub language: &'a str,
    pub kind: &'static str, // class | function | method | other
    pub name: String,
    pub parent: Option<String>,
    pub signature: String,
    pub start_line: usize,
    pub end_line: usize,
    pub calls: Option<Vec<String>>, // simple callee names
    pub doc: Option<String>,
}

#[derive(Clone, Copy, Debug)]
pub struct Progress<'a> {
    pub current_file: Option<&'a Path>,
    pub files_indexed: usize,
    pub entities_indexed: usize,
}

pub type ProgressCallback<'a> = dyn Fn(Progress<'_>) + Send + Sync + 'a;

pub struct RepoIndexOptions<'a> {
    pub root: &'a Path,
    pub output: RepoIndexOutput<'a>,
    pub include_langs: Option<HashSet<&'a str>>, // if None index all supported
    pub progress: Option<&'a ProgressCallback<'a>>,
}

pub enum RepoIndexOutput<'a> {
    FilePath(&'a Path),
    Writer(&'a mut dyn Write),
}

pub struct RepoIndexOptionsBuilder<'a> {
    root: Option<&'a Path>,
    output: Option<RepoIndexOutput<'a>>,
    include_langs: Option<HashSet<&'a str>>,
    progress: Option<&'a ProgressCallback<'a>>,
}

impl<'a> Default for RepoIndexOptionsBuilder<'a> { fn default() -> Self { Self { root: None, output: None, include_langs: None, progress: None } } }

impl<'a> RepoIndexOptions<'a> { pub fn builder() -> RepoIndexOptionsBuilder<'a> { RepoIndexOptionsBuilder::default() } }

impl<'a> RepoIndexOptionsBuilder<'a> {
    pub fn root(mut self, root: &'a Path) -> Self { self.root = Some(root); self }
    pub fn output_file(mut self, path: &'a Path) -> Self { self.output = Some(RepoIndexOutput::FilePath(path)); self }
    pub fn output_writer(mut self, w: &'a mut dyn Write) -> Self { self.output = Some(RepoIndexOutput::Writer(w)); self }
    pub fn include_langs(mut self, langs: HashSet<&'a str>) -> Self { self.include_langs = Some(langs); self }
    pub fn progress(mut self, cb: &'a ProgressCallback<'a>) -> Self { self.progress = Some(cb); self }
    pub fn build(self) -> RepoIndexOptions<'a> { RepoIndexOptions { root: self.root.expect("root required"), output: self.output.expect("output required"), include_langs: self.include_langs, progress: self.progress } }
}

struct CountingWriter<W: Write> { inner: W, counter: usize }
impl<W: Write> CountingWriter<W> { fn new(inner: W) -> Self { Self { inner, counter: 0 } } fn count(&self) -> usize { self.counter } }
impl<W: Write> Write for CountingWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> { let written = self.inner.write(buf)?; self.counter += buf[..written].iter().filter(|b| **b == b'\n').count(); Ok(written) }
    fn flush(&mut self) -> io::Result<()> { self.inner.flush() }
}

pub(crate) fn detect_language(path: &Path) -> Option<&'static str> {
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
pub(crate) fn lang_to_ts(lang: &str) -> Option<Language> {
    Some(match lang {
        "rust" => ts_rust::LANGUAGE.into(),
        "python" => ts_python::LANGUAGE.into(),
        "javascript" => ts_javascript::LANGUAGE.into(),
        "typescript" => ts_typescript::LANGUAGE_TYPESCRIPT.into(),
        "cpp" => ts_cpp::LANGUAGE.into(),
        "java" => ts_java::LANGUAGE.into(),
        "c_sharp" => ts_c_sharp::LANGUAGE.into(),
        "swift" => ts_swift::LANGUAGE.into(),
        "go" => ts_go::LANGUAGE.into(),
        _ => return None,
    })
}

pub fn index_repository(opts: RepoIndexOptions<'_>) -> Result<RepoIndexStats> {
    let start = Instant::now();
    let mut entities = 0usize; // we will snapshot from writer.counter
    let mut files = 0usize;

    let mut file_handle; // scoped for writer
    let writer: Box<dyn Write> = match opts.output {
        RepoIndexOutput::FilePath(p) => {
            file_handle = Box::new(File::create(p)?);
            Box::new(&mut *file_handle)
        }
        RepoIndexOutput::Writer(w) => Box::new(w),
    };
    // Counting writer increments entities when we newline an entity JSON
    let mut cw = CountingWriter::new(writer);

    let walker = WalkBuilder::new(opts.root).standard_filters(true).add_custom_ignore_filename(".gitignore").build();
    for dent in walker {
        let dent = match dent { Ok(d) => d, Err(_) => continue };
        let path = dent.path();
        if !path.is_file() { continue; }
        let lang = match detect_language(path) { Some(l) => l, None => continue };
        if let Some(include) = &opts.include_langs { if !include.contains(lang) { continue; } }
        let language = match lang_to_ts(lang) { Some(l) => l, None => continue };
        let src = match std::fs::read_to_string(path) { Ok(s) => s, Err(_) => continue };
        let mut parser = Parser::new();
    if parser.set_language(&language).is_err() { continue; }
        let tree = match parser.parse(&src, None) { Some(t) => t, None => continue };
        let mut entities_local: Vec<Entity> = Vec::new();
        extract_entities(lang, &tree, &src, path, &mut entities_local);
        for e in entities_local.into_iter() {
            writeln!(cw, "{}", serde_json::to_string(&e)?)?;
        }
        files += 1;
        cw.flush()?;
        entities = cw.count();
        if let Some(cb) = opts.progress {
            cb(Progress { current_file: Some(path), files_indexed: files, entities_indexed: entities });
        }
    }
    Ok(RepoIndexStats { files_indexed: files, entities_indexed: entities, duration: start.elapsed() })
}

// ---------------- Entity Extraction ----------------

pub(crate) fn extract_entities<'a>(lang: &'a str, tree: &tree_sitter::Tree, src: &'a str, path: &Path, out: &mut Vec<Entity<'a>>) {
    match lang {
        "rust" => generic_class_function_walk(lang, tree, src, path, out, &RustLangSpec),
        "python" => generic_class_function_walk(lang, tree, src, path, out, &PythonLangSpec),
        "javascript" => generic_class_function_walk(lang, tree, src, path, out, &JsLangSpec),
        "typescript" => generic_class_function_walk(lang, tree, src, path, out, &TsLangSpec),
        "java" => generic_class_function_walk(lang, tree, src, path, out, &JavaLangSpec),
        "go" => generic_class_function_walk(lang, tree, src, path, out, &GoLangSpec),
        "cpp" => generic_class_function_walk(lang, tree, src, path, out, &CppLangSpec),
        "c_sharp" => generic_class_function_walk(lang, tree, src, path, out, &CSharpLangSpec),
        "swift" => generic_class_function_walk(lang, tree, src, path, out, &SwiftLangSpec),
        _ => {
            // fallback: whole file summary as one entity
            out.push(Entity {
                file: path.display().to_string(),
                language: lang,
                kind: "file",
                name: path.file_name().and_then(|s| s.to_str()).unwrap_or("").to_string(),
                parent: None,
                signature: src.lines().next().unwrap_or("").to_string(),
                start_line: 1,
                end_line: src.lines().count(),
                calls: None,
                doc: None,
            });
        }
    }
}

// Spec describing language-specific node kinds & field names
trait LangSpec { fn class_kind(&self) -> &'static str; fn class_name_field(&self) -> &'static str; fn function_kind(&self) -> &'static str; fn function_name_field(&self) -> &'static str; }

struct RustLangSpec; impl LangSpec for RustLangSpec { fn class_kind(&self) -> &'static str { "struct_item" } fn class_name_field(&self) -> &'static str { "name" } fn function_kind(&self) -> &'static str { "function_item" } fn function_name_field(&self) -> &'static str { "name" } }
struct PythonLangSpec; impl LangSpec for PythonLangSpec { fn class_kind(&self) -> &'static str { "class_definition" } fn class_name_field(&self) -> &'static str { "name" } fn function_kind(&self) -> &'static str { "function_definition" } fn function_name_field(&self) -> &'static str { "name" } }
struct JsLangSpec; impl LangSpec for JsLangSpec { fn class_kind(&self) -> &'static str { "class_declaration" } fn class_name_field(&self) -> &'static str { "name" } fn function_kind(&self) -> &'static str { "function_declaration" } fn function_name_field(&self) -> &'static str { "name" } }
struct TsLangSpec; impl LangSpec for TsLangSpec { fn class_kind(&self) -> &'static str { "class_declaration" } fn class_name_field(&self) -> &'static str { "name" } fn function_kind(&self) -> &'static str { "function_declaration" } fn function_name_field(&self) -> &'static str { "name" } }
struct JavaLangSpec; impl LangSpec for JavaLangSpec { fn class_kind(&self) -> &'static str { "class_declaration" } fn class_name_field(&self) -> &'static str { "name" } fn function_kind(&self) -> &'static str { "method_declaration" } fn function_name_field(&self) -> &'static str { "name" } }
struct GoLangSpec; impl LangSpec for GoLangSpec { fn class_kind(&self) -> &'static str { "type_declaration" } fn class_name_field(&self) -> &'static str { "name" } fn function_kind(&self) -> &'static str { "function_declaration" } fn function_name_field(&self) -> &'static str { "name" } }
struct CppLangSpec; impl LangSpec for CppLangSpec { fn class_kind(&self) -> &'static str { "class_specifier" } fn class_name_field(&self) -> &'static str { "name" } fn function_kind(&self) -> &'static str { "function_definition" } fn function_name_field(&self) -> &'static str { "declarator" } }
struct CSharpLangSpec; impl LangSpec for CSharpLangSpec { fn class_kind(&self) -> &'static str { "class_declaration" } fn class_name_field(&self) -> &'static str { "name" } fn function_kind(&self) -> &'static str { "method_declaration" } fn function_name_field(&self) -> &'static str { "name" } }
struct SwiftLangSpec; impl LangSpec for SwiftLangSpec { fn class_kind(&self) -> &'static str { "class_declaration" } fn class_name_field(&self) -> &'static str { "name" } fn function_kind(&self) -> &'static str { "function_declaration" } fn function_name_field(&self) -> &'static str { "name" } }

fn node_text<'a>(node: Node<'a>, src: &'a str) -> &'a str { node.utf8_text(src.as_bytes()).unwrap_or("") }

fn generic_class_function_walk<'a>(lang: &'a str, tree: &tree_sitter::Tree, src: &'a str, path: &Path, out: &mut Vec<Entity<'a>>, spec: &dyn LangSpec) {
    let mut stack = vec![(tree.root_node(), None::<String>)];
    while let Some((node, parent)) = stack.pop() {
        let kind = node.kind();
        if kind == spec.class_kind() {
            let name = node.child_by_field_name(spec.class_name_field()).map(|n| node_text(n, src)).unwrap_or("");
            let entity = Entity {
                file: path.display().to_string(),
                language: lang,
                kind: "class",
                name: name.to_string(),
                parent: parent.clone(),
                signature: extract_signature(&node, src).to_string(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                calls: None,
                doc: extract_doc_comments(&node, src),
            };
            out.push(entity);
            for child in node.children(&mut node.walk()) { stack.push((child, Some(name.to_string()))); }
        } else if kind == spec.function_kind() {
            let name = node.child_by_field_name(spec.function_name_field()).map(|n| node_text(n, src)).unwrap_or("");
            let calls = collect_call_idents(node, src);
            out.push(Entity {
                file: path.display().to_string(),
                language: lang,
                kind: "function",
                name: name.to_string(),
                parent: parent.clone(),
                signature: extract_signature(&node, src).to_string(),
                start_line: node.start_position().row + 1,
                end_line: node.end_position().row + 1,
                calls: if calls.is_empty() { None } else { Some(calls) },
                doc: extract_doc_comments(&node, src),
            });
            for child in node.children(&mut node.walk()) { stack.push((child, parent.clone())); }
        } else {
            for child in node.children(&mut node.walk()) { stack.push((child, parent.clone())); }
        }
    }
}

fn extract_signature<'a>(node: &Node<'a>, src: &'a str) -> &'a str {
    let text = src.get(node.byte_range()).unwrap_or("");
    // Up to first '{' or newline (whichever shorter) for brace languages; else first line
    if let Some(idx) = text.find('{') { &text[..idx].trim_end() } else { text.lines().next().unwrap_or("") }
}

fn extract_doc_comments<'a>(node: &Node<'a>, src: &'a str) -> Option<String> {
    let start_byte = node.start_byte();
    let prefix = &src[..start_byte.min(src.len())];
    let mut lines: Vec<&str> = prefix.lines().collect();
    let mut doc_rev: Vec<String> = Vec::new();
    while let Some(&line) = lines.last() {
        let trimmed = line.trim();
        if trimmed.is_empty() { break; }
        if trimmed.starts_with("///") || trimmed.starts_with("//") || trimmed.starts_with('#') || trimmed.starts_with("/*") || trimmed.starts_with('*') {
            doc_rev.push(trimmed.trim_start_matches("///").trim_start_matches("//").trim_start_matches('#').trim_start_matches('*').trim().to_string());
            lines.pop();
        } else { break; }
    }
    if doc_rev.is_empty() { None } else { doc_rev.reverse(); Some(doc_rev.join("\n")) }
}

fn collect_call_idents(func_node: Node, src: &str) -> Vec<String> {
    let mut calls = Vec::new();
    let mut stack = vec![func_node];
    while let Some(node) = stack.pop() {
        let kind = node.kind();
        // Heuristic: identifier nodes inside call_expression or function_call
        if kind == "call_expression" || kind == "function_call" { // language dependent
            if let Some(child) = node.child_by_field_name("function") { calls.push(node_text(child, src).to_string()); }
            else if let Some(first) = node.child(0) { if first.is_named() { calls.push(node_text(first, src).to_string()); } }
        }
        for child in node.children(&mut node.walk()) { stack.push(child); }
    }
    calls.sort(); calls.dedup(); calls
}
