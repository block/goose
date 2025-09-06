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

    /// Analysis mode: "auto" (default), "structure", "semantic", or "focused"
    /// - auto: Structure for directories, semantic for files
    /// - structure: File/function counts only
    /// - semantic: Detailed with call graphs
    /// - focused: Track symbol across all files (requires focus parameter)
    #[serde(default = "default_analysis_mode")]
    pub mode: String,

    /// Focus on specific symbol (used with focused mode)
    pub focus: Option<String>,

    /// How many call levels to trace in focused mode (default: 2)
    #[serde(default = "default_follow_depth")]
    pub follow_depth: u32,

    /// Maximum directory depth for traversal (0=unlimited)
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
}

fn default_analysis_mode() -> String {
    "auto".to_string()
}

fn default_follow_depth() -> u32 {
    2
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
    // Structure mode fields (for compact overview)
    function_count: usize,
    class_count: usize,
    line_count: usize,
    import_count: usize,
    main_line: Option<usize>,
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

// Entry type for directory results - cleaner than overloading AnalysisResult
#[derive(Debug, Clone)]
enum EntryType {
    File(AnalysisResult),
    Directory,
    SymlinkDir(PathBuf),
}

/// Code analyzer with caching and tree-sitter parsing
pub struct CodeAnalyzer {
    parser_cache: Arc<Mutex<HashMap<String, Arc<Mutex<Parser>>>>>,
    analysis_cache: Arc<Mutex<LruCache<(PathBuf, SystemTime), AnalysisResult>>>,
}

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self {
            parser_cache: Arc::new(Mutex::new(HashMap::new())),
            analysis_cache: Arc::new(Mutex::new(LruCache::new(
                std::num::NonZeroUsize::new(100).unwrap(),
            ))),
        }
    }
}

impl CodeAnalyzer {
    pub fn new() -> Self {
        Self::default()
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

        // Determine the actual mode to use
        let mode = self.determine_mode(&params, &path)?;

        let mut output = String::new();

        // Add warning for semantic mode on directories
        if path.is_dir() && mode == "semantic" {
            output.push_str(
                "⚠️ Warning: Semantic analysis on directories can produce large output.\n",
            );
            output.push_str("Consider using 'structure' mode or analyzing specific files.\n\n");
        }

        if path.is_file() {
            // Check if we're in focused mode
            if mode == "focused" {
                // Focused mode - requires focus parameter
                if params.focus.is_none() {
                    return Err(ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Focused mode requires 'focus' parameter to specify the symbol to track"
                            .to_string(),
                        None,
                    ));
                }
                let focus_symbol = params.focus.as_ref().unwrap();
                let focused_result = self
                    .analyze_focused(
                        &path,
                        focus_symbol,
                        params.follow_depth,
                        params.max_depth,
                        ignore_patterns,
                    )
                    .await?;
                output.push_str(&focused_result);
            } else {
                // Regular file analysis
                let result = self.analyze_file(&path, &mode).await?;
                output.push_str(&self.format_analysis_result(&path, &result));
            }
        } else {
            // Analyze directory
            if mode == "structure" {
                // For structure mode, collect all results and format with summary
                let results = self
                    .collect_directory_results(&path, 0, params.max_depth, ignore_patterns, &mode)
                    .await?;
                output.push_str(&self.format_directory_structure(
                    &path,
                    &results,
                    params.max_depth,
                ));
            } else if mode == "focused" {
                // Focused mode - requires focus parameter
                if params.focus.is_none() {
                    return Err(ErrorData::new(
                        ErrorCode::INVALID_PARAMS,
                        "Focused mode requires 'focus' parameter to specify the symbol to track"
                            .to_string(),
                        None,
                    ));
                }
                let focus_symbol = params.focus.as_ref().unwrap();
                let focused_result = self
                    .analyze_focused(
                        &path,
                        focus_symbol,
                        params.follow_depth,
                        params.max_depth,
                        ignore_patterns,
                    )
                    .await?;
                output.push_str(&focused_result);
            } else {
                // For semantic mode on directory, analyze each file with semantic analysis
                output.push_str(&format!("DIRECTORY: {}\n\n", path.display()));
                let results = self
                    .collect_directory_results(&path, 0, params.max_depth, ignore_patterns, &mode)
                    .await?;

                // Format each file's semantic analysis
                for (file_path, entry) in &results {
                    if let EntryType::File(result) = entry {
                        output.push_str(&self.format_analysis_result(file_path, result));
                        output.push_str("\n---\n\n");
                    }
                }

                // Add summary at the end
                let files: Vec<&AnalysisResult> = results
                    .iter()
                    .filter_map(|(_, entry)| match entry {
                        EntryType::File(result) => Some(result),
                        _ => None,
                    })
                    .collect();

                let total_files = files.len();
                let total_calls: usize = files.iter().map(|r| r.calls.len()).sum();
                output.push_str(&format!("\nDIRECTORY SUMMARY:\n"));
                output.push_str(&format!("Files analyzed: {}\n", total_files));
                output.push_str(&format!("Total function calls found: {}\n", total_calls));
            }
        }

        // If focus is specified with non-focused mode, filter results
        if let Some(focus) = &params.focus {
            if mode != "focused" {
                output = self.filter_by_focus(&output, focus);
            }
        }

        Ok(CallToolResult::success(vec![
            Content::text(output.clone()).with_audience(vec![Role::Assistant]),
            Content::text(output)
                .with_audience(vec![Role::User])
                .with_priority(0.0),
        ]))
    }

    // Helper method to determine the actual mode to use
    fn determine_mode(&self, params: &AnalyzeParams, path: &Path) -> Result<String, ErrorData> {
        let mode = &params.mode;

        // Handle auto mode
        if mode == "auto" {
            if path.is_file() {
                Ok("semantic".to_string())
            } else {
                Ok("structure".to_string())
            }
        } else {
            // Validate mode
            match mode.as_str() {
                "structure" | "semantic" | "focused" => Ok(mode.clone()),
                _ => Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!(
                        "Invalid mode '{}'. Must be one of: auto, structure, semantic, focused",
                        mode
                    ),
                    None,
                )),
            }
        }
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

        // Count lines
        let line_count = content.lines().count();

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
                function_count: 0,
                class_count: 0,
                line_count,
                import_count: 0,
                main_line: None,
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
                function_count: 0,
                class_count: 0,
                line_count,
                import_count: 0,
                main_line: None,
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
        let mut result = self.extract_code_elements_with_depth(&tree, &content, language, depth)?;

        // Add line count to the result
        result.line_count = line_count;

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

        // For structure mode, clear the detailed vectors but keep the counts
        if depth == "structure" {
            // The counts are already set in extract_code_elements
            // Clear the detailed data to save memory/tokens
            result.functions.clear();
            result.classes.clear();
            result.imports.clear();
        } else if depth == "semantic" {
            // For semantic mode, also extract calls
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
                    function_count: 0,
                    class_count: 0,
                    line_count: 0,
                    import_count: 0,
                    main_line: None,
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

        // Detect main function
        let main_line = functions.iter().find(|f| f.name == "main").map(|f| f.line);

        Ok(AnalysisResult {
            functions: functions.clone(),
            classes: classes.clone(),
            imports: imports.clone(),
            calls: vec![],
            references: vec![],
            function_count: functions.len(),
            class_count: classes.len(),
            line_count: 0, // Will be set later
            import_count: imports.len(),
            main_line,
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
        // max_depth of 0 means unlimited depth
        if max_depth > 0 && depth >= max_depth {
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

    // Helper method to format structure overview (new compact format)
    fn format_structure_overview(&self, path: &Path, result: &AnalysisResult) -> String {
        let mut output = String::new();

        // Format as: path [LOC, FUNCTIONS, CLASSES] <FLAGS>
        output.push_str(&format!("{} [{}L", path.display(), result.line_count));

        if result.function_count > 0 {
            output.push_str(&format!(", {}F", result.function_count));
        }

        if result.class_count > 0 {
            output.push_str(&format!(", {}C", result.class_count));
        }

        output.push(']');

        // Add FLAGS if any
        if let Some(main_line) = result.main_line {
            output.push_str(&format!(" main:{}", main_line));
        }

        output.push('\n');
        output
    }

    // Helper method to format analysis results (optimized for LLMs)
    fn format_analysis_result(&self, path: &Path, result: &AnalysisResult) -> String {
        // Check if this is structure mode (no detailed data)
        if result.functions.is_empty()
            && result.classes.is_empty()
            && result.imports.is_empty()
            && result.calls.is_empty()
            && result.references.is_empty()
            && (result.function_count > 0 || result.class_count > 0 || result.line_count > 0)
        {
            // Structure mode - use compact format
            return self.format_structure_overview(path, result);
        }

        // Semantic mode - optimized format for LLMs
        let mut output = format!("FILE: {}\n", path.display());
        output.push_str(&format!(
            "METRICS: {}L, {}F, {}C\n\n",
            result.line_count, result.function_count, result.class_count
        ));

        // List functions with line numbers
        if !result.functions.is_empty() {
            output.push_str("FUNCTIONS:\n");
            for func in &result.functions {
                output.push_str(&format!("{} (line {})\n", func.name, func.line));
            }
            output.push('\n');
        }

        // List classes/types
        if !result.classes.is_empty() {
            output.push_str("CLASSES:\n");
            for class in &result.classes {
                output.push_str(&format!("{} (line {})\n", class.name, class.line));
            }
            output.push('\n');
        }

        // Add call relationships for semantic analysis
        if !result.calls.is_empty() {
            output.push_str(&self.format_call_relationships(result));
            output.push('\n');
        }

        // Only show imports if there are few of them
        if !result.imports.is_empty() && result.imports.len() <= 5 {
            output.push_str("IMPORTS:\n");
            for import in &result.imports.iter().take(5).collect::<Vec<_>>() {
                // Simplify import display
                let simplified = if import.len() > 50 {
                    format!("{}...", &import[..47])
                } else {
                    import.to_string()
                };
                output.push_str(&format!("{}\n", simplified));
            }
            if result.imports.len() > 5 {
                output.push_str(&format!("... and {} more\n", result.imports.len() - 5));
            }
            output.push('\n');
        }

        output
    }

    // Helper method to format call relationships as flat list (optimized for LLMs)
    fn format_call_relationships(&self, result: &AnalysisResult) -> String {
        let mut output = String::new();

        if result.calls.is_empty() {
            output.push_str("No function calls detected\n");
            return output;
        }

        // Create a deduplicated list of relationships with line numbers
        let mut relationships: Vec<String> = Vec::new();
        let mut seen = std::collections::HashSet::new();

        for call in &result.calls {
            let caller = call
                .caller_name
                .clone()
                .unwrap_or_else(|| "<module>".to_string());
            let relationship = format!("{} (line {}) -> {}", caller, call.line, call.callee_name);

            // Only add unique relationships
            let key = format!("{}:{}", caller, call.callee_name);
            if seen.insert(key) {
                relationships.push(relationship);
            }
        }

        // Sort for consistent output
        relationships.sort();

        // Format as simple list
        output.push_str("CALL RELATIONSHIPS:\n");
        for rel in relationships {
            output.push_str(&format!("{}\n", rel));
        }

        output
    }

    // Helper method to collect all results from a directory
    async fn collect_directory_results(
        &self,
        path: &Path,
        depth: u32,
        max_depth: u32,
        ignore_patterns: &Gitignore,
        analysis_depth: &str,
    ) -> Result<Vec<(PathBuf, EntryType)>, ErrorData> {
        let mut results = Vec::new();

        // max_depth of 0 means unlimited depth
        if max_depth > 0 && depth >= max_depth {
            return Ok(results);
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

            // Get metadata without following symlinks
            let metadata = entry.metadata().map_err(|e| {
                ErrorData::new(
                    ErrorCode::INTERNAL_ERROR,
                    format!("Failed to get metadata: {}", e),
                    None,
                )
            })?;

            if metadata.is_symlink() {
                // Check if symlink points to a directory
                if let Ok(target_meta) = std::fs::metadata(&entry_path) {
                    if target_meta.is_dir() {
                        // Get the symlink target
                        if let Ok(target) = std::fs::read_link(&entry_path) {
                            results.push((entry_path, EntryType::SymlinkDir(target)));
                        }
                    }
                }
                // Skip if symlink points to file or is broken
            } else if metadata.is_dir() {
                if max_depth > 0 && depth + 1 >= max_depth {
                    // At max depth, just mark as directory
                    results.push((entry_path, EntryType::Directory));
                } else {
                    // Recurse into subdirectory
                    let mut sub_results = Box::pin(self.collect_directory_results(
                        &entry_path,
                        depth + 1,
                        max_depth,
                        ignore_patterns,
                        analysis_depth,
                    ))
                    .await?;
                    results.append(&mut sub_results);
                }
            } else if metadata.is_file() {
                // Only analyze supported file types
                let lang = lang::get_language_identifier(&entry_path);
                if !lang.is_empty() {
                    let result = self.analyze_file(&entry_path, analysis_depth).await?;
                    if result.function_count > 0 || result.class_count > 0 || result.line_count > 0
                    {
                        results.push((entry_path, EntryType::File(result)));
                    }
                }
            }
        }

        Ok(results)
    }

    // Helper method to format directory structure with summary
    fn format_directory_structure(
        &self,
        base_path: &Path,
        results: &[(PathBuf, EntryType)],
        max_depth: u32,
    ) -> String {
        let mut output = String::new();

        // Calculate totals (only from files)
        let files: Vec<&AnalysisResult> = results
            .iter()
            .filter_map(|(_, entry)| match entry {
                EntryType::File(result) => Some(result),
                _ => None,
            })
            .collect();

        let total_files = files.len();
        let total_lines: usize = files.iter().map(|r| r.line_count).sum();
        let total_functions: usize = files.iter().map(|r| r.function_count).sum();
        let total_classes: usize = files.iter().map(|r| r.class_count).sum();

        // Calculate language distribution
        let mut language_lines: HashMap<String, usize> = HashMap::new();
        for (path, entry) in results {
            if let EntryType::File(result) = entry {
                let lang = lang::get_language_identifier(path);
                if !lang.is_empty() && result.line_count > 0 {
                    *language_lines.entry(lang.to_string()).or_insert(0) += result.line_count;
                }
            }
        }

        // Format summary with depth indicator
        output.push_str("SUMMARY:\n");
        if max_depth == 0 {
            output.push_str(&format!(
                "Shown: {} files, {}L, {}F, {}C (unlimited depth)\n",
                total_files, total_lines, total_functions, total_classes
            ));
        } else {
            output.push_str(&format!(
                "Shown: {} files, {}L, {}F, {}C (max_depth={})\n",
                total_files, total_lines, total_functions, total_classes, max_depth
            ));
        }

        // Format language percentages
        if !language_lines.is_empty() && total_lines > 0 {
            let mut languages: Vec<_> = language_lines.iter().collect();
            languages.sort_by(|a, b| b.1.cmp(a.1)); // Sort by lines descending

            let lang_str: Vec<String> = languages
                .iter()
                .map(|(lang, lines)| {
                    let percentage = (**lines as f64 / total_lines as f64 * 100.0) as u32;
                    format!("{} ({}%)", lang, percentage)
                })
                .collect();

            output.push_str(&format!("Languages: {}\n", lang_str.join(", ")));
        }

        output.push_str("\nPATH [LOC, FUNCTIONS, CLASSES] <FLAGS>\n");

        // Sort results by path for consistent output
        let mut sorted_results = results.to_vec();
        sorted_results.sort_by(|a, b| a.0.cmp(&b.0));

        // Format each entry
        for (path, entry) in sorted_results {
            // Make path relative to base_path
            let relative_path = path.strip_prefix(base_path).unwrap_or(&path);

            match entry {
                EntryType::File(result) => {
                    output.push_str(&self.format_structure_overview(relative_path, &result));
                }
                EntryType::Directory => {
                    output.push_str(&format!("{}/\n", relative_path.display()));
                }
                EntryType::SymlinkDir(target) => {
                    // Make target relative if possible for cleaner display
                    let target_display = if target.is_relative() {
                        target.display().to_string()
                    } else if let Ok(rel) = target.strip_prefix(base_path) {
                        rel.display().to_string()
                    } else {
                        target.display().to_string()
                    };
                    output.push_str(&format!(
                        "{}/ → {}\n",
                        relative_path.display(),
                        target_display
                    ));
                }
            }
        }

        output
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

    // Focused mode implementation - track a symbol across all files
    async fn analyze_focused(
        &self,
        path: &Path,
        focus_symbol: &str,
        _follow_depth: u32,
        max_depth: u32,
        ignore_patterns: &Gitignore,
    ) -> Result<String, ErrorData> {
        let mut output = String::new();

        output.push_str(&format!("FOCUSED ANALYSIS: {}\n\n", focus_symbol));

        // Collect all files to analyze
        let files_to_analyze = if path.is_file() {
            // Single file mode
            vec![path.to_path_buf()]
        } else {
            // Directory mode - collect all supported files
            self.collect_files_for_focused(path, 0, max_depth, ignore_patterns)
                .await?
        };

        // Track definitions and calls across all files
        let mut definitions: Vec<(PathBuf, usize, String)> = Vec::new(); // (file, line, context)
        let mut call_paths: Vec<(PathBuf, String, usize, PathBuf, String, usize)> = Vec::new(); // (caller_file, caller_func, caller_line, callee_file, callee_func, callee_line)
        let mut references: Vec<(PathBuf, usize, String)> = Vec::new(); // (file, line, context)

        // Analyze each file for the focus symbol
        for file_path in &files_to_analyze {
            let result = self.analyze_file(file_path, "semantic").await?;

            // Find definitions
            for func in &result.functions {
                if func.name == focus_symbol {
                    definitions.push((file_path.clone(), func.line, format!("{}()", func.name)));
                }
            }

            for class in &result.classes {
                if class.name == focus_symbol {
                    definitions.push((
                        file_path.clone(),
                        class.line,
                        format!("class {}", class.name),
                    ));
                }
            }

            // Find calls and references
            for call in &result.calls {
                if call.callee_name == focus_symbol {
                    let caller = call
                        .caller_name
                        .clone()
                        .unwrap_or_else(|| "<module>".to_string());
                    references.push((file_path.clone(), call.line, call.context.clone()));

                    // Try to find where the callee is defined
                    for (def_file, def_line, _) in &definitions {
                        call_paths.push((
                            file_path.clone(),
                            caller.clone(),
                            call.line,
                            def_file.clone(),
                            focus_symbol.to_string(),
                            *def_line,
                        ));
                    }
                }

                // Also check if the caller is our focus symbol
                if let Some(ref caller_name) = call.caller_name {
                    if caller_name == focus_symbol {
                        // This function calls something else
                        // We could track this for follow_depth > 1
                    }
                }
            }
        }

        // Format output in path notation
        if !definitions.is_empty() {
            output.push_str("DEFINITIONS:\n");
            for (file, line, context) in &definitions {
                let relative_path = if path.is_dir() {
                    file.strip_prefix(path).unwrap_or(file)
                } else {
                    file.as_path()
                };
                output.push_str(&format!(
                    "{}:{} - {}\n",
                    relative_path.display(),
                    line,
                    context
                ));
            }
            output.push_str("\n");
        }

        if !call_paths.is_empty() {
            output.push_str("CALL PATHS:\n");
            // Deduplicate and sort call paths
            let mut unique_paths = std::collections::HashSet::new();
            for (caller_file, caller_func, caller_line, _, callee_func, _) in &call_paths {
                let relative_path = if path.is_dir() {
                    caller_file.strip_prefix(path).unwrap_or(caller_file)
                } else {
                    caller_file.as_path()
                };
                let path_str = format!(
                    "{}:{} ({}) -> {}",
                    relative_path.display(),
                    caller_line,
                    caller_func,
                    callee_func
                );
                unique_paths.insert(path_str);
            }

            let mut sorted_paths: Vec<_> = unique_paths.into_iter().collect();
            sorted_paths.sort();

            for path_str in sorted_paths {
                output.push_str(&format!("{}\n", path_str));
            }
            output.push_str("\n");
        }

        // Summary
        output.push_str(&format!(
            "REFERENCES: {} files, {} locations\n",
            files_to_analyze.len(),
            references.len()
        ));

        if definitions.is_empty() && references.is_empty() {
            output = format!(
                "Symbol '{}' not found in any analyzed files.\n",
                focus_symbol
            );
        }

        Ok(output)
    }

    // Helper to collect all files for focused analysis
    async fn collect_files_for_focused(
        &self,
        path: &Path,
        depth: u32,
        max_depth: u32,
        ignore_patterns: &Gitignore,
    ) -> Result<Vec<PathBuf>, ErrorData> {
        let mut files = Vec::new();

        // max_depth of 0 means unlimited depth
        if max_depth > 0 && depth >= max_depth {
            return Ok(files);
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
                // Only include supported file types
                let lang = lang::get_language_identifier(&entry_path);
                if !lang.is_empty() {
                    files.push(entry_path);
                }
            } else if entry_path.is_dir() {
                // Recurse into subdirectory
                let mut sub_files = Box::pin(self.collect_files_for_focused(
                    &entry_path,
                    depth + 1,
                    max_depth,
                    ignore_patterns,
                ))
                .await?;
                files.append(&mut sub_files);
            }
        }

        Ok(files)
    }
}
