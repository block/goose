use ignore::gitignore::Gitignore;
use lru::LruCache;
use rmcp::{
    model::{CallToolResult, Content, ErrorCode, ErrorData, Role},
    schemars::JsonSchema,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    time::SystemTime,
};
use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor, Tree};

use super::lang;

/// Parameters for the analyze tool
#[derive(Debug, Serialize, Deserialize, JsonSchema)]
pub struct AnalyzeParams {
    /// Path to analyze (file or directory)
    pub path: String,

    /// Analysis depth: "structure" (fast) or "semantic" (detailed)
    #[serde(default = "default_analysis_depth")]
    pub depth: String,

    /// Focus on specific symbol
    pub focus: Option<String>,

    /// Maximum directory depth
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
}

fn default_analysis_depth() -> String {
    "structure".to_string()
}

fn default_max_depth() -> u32 {
    3
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AnalysisResult {
    functions: Vec<FunctionInfo>,
    classes: Vec<ClassInfo>,
    imports: Vec<String>,
    // Semantic analysis fields
    calls: Vec<CallInfo>,
    references: Vec<ReferenceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct FunctionInfo {
    name: String,
    line: usize,
    params: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ClassInfo {
    name: String,
    line: usize,
    methods: Vec<FunctionInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct CallInfo {
    caller_name: Option<String>, // Function containing this call
    callee_name: String,         // Function being called
    line: usize,
    column: usize,
    context: String, // Line of code containing the call
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ReferenceInfo {
    symbol: String,
    ref_type: ReferenceType,
    line: usize,
    context: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
enum ReferenceType {
    Definition,
    Call,
    Import,
    Assignment,
}

/// Code analyzer with caching and tree-sitter parsing
pub struct CodeAnalyzer {
    parser_cache: Arc<Mutex<HashMap<String, Arc<Mutex<Parser>>>>>,
    analysis_cache: Arc<Mutex<LruCache<(PathBuf, SystemTime), AnalysisResult>>>,
}

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self {
            parser_cache: Arc::new(Mutex::new(HashMap::new())),
            analysis_cache: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(100).unwrap(),
            ))),
        }
    }

    /// Main analyze entry point
    pub async fn analyze(
        &self,
        params: AnalyzeParams,
        path: PathBuf,
        ignore_patterns: &Gitignore,
    ) -> Result<CallToolResult, ErrorData> {
        // Check if path is ignored
        if self.is_ignored(&path, ignore_patterns) {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "Access to '{}' is restricted by .gooseignore",
                    path.display()
                ),
                None,
            ));
        }

        // Check if path exists
        if !path.exists() {
            return Err(ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Path '{}' does not exist", path.display()),
                None,
            ));
        }

        let mut output = String::new();

        if path.is_file() {
            // Analyze single file
            let result = self.analyze_file(&path, &params.depth).await?;
            output.push_str(&self.format_analysis_result(&path, &result));
        } else {
            // Analyze directory
            output.push_str(&format!("# Code Analysis: {}\n\n", path.display()));
            self.analyze_directory(&path, &mut output, 0, params.max_depth, ignore_patterns)
                .await?;
        }

        // If focus is specified, filter results
        if let Some(focus) = &params.focus {
            output = self.filter_by_focus(&output, focus);
        }

        Ok(CallToolResult::success(vec![
            Content::text(output.clone()).with_audience(vec![Role::Assistant]),
            Content::text(output)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }

    // Helper method to check if a path should be ignored
    fn is_ignored(&self, path: &Path, ignore_patterns: &Gitignore) -> bool {
        ignore_patterns.matched(path, false).is_ignore()
    }

    // Helper method to analyze a single file
    async fn analyze_file(&self, path: &Path, depth: &str) -> Result<AnalysisResult, ErrorData> {
        // Check cache first
        let metadata = std::fs::metadata(path).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to get file metadata: {}", e),
                None,
            )
        })?;

        let modified = metadata.modified().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to get modification time: {}", e),
                None,
            )
        })?;

        // Check cache
        {
            let mut cache = self.analysis_cache.lock().unwrap();
            if let Some(cached) = cache.get(&(path.to_path_buf(), modified)) {
                return Ok(cached.clone());
            }
        }

        // Read file content
        let content = std::fs::read_to_string(path).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to read file: {}", e),
                None,
            )
        })?;

        // Get language
        let language = lang::get_language_identifier(path);
        if language.is_empty() {
            // Unsupported language, return empty result
            return Ok(AnalysisResult {
                functions: vec![],
                classes: vec![],
                imports: vec![],
                calls: vec![],
                references: vec![],
            });
        }

        // Check if we support this language for parsing
        let supported = matches!(
            language,
            "python" | "rust" | "javascript" | "typescript" | "go" | "java"
        );

        if !supported {
            // Language detected but not supported for parsing, return empty result
            return Ok(AnalysisResult {
                functions: vec![],
                classes: vec![],
                imports: vec![],
                calls: vec![],
                references: vec![],
            });
        }

        // Get or create parser for this language
        let parser_arc = self.get_or_create_parser(language)?;

        // Parse the file
        let tree = {
            let mut parser = parser_arc.lock().unwrap();
            parser.parse(&content, None).ok_or_else(|| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    "Failed to parse file".to_string(),
                    None,
                )
            })?
        };

        // Extract information based on language
        let result = self.extract_code_elements_with_depth(&tree, &content, language, depth)?;

        // Cache the result
        {
            let mut cache = self.analysis_cache.lock().unwrap();
            cache.put((path.to_path_buf(), modified), result.clone());
        }

        Ok(result)
    }

    // Helper method to get or create a parser for a language
    fn get_or_create_parser(&self, language: &str) -> Result<Arc<Mutex<Parser>>, ErrorData> {
        let mut cache = self.parser_cache.lock().unwrap();

        if let Some(parser) = cache.get(language) {
            return Ok(Arc::clone(parser));
        }

        let mut parser = Parser::new();
        let language_config = match language {
            "python" => tree_sitter_python::LANGUAGE,
            "rust" => tree_sitter_rust::LANGUAGE,
            "javascript" | "typescript" => tree_sitter_javascript::LANGUAGE,
            "go" => tree_sitter_go::LANGUAGE,
            "java" => tree_sitter_java::LANGUAGE,
            _ => {
                return Err(ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Unsupported language: {}", language),
                    None,
                ))
            }
        };

        parser.set_language(&language_config.into()).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to set language: {}", e),
                None,
            )
        })?;

        let parser_arc = Arc::new(Mutex::new(parser));
        cache.insert(language.to_string(), Arc::clone(&parser_arc));
        Ok(parser_arc)
    }

    // Helper method to extract code elements with optional semantic analysis
    fn extract_code_elements_with_depth(
        &self,
        tree: &Tree,
        source: &str,
        language: &str,
        depth: &str,
    ) -> Result<AnalysisResult, ErrorData> {
        // First get the structural analysis
        let mut result = self.extract_code_elements(tree, source, language)?;

        // If semantic depth is requested, also extract calls
        if depth == "semantic" {
            let calls = self.extract_calls(tree, source, language)?;
            result.calls = calls;

            // Also populate references from the calls
            for call in &result.calls {
                result.references.push(ReferenceInfo {
                    symbol: call.callee_name.clone(),
                    ref_type: ReferenceType::Call,
                    line: call.line,
                    context: call.context.clone(),
                });
            }
        }

        Ok(result)
    }

    // Helper method to extract code elements from the parse tree
    fn extract_code_elements(
        &self,
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<AnalysisResult, ErrorData> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

        // Create queries based on language
        let query_str = match language {
            "python" => {
                r#"
                (function_definition name: (identifier) @func)
                (class_definition name: (identifier) @class)
                (import_statement) @import
                (import_from_statement) @import
            "#
            }
            "rust" => {
                r#"
                (function_item name: (identifier) @func)
                (impl_item type: (type_identifier) @class)
                (struct_item name: (type_identifier) @struct)
                (use_declaration) @import
            "#
            }
            "javascript" | "typescript" => {
                r#"
                (function_declaration name: (identifier) @func)
                (class_declaration name: (identifier) @class)
                (import_statement) @import
            "#
            }
            "go" => {
                r#"
                (function_declaration name: (identifier) @func)
                (method_declaration name: (field_identifier) @func)
                (type_declaration (type_spec name: (type_identifier) @struct))
                (import_declaration) @import
            "#
            }
            "java" => {
                r#"
                (method_declaration name: (identifier) @func)
                (class_declaration name: (identifier) @class)
                (import_declaration) @import
            "#
            }
            _ => {
                return Ok(AnalysisResult {
                    functions: vec![],
                    classes: vec![],
                    imports: vec![],
                    calls: vec![],
                    references: vec![],
                })
            }
        };

        let query = Query::new(&tree.language(), query_str).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create query: {}", e),
                None,
            )
        })?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let text = &source[node.byte_range()];
                let line = source[..node.start_byte()].lines().count() + 1;

                match query.capture_names()[capture.index as usize] {
                    "func" => {
                        functions.push(FunctionInfo {
                            name: text.to_string(),
                            line,
                            params: vec![], // Simplified for now
                        });
                    }
                    "class" | "struct" => {
                        classes.push(ClassInfo {
                            name: text.to_string(),
                            line,
                            methods: vec![], // Simplified for now
                        });
                    }
                    "import" => {
                        imports.push(text.to_string());
                    }
                    _ => {}
                }
            }
        }

        Ok(AnalysisResult {
            functions,
            classes,
            imports,
            calls: vec![],
            references: vec![],
        })
    }

    // Helper method to get language-specific query for finding function calls
    fn get_call_query(&self, language: &str) -> &'static str {
        match language {
            "python" => {
                r#"
                ; Function calls
                (call
                  function: (identifier) @function.call)
                
                ; Method calls
                (call
                  function: (attribute
                    attribute: (identifier) @method.call))
            "#
            }

            "rust" => {
                r#"
                ; Function calls
                (call_expression
                  function: (identifier) @function.call)
                
                ; Method calls
                (call_expression
                  function: (field_expression
                    field: (field_identifier) @method.call))
                
                ; Associated function calls (e.g., Type::method())
                (call_expression
                  function: (scoped_identifier
                    name: (identifier) @scoped.call))
                
                ; Macro calls (often contain function-like behavior)
                (macro_invocation
                  macro: (identifier) @macro.call)
            "#
            }

            "javascript" | "typescript" => {
                r#"
                ; Function calls
                (call_expression
                  function: (identifier) @function.call)
                
                ; Method calls
                (call_expression
                  function: (member_expression
                    property: (property_identifier) @method.call))
                
                ; Constructor calls
                (new_expression
                  constructor: (identifier) @constructor.call)
            "#
            }

            "go" => {
                r#"
                ; Function calls
                (call_expression
                  function: (identifier) @function.call)
                
                ; Method calls
                (call_expression
                  function: (selector_expression
                    field: (field_identifier) @method.call))
            "#
            }

            "java" => {
                r#"
                ; Method invocations
                (method_invocation
                  name: (identifier) @method.call)
                
                ; Constructor calls
                (object_creation_expression
                  type: (type_identifier) @constructor.call)
            "#
            }

            _ => "",
        }
    }

    // Helper method to extract function calls from the parse tree
    fn extract_calls(
        &self,
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<Vec<CallInfo>, ErrorData> {
        let mut calls = Vec::new();

        // Get language-specific call query
        let query_str = self.get_call_query(language);
        if query_str.is_empty() {
            return Ok(calls); // No call query for this language
        }

        let query = Query::new(&tree.language(), query_str).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create call query: {}", e),
                None,
            )
        })?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        while let Some(match_) = matches.next() {
            for capture in match_.captures {
                let node = capture.node;
                let text = &source[node.byte_range()];
                let start_pos = node.start_position();

                // Get the line of code for context
                let line_start = source[..node.start_byte()]
                    .rfind('\n')
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let line_end = source[node.end_byte()..]
                    .find('\n')
                    .map(|i| node.end_byte() + i)
                    .unwrap_or(source.len());
                let context = source[line_start..line_end].trim().to_string();

                // Find the containing function
                let caller_name = self.find_containing_function(&node, source, language);

                // Add the call based on capture name
                match query.capture_names()[capture.index as usize] {
                    "function.call" | "method.call" | "scoped.call" | "macro.call"
                    | "constructor.call" => {
                        calls.push(CallInfo {
                            caller_name,
                            callee_name: text.to_string(),
                            line: start_pos.row + 1,
                            column: start_pos.column,
                            context,
                        });
                    }
                    _ => {}
                }
            }
        }

        Ok(calls)
    }

    // Helper method to find which function contains a given node
    fn find_containing_function(
        &self,
        node: &tree_sitter::Node,
        source: &str,
        language: &str,
    ) -> Option<String> {
        let mut current = *node;

        // Walk up the tree to find a function definition
        while let Some(parent) = current.parent() {
            let kind = parent.kind();

            // Check for function-like nodes based on language
            let is_function = match language {
                "python" => kind == "function_definition",
                "rust" => kind == "function_item" || kind == "impl_item",
                "javascript" | "typescript" => {
                    kind == "function_declaration"
                        || kind == "method_definition"
                        || kind == "arrow_function"
                }
                "go" => kind == "function_declaration" || kind == "method_declaration",
                "java" => kind == "method_declaration" || kind == "constructor_declaration",
                _ => false,
            };

            if is_function {
                // Try to extract the function name
                for i in 0..parent.child_count() {
                    if let Some(child) = parent.child(i) {
                        // Look for identifier nodes that represent the function name
                        if child.kind() == "identifier"
                            || child.kind() == "field_identifier"
                            || child.kind() == "property_identifier"
                        {
                            // For Python, skip the first identifier if it's 'def'
                            if language == "python" && i == 0 {
                                continue;
                            }
                            return Some(source[child.byte_range()].to_string());
                        }

                        // For Rust impl blocks, look for the type
                        if language == "rust"
                            && kind == "impl_item"
                            && child.kind() == "type_identifier"
                        {
                            return Some(format!("impl {}", &source[child.byte_range()]));
                        }
                    }
                }
            }

            current = parent;
        }

        None // No containing function found (module-level call)
    }

    // Helper method to analyze a directory recursively
    async fn analyze_directory(
        &self,
        path: &Path,
        output: &mut String,
        depth: u32,
        max_depth: u32,
        ignore_patterns: &Gitignore,
    ) -> Result<(), ErrorData> {
        if depth >= max_depth {
            return Ok(());
        }

        let entries = std::fs::read_dir(path).map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to read directory: {}", e),
                None,
            )
        })?;

        for entry in entries {
            let entry = entry.map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to read directory entry: {}", e),
                    None,
                )
            })?;

            let entry_path = entry.path();

            // Skip ignored paths
            if self.is_ignored(&entry_path, ignore_patterns) {
                continue;
            }

            if entry_path.is_file() {
                // Only analyze supported file types
                let lang = lang::get_language_identifier(&entry_path);
                if !lang.is_empty() {
                    let result = self.analyze_file(&entry_path, "structure").await?;
                    if !result.functions.is_empty() || !result.classes.is_empty() {
                        output.push_str(&self.format_analysis_result(&entry_path, &result));
                    }
                }
            } else if entry_path.is_dir() {
                // Recurse into subdirectory
                Box::pin(self.analyze_directory(
                    &entry_path,
                    output,
                    depth + 1,
                    max_depth,
                    ignore_patterns,
                ))
                .await?;
            }
        }

        Ok(())
    }

    // Helper method to format analysis results
    fn format_analysis_result(&self, path: &Path, result: &AnalysisResult) -> String {
        let mut output = format!("\n## {}\n", path.display());

        // Add analysis mode indicator if semantic analysis was performed
        if !result.calls.is_empty() || !result.references.is_empty() {
            output.push_str("*Analysis Mode: Semantic (with call graph)*\n\n");
        } else {
            output.push('\n');
        }

        if !result.functions.is_empty() {
            output.push_str("### Functions:\n");
            for func in &result.functions {
                output.push_str(&format!("- `{}` (line {})\n", func.name, func.line));

                // Add semantic information if available
                if !result.calls.is_empty() {
                    // Find calls made by this function
                    let calls_from: Vec<&CallInfo> = result
                        .calls
                        .iter()
                        .filter(|c| c.caller_name.as_ref() == Some(&func.name))
                        .collect();

                    if !calls_from.is_empty() {
                        // Group calls by callee name to avoid duplicates
                        let mut unique_callees: Vec<String> =
                            calls_from.iter().map(|c| c.callee_name.clone()).collect();
                        unique_callees.sort();
                        unique_callees.dedup();

                        output.push_str(&format!("  ↳ Calls: {}\n", unique_callees.join(", ")));
                    }

                    // Find who calls this function
                    let called_by: Vec<&CallInfo> = result
                        .calls
                        .iter()
                        .filter(|c| c.callee_name == func.name)
                        .collect();

                    if !called_by.is_empty() {
                        let callers: Vec<String> = called_by
                            .iter()
                            .filter_map(|c| c.caller_name.as_ref())
                            .cloned()
                            .collect::<Vec<_>>();

                        if !callers.is_empty() {
                            let mut unique_callers = callers;
                            unique_callers.sort();
                            unique_callers.dedup();
                            output.push_str(&format!(
                                "  ↳ Called by: {}\n",
                                unique_callers.join(", ")
                            ));
                        } else {
                            output.push_str("  ↳ Called from: module level\n");
                        }
                    }
                }
            }
            output.push('\n');
        }

        if !result.classes.is_empty() {
            output.push_str("### Classes/Types:\n");
            for class in &result.classes {
                output.push_str(&format!("- `{}` (line {})\n", class.name, class.line));
            }
            output.push('\n');
        }

        // Add call graph visualization for semantic analysis
        if !result.calls.is_empty() {
            output.push_str("### Call Graph:\n```\n");
            output.push_str(&self.generate_ascii_call_graph(result));
            output.push_str("```\n\n");
        }

        if !result.imports.is_empty() && result.imports.len() <= 10 {
            output.push_str("### Imports:\n");
            for import in &result.imports {
                output.push_str(&format!("- {}\n", import));
            }
            output.push('\n');
        }

        output
    }

    // Helper method to generate ASCII call graph
    fn generate_ascii_call_graph(&self, result: &AnalysisResult) -> String {
        let mut graph = String::new();

        // Group calls by caller
        let mut call_map: std::collections::HashMap<String, Vec<String>> =
            std::collections::HashMap::new();

        for call in &result.calls {
            let caller = call
                .caller_name
                .clone()
                .unwrap_or_else(|| "<module>".to_string());
            call_map
                .entry(caller)
                .or_default()
                .push(call.callee_name.clone());
        }

        // Sort and deduplicate callees for each caller
        for callees in call_map.values_mut() {
            callees.sort();
            callees.dedup();
        }

        // Generate the graph
        let mut callers: Vec<_> = call_map.keys().cloned().collect();
        callers.sort();

        for caller in callers {
            if let Some(callees) = call_map.get(&caller) {
                graph.push_str(&format!("{}\n", caller));
                for (i, callee) in callees.iter().enumerate() {
                    let prefix = if i == callees.len() - 1 {
                        "└─"
                    } else {
                        "├─"
                    };
                    graph.push_str(&format!("  {} {}\n", prefix, callee));
                }
            }
        }

        if graph.is_empty() {
            graph.push_str("(No function calls detected)\n");
        }

        graph
    }

    // Helper method to filter results by focus symbol
    fn filter_by_focus(&self, output: &str, focus: &str) -> String {
        let mut filtered = String::new();
        let mut include_section = false;

        for line in output.lines() {
            if line.starts_with("##") {
                include_section = false;
            }

            if line.contains(focus) {
                include_section = true;
                // Include the file header
                if let Some(header_line) = output
                    .lines()
                    .rev()
                    .find(|l| l.starts_with("##") && line.contains(&l[3..]))
                {
                    if !filtered.contains(header_line) {
                        filtered.push_str(header_line);
                        filtered.push('\n');
                    }
                }
            }

            if include_section || line.starts_with('#') {
                filtered.push_str(line);
                filtered.push('\n');
            }
        }

        if filtered.is_empty() {
            format!("No results found for symbol: {}", focus)
        } else {
            filtered
        }
    }
}
