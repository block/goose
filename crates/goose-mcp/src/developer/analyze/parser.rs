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
    /// Find a child node matching one of the specified kinds
    fn find_child_by_kind<'a>(
        node: &'a tree_sitter::Node,
        kinds: &[&str],
    ) -> Option<tree_sitter::Node<'a>> {
        (0..node.child_count())
            .filter_map(|i| node.child(i))
            .find(|child| kinds.contains(&child.kind()))
    }

    /// Extract text from a child node matching one of the specified kinds
    fn extract_text_from_child(
        node: &tree_sitter::Node,
        source: &str,
        kinds: &[&str],
    ) -> Option<String> {
        Self::find_child_by_kind(node, kinds).map(|child| source[child.byte_range()].to_string())
    }

    /// Extract code elements with optional semantic analysis
    pub fn extract_with_depth(
        tree: &Tree,
        source: &str,
        language: &str,
        depth: &str,
    ) -> Result<AnalysisResult, ErrorData> {
        use crate::developer::analyze::languages;

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
            // The presence of a non-empty reference query indicates support for this feature.
            if !languages::get_reference_query(language).is_empty() {
                let references = Self::extract_references(tree, source, language)?;
                result.references.extend(references);
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
        use crate::developer::analyze::languages;

        // Get language-specific query
        let query_str = languages::get_element_query(language);
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

    /// Extract function calls from the parse tree
    fn extract_calls(
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<Vec<CallInfo>, ErrorData> {
        use crate::developer::analyze::languages;
        use tree_sitter::{Query, QueryCursor};

        let mut calls = Vec::new();

        // Get language-specific call query
        let query_str = languages::get_call_query(language);
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

    /// Extract struct/type references from the parse tree
    fn extract_references(
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<Vec<ReferenceInfo>, ErrorData> {
        use crate::developer::analyze::languages;
        use tree_sitter::{Query, QueryCursor};

        let mut references = Vec::new();

        // Get language-specific reference query
        let query_str = languages::get_reference_query(language);
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
        use crate::developer::analyze::languages;

        // Delegate to language-specific implementations
        languages::find_method_for_receiver(receiver_node, source, language)
    }

    /// Find which function contains a given node
    fn find_containing_function(
        node: &tree_sitter::Node,
        source: &str,
        language: &str,
    ) -> Option<String> {
        use crate::developer::analyze::languages;

        let function_kinds = languages::get_function_node_kinds(language);
        let name_kinds = languages::get_function_name_kinds(language);

        let mut current = *node;

        // Walk up the tree to find a function definition
        while let Some(parent) = current.parent() {
            let kind = parent.kind();

            // Check if this is a function-like node
            if function_kinds.contains(&kind) {
                // Two-step extraction process:
                // 1. Try language-specific extraction for special cases (e.g., Rust impl blocks, Swift init/deinit)
                // 2. Fall back to generic extraction using standard identifier node kinds
                // This pattern allows languages to override default behavior when needed
                if let Some(name) =
                    languages::extract_function_name_for_kind(&parent, source, language, kind)
                {
                    return Some(name);
                }

                // Standard extraction: find first child matching expected identifier kinds
                if let Some(name) = Self::extract_text_from_child(&parent, source, name_kinds) {
                    return Some(name);
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
