use rmcp::model::{ErrorCode, ErrorData};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tree_sitter::{Language, Parser, Tree};

use super::lock_or_recover;
use crate::developer::analyze::types::{
    AnalysisResult, CallInfo, ClassInfo, ElementQueryResult, FunctionInfo, ReferenceInfo,
    ReferenceType,
};

/// Manages tree-sitter parsers for different languages
#[derive(Clone)]
pub struct ParserManager {
    parsers: Arc<Mutex<HashMap<String, Arc<Mutex<Parser>>>>>,
}

impl ParserManager {
    pub fn new() -> Self {
        tracing::debug!("Initializing ParserManager");
        Self {
            parsers: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Get or create a parser for the specified language
    pub fn get_or_create_parser(&self, language: &str) -> Result<Arc<Mutex<Parser>>, ErrorData> {
        let mut cache = lock_or_recover(&self.parsers, |c| c.clear());

        if let Some(parser) = cache.get(language) {
            tracing::trace!("Reusing cached parser for {}", language);
            return Ok(Arc::clone(parser));
        }

        tracing::debug!("Creating new parser for {}", language);
        let mut parser = Parser::new();
        let language_config: Language = match language {
            "python" => tree_sitter_python::language(),
            "rust" => tree_sitter_rust::language(),
            "javascript" | "typescript" => tree_sitter_javascript::language(),
            "go" => tree_sitter_go::language(),
            "java" => tree_sitter_java::language(),
            "kotlin" => tree_sitter_kotlin::language(),
            "swift" => devgen_tree_sitter_swift::language(),
            "ruby" => tree_sitter_ruby::language(),
            _ => {
                tracing::warn!("Unsupported language: {}", language);
                return Err(ErrorData::new(
                    ErrorCode::INVALID_PARAMS,
                    format!("Unsupported language: {}", language),
                    None,
                ));
            }
        };

        parser.set_language(&language_config).map_err(|e| {
            tracing::error!("Failed to set language for {}: {}", language, e);
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

    /// Parse source code and return the syntax tree
    pub fn parse(&self, content: &str, language: &str) -> Result<Tree, ErrorData> {
        let parser_arc = self.get_or_create_parser(language)?;
        // Parser doesn't have a clear() method, so we just continue with it
        let mut parser = lock_or_recover(&parser_arc, |_| {});

        parser.parse(content, None).ok_or_else(|| {
            tracing::error!("Failed to parse content as {}", language);
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to parse file as {}", language),
                None,
            )
        })
    }
}

impl Default for ParserManager {
    fn default() -> Self {
        Self::new()
    }
}

/// Extract code elements from a parsed tree
pub struct ElementExtractor;

impl ElementExtractor {
    /// Extract code elements with optional semantic analysis
    pub fn extract_with_depth(
        tree: &Tree,
        source: &str,
        language: &str,
        depth: &str,
    ) -> Result<AnalysisResult, ErrorData> {
        tracing::trace!(
            "Extracting elements from {} code with depth {}",
            language,
            depth
        );

        // First get the structural analysis
        let mut result = Self::extract_elements(tree, source, language)?;

        // For structure mode, clear the detailed vectors but keep the counts
        if depth == "structure" {
            result.functions.clear();
            result.classes.clear();
            result.imports.clear();
        } else if depth == "semantic" {
            // For semantic mode, also extract calls
            let calls = Self::extract_calls(tree, source, language)?;
            result.calls = calls;

            // Also populate references from the calls
            for call in &result.calls {
                result.references.push(ReferenceInfo {
                    symbol: call.callee_name.clone(),
                    ref_type: ReferenceType::Call,
                    line: call.line,
                    context: call.context.clone(),
                    associated_type: None,
                });
            }

            // Languages can opt-in to advanced reference tracking by providing a REFERENCE_QUERY
            // in their language definition. This enables tracking of:
            // - Type instantiation (struct literals, object creation)
            // - Field/variable/parameter type references
            // - Method-to-type associations
            // To add reference tracking for a new language, define REFERENCE_QUERY in
            // languages/<lang>.rs and add the language to the check below.
            if language == "go" {
                let go_references = Self::extract_references(tree, source, language)?;
                result.references.extend(go_references);
            }
        }

        Ok(result)
    }

    /// Extract basic code elements (functions, classes, imports)
    pub fn extract_elements(
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<AnalysisResult, ErrorData> {
        // Get language-specific query
        let query_str = Self::get_element_query(language);
        if query_str.is_empty() {
            return Ok(Self::empty_analysis_result());
        }

        // Parse and process the query
        let (functions, classes, imports) = Self::process_element_query(tree, source, query_str)?;

        // Detect main function
        let main_line = functions.iter().find(|f| f.name == "main").map(|f| f.line);

        Ok(AnalysisResult {
            function_count: functions.len(),
            class_count: classes.len(),
            import_count: imports.len(),
            functions,
            classes,
            imports,
            calls: vec![],
            references: vec![],
            line_count: 0,
            main_line,
        })
    }

    /// Get language-specific query for elements
    fn get_element_query(language: &str) -> &'static str {
        use crate::developer::analyze::languages;

        match language {
            "python" => languages::python::ELEMENT_QUERY,
            "rust" => languages::rust::ELEMENT_QUERY,
            "javascript" | "typescript" => languages::javascript::ELEMENT_QUERY,
            "go" => languages::go::ELEMENT_QUERY,
            "java" => languages::java::ELEMENT_QUERY,
            "kotlin" => languages::kotlin::ELEMENT_QUERY,
            "swift" => languages::swift::ELEMENT_QUERY,
            "ruby" => languages::ruby::ELEMENT_QUERY,
            _ => "",
        }
    }

    /// Process element query and extract functions, classes, imports
    fn process_element_query(
        tree: &Tree,
        source: &str,
        query_str: &str,
    ) -> Result<ElementQueryResult, ErrorData> {
        use tree_sitter::{Query, QueryCursor};

        let mut functions = Vec::new();
        let mut classes = Vec::new();
        let mut imports = Vec::new();

        let query = Query::new(&tree.language(), query_str).map_err(|e| {
            tracing::error!("Failed to create query: {}", e);
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create query: {}", e),
                None,
            )
        })?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        for match_ in matches.by_ref() {
            for capture in match_.captures {
                let node = capture.node;
                let text = &source[node.byte_range()];
                let line = source[..node.start_byte()].lines().count() + 1;

                match query.capture_names()[capture.index as usize] {
                    "func" | "const" => {
                        // Treat constants like functions (defined once, referenced many times)
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

        tracing::trace!(
            "Extracted {} functions, {} classes, {} imports",
            functions.len(),
            classes.len(),
            imports.len()
        );

        Ok((functions, classes, imports))
    }

    /// Get language-specific query for finding function calls
    fn get_call_query(language: &str) -> &'static str {
        use crate::developer::analyze::languages;

        match language {
            "python" => languages::python::CALL_QUERY,
            "rust" => languages::rust::CALL_QUERY,
            "javascript" | "typescript" => languages::javascript::CALL_QUERY,
            "go" => languages::go::CALL_QUERY,
            "java" => languages::java::CALL_QUERY,
            "kotlin" => languages::kotlin::CALL_QUERY,
            "swift" => languages::swift::CALL_QUERY,
            "ruby" => languages::ruby::CALL_QUERY,
            _ => "",
        }
    }

    /// Extract function calls from the parse tree
    fn extract_calls(
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<Vec<CallInfo>, ErrorData> {
        use tree_sitter::{Query, QueryCursor};

        let mut calls = Vec::new();

        // Get language-specific call query
        let query_str = Self::get_call_query(language);
        if query_str.is_empty() {
            return Ok(calls); // No call query for this language
        }

        let query = Query::new(&tree.language(), query_str).map_err(|e| {
            tracing::error!("Failed to create call query: {}", e);
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create call query: {}", e),
                None,
            )
        })?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        for match_ in matches.by_ref() {
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
                let caller_name = Self::find_containing_function(&node, source, language);

                // Add the call based on capture name
                match query.capture_names()[capture.index as usize] {
                    "function.call"
                    | "method.call"
                    | "scoped.call"
                    | "macro.call"
                    | "constructor.call"
                    | "identifier.reference" => {
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

        tracing::trace!("Extracted {} calls", calls.len());
        Ok(calls)
    }

    /// Get language-specific query for struct/type references
    fn get_reference_query(language: &str) -> &'static str {
        use crate::developer::analyze::languages;

        match language {
            "go" => languages::go::REFERENCE_QUERY,
            _ => "",
        }
    }

    /// Extract struct/type references from the parse tree
    fn extract_references(
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<Vec<ReferenceInfo>, ErrorData> {
        use tree_sitter::{Query, QueryCursor};

        let mut references = Vec::new();

        // Get language-specific reference query
        let query_str = Self::get_reference_query(language);
        if query_str.is_empty() {
            return Ok(references);
        }

        let query = Query::new(&tree.language(), query_str).map_err(|e| {
            tracing::error!("Failed to create reference query: {}", e);
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to create reference query: {}", e),
                None,
            )
        })?;

        let mut cursor = QueryCursor::new();
        let mut matches = cursor.matches(&query, tree.root_node(), source.as_bytes());

        for match_ in matches.by_ref() {
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

                let capture_name = query.capture_names()[capture.index as usize];

                // Map capture types to reference types with appropriate semantics
                let (ref_type, symbol, associated_type) = match capture_name {
                    "method.receiver" => {
                        // For method receivers, extract the method name and associate it with the type
                        let method_name =
                            Self::find_method_name_for_receiver(&node, source, language);
                        if let Some(method_name) = method_name {
                            (
                                ReferenceType::MethodDefinition,
                                method_name,
                                Some(text.to_string()), // type name
                            )
                        } else {
                            continue; // Skip if we can't find the method name
                        }
                    }
                    "struct.literal" => (ReferenceType::TypeInstantiation, text.to_string(), None),
                    "field.type" => (ReferenceType::FieldType, text.to_string(), None),
                    "param.type" => (ReferenceType::ParameterType, text.to_string(), None),
                    "var.type" | "shortvar.type" => {
                        (ReferenceType::VariableType, text.to_string(), None)
                    }
                    "type.assertion" | "type.conversion" => {
                        (ReferenceType::Call, text.to_string(), None)
                    }
                    _ => continue,
                };

                references.push(ReferenceInfo {
                    symbol,
                    ref_type,
                    line: start_pos.row + 1,
                    context,
                    associated_type,
                });
            }
        }

        tracing::trace!("Extracted {} struct references", references.len());
        Ok(references)
    }

    /// Find the method name for a method receiver node
    fn find_method_name_for_receiver(
        receiver_node: &tree_sitter::Node,
        source: &str,
        language: &str,
    ) -> Option<String> {
        // Walk up to find the method_declaration parent
        let mut current = *receiver_node;
        while let Some(parent) = current.parent() {
            if language == "go" && parent.kind() == "method_declaration" {
                // Find the method name within the method_declaration
                for i in 0..parent.child_count() {
                    if let Some(child) = parent.child(i) {
                        if child.kind() == "field_identifier" {
                            return Some(source[child.byte_range()].to_string());
                        }
                    }
                }
            }
            current = parent;
        }
        None
    }

    /// Find which function contains a given node
    fn find_containing_function(
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
                "kotlin" => kind == "function_declaration" || kind == "class_body",
                "swift" => {
                    kind == "function_declaration"
                        || kind == "init_declaration"
                        || kind == "deinit_declaration"
                        || kind == "subscript_declaration"
                }
                "ruby" => kind == "method" || kind == "singleton_method",
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
                            || (language == "swift" && child.kind() == "simple_identifier")
                        {
                            // For Python, skip the first identifier if it's 'def'
                            if language == "python" && i == 0 {
                                continue;
                            }
                            // For Swift init/deinit, use special names
                            if language == "swift" {
                                if kind == "init_declaration" {
                                    return Some("init".to_string());
                                } else if kind == "deinit_declaration" {
                                    return Some("deinit".to_string());
                                }
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

    /// Create an empty analysis result
    fn empty_analysis_result() -> AnalysisResult {
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
}
