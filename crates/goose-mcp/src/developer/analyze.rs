use ignore::gitignore::Gitignore;
use lru::LruCache;
use rmcp::{
    model::{CallToolResult, Content, ErrorCode, ErrorData, Role},
    schemars::JsonSchema,
};
use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet, VecDeque},
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
    /// Absolute path. Step 1: Directory for overview. Step 2: File for details. Step 3: Directory with focus param for call graphs
    pub path: String,

    /// Symbol name for call graph analysis (Step 3). Requires directory path. Shows who calls it and what it calls
    pub focus: Option<String>,

    /// Call graph depth. 0=where defined, 1=direct callers/callees, 2+=transitive chains
    #[serde(default = "default_follow_depth")]
    pub follow_depth: u32,

    /// Directory recursion limit. 0=unlimited (warning: fails on binary files)
    #[serde(default = "default_max_depth")]
    pub max_depth: u32,
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
    SymlinkFile(PathBuf),
}

// Type alias for complex query results
type ElementQueryResult = (Vec<FunctionInfo>, Vec<ClassInfo>, Vec<String>);

// Minimal graph structure for focus mode only
#[derive(Debug, Clone)]
struct CallGraph {
    // Map from symbol name to its callers: Vec<(file, line, caller_function)>
    callers: HashMap<String, Vec<(PathBuf, usize, String)>>,
    // Map from symbol name to what it calls: Vec<(file, line, callee_function)>
    callees: HashMap<String, Vec<(PathBuf, usize, String)>>,
    // Map from symbol to its definition locations
    definitions: HashMap<String, Vec<(PathBuf, usize)>>,
}

impl CallGraph {
    fn new() -> Self {
        Self {
            callers: HashMap::new(),
            callees: HashMap::new(),
            definitions: HashMap::new(),
        }
    }

    fn build_from_results(results: &[(PathBuf, AnalysisResult)]) -> Self {
        let mut graph = Self::new();

        for (file_path, result) in results {
            // Record definitions
            for func in &result.functions {
                graph
                    .definitions
                    .entry(func.name.clone())
                    .or_default()
                    .push((file_path.clone(), func.line));
            }

            for class in &result.classes {
                graph
                    .definitions
                    .entry(class.name.clone())
                    .or_default()
                    .push((file_path.clone(), class.line));
            }

            // Record call relationships
            for call in &result.calls {
                let caller = call
                    .caller_name
                    .clone()
                    .unwrap_or_else(|| "<module>".to_string());

                // Add to callers map (who calls this function)
                graph
                    .callers
                    .entry(call.callee_name.clone())
                    .or_default()
                    .push((file_path.clone(), call.line, caller.clone()));

                // Add to callees map (what this function calls)
                if caller != "<module>" {
                    graph.callees.entry(caller).or_default().push((
                        file_path.clone(),
                        call.line,
                        call.callee_name.clone(),
                    ));
                }
            }
        }

        graph
    }
}

#[derive(Debug, Clone)]
struct CallChain {
    path: Vec<(PathBuf, usize, String, String)>, // (file, line, from, to)
}

// Data structure to pass to format_focused_output_with_chains
struct FocusedAnalysisData<'a> {
    focus_symbol: &'a str,
    follow_depth: u32,
    files_analyzed: &'a [PathBuf],
    definitions: &'a [(PathBuf, usize)],
    incoming_chains: &'a [CallChain],
    outgoing_chains: &'a [CallChain],
}

impl CallGraph {
    fn find_incoming_chains(&self, symbol: &str, max_depth: u32) -> Vec<CallChain> {
        if max_depth == 0 {
            return vec![];
        }

        let mut chains = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with direct callers
        if let Some(direct_callers) = self.callers.get(symbol) {
            for (file, line, caller) in direct_callers {
                let initial_path = vec![(file.clone(), *line, caller.clone(), symbol.to_string())];

                if max_depth == 1 {
                    chains.push(CallChain { path: initial_path });
                } else {
                    queue.push_back((caller.clone(), initial_path, 1));
                }
            }
        }

        // BFS to find deeper chains
        while let Some((current_symbol, path, depth)) = queue.pop_front() {
            if depth >= max_depth {
                chains.push(CallChain { path });
                continue;
            }

            // Avoid cycles
            if visited.contains(&current_symbol) {
                chains.push(CallChain { path }); // Still record the path we found
                continue;
            }
            visited.insert(current_symbol.clone());

            // Find who calls the current symbol
            if let Some(callers) = self.callers.get(&current_symbol) {
                for (file, line, caller) in callers {
                    let mut new_path =
                        vec![(file.clone(), *line, caller.clone(), current_symbol.clone())];
                    new_path.extend(path.clone());

                    if depth + 1 >= max_depth {
                        chains.push(CallChain { path: new_path });
                    } else {
                        queue.push_back((caller.clone(), new_path, depth + 1));
                    }
                }
            } else {
                // No more callers, this is a chain end
                chains.push(CallChain { path });
            }
        }

        chains
    }

    fn find_outgoing_chains(&self, symbol: &str, max_depth: u32) -> Vec<CallChain> {
        if max_depth == 0 {
            return vec![];
        }

        let mut chains = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        // Start with what this symbol calls
        if let Some(direct_callees) = self.callees.get(symbol) {
            for (file, line, callee) in direct_callees {
                let initial_path = vec![(file.clone(), *line, symbol.to_string(), callee.clone())];

                if max_depth == 1 {
                    chains.push(CallChain { path: initial_path });
                } else {
                    queue.push_back((callee.clone(), initial_path, 1));
                }
            }
        }

        // BFS to find deeper chains
        while let Some((current_symbol, path, depth)) = queue.pop_front() {
            if depth >= max_depth {
                chains.push(CallChain { path });
                continue;
            }

            // Avoid cycles
            if visited.contains(&current_symbol) {
                chains.push(CallChain { path });
                continue;
            }
            visited.insert(current_symbol.clone());

            // Find what the current symbol calls
            if let Some(callees) = self.callees.get(&current_symbol) {
                for (file, line, callee) in callees {
                    let mut new_path = path.clone();
                    new_path.push((file.clone(), *line, current_symbol.clone(), callee.clone()));

                    if depth + 1 >= max_depth {
                        chains.push(CallChain { path: new_path });
                    } else {
                        queue.push_back((callee.clone(), new_path, depth + 1));
                    }
                }
            } else {
                // No more callees, this is a chain end
                chains.push(CallChain { path });
            }
        }

        chains
    }
}

/// Code analyzer with caching and tree-sitter parsing
#[derive(Clone)]
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
        // Validate path
        self.validate_path(&path, ignore_patterns)?;

        // Determine the actual mode to use
        let mode = self.determine_mode(&params, &path)?;

        // Process based on path type and mode
        let mut output = if path.is_file() {
            self.analyze_file_with_mode(&path, &mode, &params, ignore_patterns)
                .await?
        } else {
            self.analyze_directory_with_mode(&path, &mode, &params, ignore_patterns)
                .await?
        };

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

    // Helper to validate path
    fn validate_path(&self, path: &Path, ignore_patterns: &Gitignore) -> Result<(), ErrorData> {
        // Check if path is ignored
        if self.is_ignored(path, ignore_patterns) {
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

        Ok(())
    }

    // Helper to analyze file based on mode
    async fn analyze_file_with_mode(
        &self,
        path: &Path,
        mode: &str,
        params: &AnalyzeParams,
        ignore_patterns: &Gitignore,
    ) -> Result<String, ErrorData> {
        if mode == "focused" {
            self.validate_and_run_focused(path, params, ignore_patterns)
                .await
        } else {
            let result = self.analyze_file(path, mode).await?;
            Ok(self.format_analysis_result(path, &result, mode))
        }
    }

    // Helper to analyze directory based on mode
    async fn analyze_directory_with_mode(
        &self,
        path: &Path,
        mode: &str,
        params: &AnalyzeParams,
        ignore_patterns: &Gitignore,
    ) -> Result<String, ErrorData> {
        match mode {
            "focused" => {
                self.validate_and_run_focused(path, params, ignore_patterns)
                    .await
            }
            _ => {
                // Structure mode for directories
                let results = self
                    .collect_directory_results(
                        path,
                        0,
                        params.max_depth,
                        ignore_patterns,
                        "structure",
                    )
                    .await?;
                Ok(self.format_directory_structure(path, &results, params.max_depth))
            }
        }
    }

    // Helper to validate focused mode and run it
    async fn validate_and_run_focused(
        &self,
        path: &Path,
        params: &AnalyzeParams,
        ignore_patterns: &Gitignore,
    ) -> Result<String, ErrorData> {
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
        self.analyze_focused(
            path,
            focus_symbol,
            params.follow_depth,
            params.max_depth,
            ignore_patterns,
        )
        .await
    }

    // Helper method to determine the actual mode to use
    fn determine_mode(&self, params: &AnalyzeParams, path: &Path) -> Result<String, ErrorData> {
        // If focus is specified, use focused mode
        if params.focus.is_some() {
            return Ok("focused".to_string());
        }

        // Otherwise, use semantic for files, structure for directories
        if path.is_file() {
            Ok("semantic".to_string())
        } else {
            Ok("structure".to_string())
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
        // Get language-specific query
        let query_str = self.get_element_query(language);
        if query_str.is_empty() {
            return Ok(self.empty_analysis_result());
        }

        // Parse and process the query
        let (functions, classes, imports) = self.process_element_query(tree, source, query_str)?;

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

    // Get language-specific query for elements
    fn get_element_query(&self, language: &str) -> &'static str {
        match language {
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
            _ => "",
        }
    }

    // Process element query and extract functions, classes, imports
    fn process_element_query(
        &self,
        tree: &Tree,
        source: &str,
        query_str: &str,
    ) -> Result<ElementQueryResult, ErrorData> {
        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

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

        Ok((functions, classes, imports))
    }

    // Create empty analysis result
    fn empty_analysis_result(&self) -> AnalysisResult {
        AnalysisResult {
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
        }
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

    // Helper method to format analysis results (dense matrix format)
    fn format_analysis_result(&self, path: &Path, result: &AnalysisResult, mode: &str) -> String {
        // Check the actual mode parameter instead of inferring from data
        if mode == "structure" {
            // Structure mode - use compact format
            return self.format_structure_overview(path, result);
        }

        // Dense matrix format for semantic mode
        let mut output = format!(
            "FILE: {} [{}L, {}F, {}C]\n\n",
            path.display(),
            result.line_count,
            result.function_count,
            result.class_count
        );

        // Classes on single/multiple lines with colon-separated line numbers
        if !result.classes.is_empty() {
            output.push_str("C: ");
            let class_strs: Vec<String> = result
                .classes
                .iter()
                .map(|c| format!("{}:{}", c.name, c.line))
                .collect();
            output.push_str(&class_strs.join(" "));
            output.push_str("\n\n");
        }

        // Functions with call counts where significant
        if !result.functions.is_empty() {
            output.push_str("F: ");

            // Count how many times each function is called
            let mut call_counts: HashMap<String, usize> = HashMap::new();
            for call in &result.calls {
                *call_counts.entry(call.callee_name.clone()).or_insert(0) += 1;
            }

            let func_strs: Vec<String> = result
                .functions
                .iter()
                .map(|f| {
                    let count = call_counts.get(&f.name).unwrap_or(&0);
                    if *count > 3 {
                        format!("{}:{}•{}", f.name, f.line, count)
                    } else {
                        format!("{}:{}", f.name, f.line)
                    }
                })
                .collect();

            // Format functions, wrapping at reasonable line length
            let mut line_len = 3; // "F: "
            for (i, func_str) in func_strs.iter().enumerate() {
                if i > 0 && line_len + func_str.len() + 1 > 100 {
                    output.push_str("\n   ");
                    line_len = 3;
                }
                if i > 0 {
                    output.push(' ');
                    line_len += 1;
                }
                output.push_str(func_str);
                line_len += func_str.len();
            }
            output.push_str("\n\n");
        }

        // Condensed imports
        if !result.imports.is_empty() {
            output.push_str("I: ");

            // Group imports by module/package
            let mut grouped_imports: HashMap<String, Vec<String>> = HashMap::new();
            for import in &result.imports {
                // Simple heuristic: first word/module is the group
                let group = if import.starts_with("use ") {
                    import.split("::").next().unwrap_or("use").to_string()
                } else if import.starts_with("import ") {
                    import
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("import")
                        .to_string()
                } else if import.starts_with("from ") {
                    import
                        .split_whitespace()
                        .nth(1)
                        .unwrap_or("from")
                        .to_string()
                } else {
                    import.split_whitespace().next().unwrap_or("").to_string()
                };
                grouped_imports
                    .entry(group)
                    .or_default()
                    .push(import.clone());
            }

            // Show condensed import summary
            let import_summary: Vec<String> = grouped_imports
                .iter()
                .map(|(group, imports)| {
                    if imports.len() > 1 {
                        format!("{}({})", group, imports.len())
                    } else {
                        // For single imports, show more detail
                        let imp = &imports[0];
                        if imp.len() > 40 {
                            format!("{}...", &imp[..37])
                        } else {
                            imp.clone()
                        }
                    }
                })
                .collect();

            output.push_str(&import_summary.join("; "));
            output.push('\n');
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
                // Get the symlink target
                if let Ok(target) = std::fs::read_link(&entry_path) {
                    // Check what the symlink points to (if it exists)
                    match std::fs::metadata(&entry_path) {
                        Ok(target_meta) => {
                            if target_meta.is_dir() {
                                results.push((entry_path, EntryType::SymlinkDir(target)));
                            } else if target_meta.is_file() {
                                // Handle file symlinks
                                results.push((entry_path, EntryType::SymlinkFile(target)));
                            }
                        }
                        Err(_) => {
                            // Broken symlink - skip as per current behavior
                        }
                    }
                }
                // Skip further processing of symlinks
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
    #[allow(clippy::too_many_lines)]
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

        // Track which directories we've already printed to avoid duplicates
        let mut printed_dirs = HashSet::new();

        // Format each entry with tree-style indentation
        for (path, entry) in sorted_results {
            // Make path relative to base_path
            let relative_path = path.strip_prefix(base_path).unwrap_or(&path);

            // Get path components for determining structure
            let components: Vec<_> = relative_path.components().collect();
            if components.is_empty() {
                continue;
            }

            // Print parent directories if not already printed
            for i in 0..components.len().saturating_sub(1) {
                let parent_path: PathBuf = components[..=i].iter().collect();
                if !printed_dirs.contains(&parent_path) {
                    let indent = "  ".repeat(i);
                    let dir_name = components[i].as_os_str().to_string_lossy();
                    output.push_str(&format!("{}{}/\n", indent, dir_name));
                    printed_dirs.insert(parent_path);
                }
            }

            // Determine indentation level for this entry
            let indent_level = components.len().saturating_sub(1);
            let indent = "  ".repeat(indent_level);

            // Get the file/directory name (last component)
            let name = components
                .last()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .unwrap_or_else(|| relative_path.display().to_string());

            match entry {
                EntryType::File(result) => {
                    output.push_str(&format!("{}{} [{}L", indent, name, result.line_count));
                    if result.function_count > 0 {
                        output.push_str(&format!(", {}F", result.function_count));
                    }
                    if result.class_count > 0 {
                        output.push_str(&format!(", {}C", result.class_count));
                    }
                    output.push(']');
                    if let Some(main_line) = result.main_line {
                        output.push_str(&format!(" main:{}", main_line));
                    }
                    output.push('\n');
                }
                EntryType::Directory => {
                    // Only print if not already printed as a parent
                    if !printed_dirs.contains(relative_path) {
                        output.push_str(&format!("{}{}/\n", indent, name));
                        printed_dirs.insert(relative_path.to_path_buf());
                    }
                }
                EntryType::SymlinkDir(target) => {
                    let target_display = if target.is_relative() {
                        target.display().to_string()
                    } else if let Ok(rel) = target.strip_prefix(base_path) {
                        rel.display().to_string()
                    } else {
                        target.display().to_string()
                    };
                    output.push_str(&format!("{}{}/ -> {}\n", indent, name, target_display));
                }
                EntryType::SymlinkFile(target) => {
                    let target_display = if target.is_relative() {
                        target.display().to_string()
                    } else if let Ok(rel) = target.strip_prefix(base_path) {
                        rel.display().to_string()
                    } else {
                        target.display().to_string()
                    };
                    output.push_str(&format!("{}{} -> {}\n", indent, name, target_display));
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
        follow_depth: u32, // NOW WE ACTUALLY USE THIS!
        max_depth: u32,
        ignore_patterns: &Gitignore,
    ) -> Result<String, ErrorData> {
        // Step 1: Collect all files to analyze (UNCHANGED)
        let files_to_analyze = if path.is_file() {
            vec![path.to_path_buf()]
        } else {
            self.collect_files_for_focused(path, 0, max_depth, ignore_patterns)
                .await?
        };

        // Step 2: Analyze all files and collect results (REUSE EXISTING)
        let mut all_results = Vec::new();
        for file_path in &files_to_analyze {
            let result = self.analyze_file(file_path, "semantic").await?;
            all_results.push((file_path.clone(), result));
        }

        // Step 3: Build the call graph (NEW)
        let graph = CallGraph::build_from_results(&all_results);

        // Step 4: Find call chains based on follow_depth (NEW)
        let incoming_chains = if follow_depth > 0 {
            graph.find_incoming_chains(focus_symbol, follow_depth)
        } else {
            vec![]
        };

        let outgoing_chains = if follow_depth > 0 {
            graph.find_outgoing_chains(focus_symbol, follow_depth)
        } else {
            vec![]
        };

        // Step 5: Get definitions from graph (SIMPLIFIED)
        let definitions = graph
            .definitions
            .get(focus_symbol)
            .cloned()
            .unwrap_or_default();

        // Step 6: Format the enhanced output (MODIFIED)
        let focus_data = FocusedAnalysisData {
            focus_symbol,
            follow_depth,
            files_analyzed: &files_to_analyze,
            definitions: &definitions,
            incoming_chains: &incoming_chains,
            outgoing_chains: &outgoing_chains,
        };
        self.format_focused_output_with_chains(&focus_data)
    }

    // Format focused analysis output with enhanced call chains
    fn format_focused_output_with_chains(
        &self,
        focus_data: &FocusedAnalysisData,
    ) -> Result<String, ErrorData> {
        let focus_symbol = focus_data.focus_symbol;
        let follow_depth = focus_data.follow_depth;
        let files_analyzed = focus_data.files_analyzed;
        let definitions = focus_data.definitions;
        let incoming_chains = focus_data.incoming_chains;
        let outgoing_chains = focus_data.outgoing_chains;
        let mut output = format!("FOCUSED ANALYSIS: {}\n\n", focus_symbol);

        // Build file alias mapping
        let (file_map, sorted_files) =
            self.build_file_aliases(definitions, incoming_chains, outgoing_chains);

        // Section 1: Definitions
        self.append_definitions(&mut output, definitions, &file_map, focus_symbol);

        // Section 2: Incoming Call Chains
        self.append_call_chains(&mut output, incoming_chains, &file_map, follow_depth, true);

        // Section 3: Outgoing Call Chains
        self.append_call_chains(&mut output, outgoing_chains, &file_map, follow_depth, false);

        // Section 4: Summary Statistics
        self.append_statistics(
            &mut output,
            files_analyzed,
            definitions,
            incoming_chains,
            outgoing_chains,
            follow_depth,
        );

        // Section 5: File Legend
        self.append_file_legend(
            &mut output,
            &file_map,
            &sorted_files,
            definitions,
            incoming_chains,
            outgoing_chains,
        );

        if definitions.is_empty() && incoming_chains.is_empty() && outgoing_chains.is_empty() {
            output = format!(
                "Symbol '{}' not found in any analyzed files.\n",
                focus_symbol
            );
        }

        Ok(output)
    }

    // Helper: Build file alias mapping
    fn build_file_aliases(
        &self,
        definitions: &[(PathBuf, usize)],
        incoming_chains: &[CallChain],
        outgoing_chains: &[CallChain],
    ) -> (HashMap<PathBuf, String>, Vec<PathBuf>) {
        let mut all_files = HashSet::new();

        for (file, _) in definitions {
            all_files.insert(file.clone());
        }

        for chain in incoming_chains.iter().chain(outgoing_chains.iter()) {
            for (file, _, _, _) in &chain.path {
                all_files.insert(file.clone());
            }
        }

        let mut sorted_files: Vec<_> = all_files.into_iter().collect();
        sorted_files.sort();

        let mut file_map = HashMap::new();
        for (index, file) in sorted_files.iter().enumerate() {
            let alias = if sorted_files.len() == 1 {
                file.file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unknown")
                    .to_string()
            } else {
                format!("F{}", index + 1)
            };
            file_map.insert(file.clone(), alias);
        }

        (file_map, sorted_files)
    }

    // Helper: Append definitions section
    fn append_definitions(
        &self,
        output: &mut String,
        definitions: &[(PathBuf, usize)],
        file_map: &HashMap<PathBuf, String>,
        focus_symbol: &str,
    ) {
        if !definitions.is_empty() {
            output.push_str("DEFINITIONS:\n");
            for (file, line) in definitions {
                let alias = file_map.get(file).cloned().unwrap_or_else(|| {
                    file.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                });
                output.push_str(&format!("{}:{} - {}\n", alias, line, focus_symbol));
            }
            output.push('\n');
        }
    }

    // Helper: Append call chains section
    fn append_call_chains(
        &self,
        output: &mut String,
        chains: &[CallChain],
        file_map: &HashMap<PathBuf, String>,
        follow_depth: u32,
        is_incoming: bool,
    ) {
        if !chains.is_empty() {
            let chain_type = if is_incoming { "INCOMING" } else { "OUTGOING" };
            output.push_str(&format!(
                "{} CALL CHAINS (depth={}):\n",
                chain_type, follow_depth
            ));

            let mut unique_chains = HashSet::new();
            for chain in chains {
                let chain_str = self.format_chain_path(&chain.path, file_map);
                unique_chains.insert(chain_str);
            }

            let mut sorted_chains: Vec<_> = unique_chains.into_iter().collect();
            sorted_chains.sort();

            for chain in sorted_chains {
                output.push_str(&format!("{}\n", chain));
            }
            output.push('\n');
        }
    }

    // Helper: Format a single chain path
    fn format_chain_path(
        &self,
        path: &[(PathBuf, usize, String, String)],
        file_map: &HashMap<PathBuf, String>,
    ) -> String {
        path.iter()
            .map(|(file, line, from, to)| {
                let alias = file_map.get(file).cloned().unwrap_or_else(|| {
                    file.file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string()
                });
                format!("{}:{} ({} -> {})", alias, line, from, to)
            })
            .collect::<Vec<_>>()
            .join(" -> ")
    }

    // Helper: Append statistics section
    fn append_statistics(
        &self,
        output: &mut String,
        files_analyzed: &[PathBuf],
        definitions: &[(PathBuf, usize)],
        incoming_chains: &[CallChain],
        outgoing_chains: &[CallChain],
        follow_depth: u32,
    ) {
        output.push_str("STATISTICS:\n");
        output.push_str(&format!("  Files analyzed: {}\n", files_analyzed.len()));
        output.push_str(&format!("  Definitions found: {}\n", definitions.len()));
        output.push_str(&format!("  Incoming chains: {}\n", incoming_chains.len()));
        output.push_str(&format!("  Outgoing chains: {}\n", outgoing_chains.len()));
        output.push_str(&format!("  Follow depth: {}\n", follow_depth));
    }

    // Helper: Append file legend section
    fn append_file_legend(
        &self,
        output: &mut String,
        file_map: &HashMap<PathBuf, String>,
        sorted_files: &[PathBuf],
        definitions: &[(PathBuf, usize)],
        incoming_chains: &[CallChain],
        outgoing_chains: &[CallChain],
    ) {
        if !file_map.is_empty()
            && (sorted_files.len() > 1
                || !incoming_chains.is_empty()
                || !outgoing_chains.is_empty()
                || !definitions.is_empty())
        {
            output.push_str("\nFILES:\n");
            let mut legend_entries: Vec<_> = file_map.iter().collect();
            legend_entries.sort_by_key(|(_, alias)| alias.as_str());

            for (file_path, alias) in legend_entries {
                if sorted_files.len() == 1
                    && alias == file_path.file_name().and_then(|n| n.to_str()).unwrap_or("")
                {
                    continue;
                }
                output.push_str(&format!("  {}: {}\n", alias, file_path.display()));
            }
        }
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
