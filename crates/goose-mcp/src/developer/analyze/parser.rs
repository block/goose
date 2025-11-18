use rmcp::model::{ErrorCode, ErrorData};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tree_sitter::{Language, Parser, Tree};

use super::lock_or_recover;
use crate::developer::analyze::types::{
    AnalysisResult, CallInfo, ClassInfo, ElementQueryResult, FunctionInfo, ReferenceInfo,
    ReferenceType,
};

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

    pub fn parse(&self, content: &str, language: &str) -> Result<Tree, ErrorData> {
        let parser_arc = self.get_or_create_parser(language)?;
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

pub struct ElementExtractor;

impl ElementExtractor {
    fn find_child_by_kind<'a>(
        node: &'a tree_sitter::Node,
        kinds: &[&str],
    ) -> Option<tree_sitter::Node<'a>> {
        (0..node.child_count())
            .filter_map(|i| node.child(i))
            .find(|child| kinds.contains(&child.kind()))
    }

    fn extract_text_from_child(
        node: &tree_sitter::Node,
        source: &str,
        kinds: &[&str],
    ) -> Option<String> {
        Self::find_child_by_kind(node, kinds)
            .and_then(|child| source.get(child.byte_range()).map(|s| s.to_string()))
    }

    pub fn extract_with_depth(
        tree: &Tree,
        source: &str,
        language: &str,
        depth: &str,
        ast_recursion_limit: Option<usize>,
    ) -> Result<AnalysisResult, ErrorData> {
        use crate::developer::analyze::languages;

        tracing::trace!(
            "Extracting elements from {} code with depth {}",
            language,
            depth
        );

        let mut result = Self::extract_elements(tree, source, language)?;

        if depth == "structure" {
            result.functions.clear();
            result.classes.clear();
            result.imports.clear();
        } else if depth == "semantic" {
            let calls = Self::extract_calls(tree, source, language)?;
            result.calls = calls;

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
            if let Some(info) = languages::get_language_info(language) {
                if !info.reference_query.is_empty() {
                    let references =
                        Self::extract_references(tree, source, language, ast_recursion_limit)?;
                    result.references.extend(references);
                }
            }
        }

        Ok(result)
    }

    pub fn extract_elements(
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<AnalysisResult, ErrorData> {
        use crate::developer::analyze::languages;

        let info = match languages::get_language_info(language) {
            Some(info) if !info.element_query.is_empty() => info,
            _ => return Ok(Self::empty_analysis_result()),
        };

        let query_str = info.element_query;

        let (functions, classes, imports) = Self::process_element_query(tree, source, query_str)?;

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
                let Some(text) = source.get(node.byte_range()) else {
                    continue;
                };
                let line = source
                    .get(..node.start_byte())
                    .map(|s| s.lines().count() + 1)
                    .unwrap_or(1);

                match query.capture_names()[capture.index as usize] {
                    "func" | "const" => {
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

    fn extract_calls(
        tree: &Tree,
        source: &str,
        language: &str,
    ) -> Result<Vec<CallInfo>, ErrorData> {
        use crate::developer::analyze::languages;
        use tree_sitter::{Query, QueryCursor};

        let mut calls = Vec::new();

        let info = match languages::get_language_info(language) {
            Some(info) if !info.call_query.is_empty() => info,
            _ => return Ok(calls),
        };

        let query_str = info.call_query;

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
                let Some(text) = source.get(node.byte_range()) else {
                    continue;
                };
                let start_pos = node.start_position();

                let line_start = source
                    .get(..node.start_byte())
                    .and_then(|s| s.rfind('\n'))
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let line_end = source
                    .get(node.end_byte()..)
                    .and_then(|s| s.find('\n'))
                    .map(|i| node.end_byte() + i)
                    .unwrap_or(source.len());
                let context = source
                    .get(line_start..line_end)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                let caller_name = Self::find_containing_function(&node, source, language);

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

    fn extract_references(
        tree: &Tree,
        source: &str,
        language: &str,
        ast_recursion_limit: Option<usize>,
    ) -> Result<Vec<ReferenceInfo>, ErrorData> {
        use crate::developer::analyze::languages;
        use tree_sitter::{Query, QueryCursor};

        let mut references = Vec::new();

        let info = match languages::get_language_info(language) {
            Some(info) if !info.reference_query.is_empty() => info,
            _ => return Ok(references),
        };

        let query_str = info.reference_query;

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
                let Some(text) = source.get(node.byte_range()) else {
                    continue;
                };
                let start_pos = node.start_position();

                let line_start = source
                    .get(..node.start_byte())
                    .and_then(|s| s.rfind('\n'))
                    .map(|i| i + 1)
                    .unwrap_or(0);
                let line_end = source
                    .get(node.end_byte()..)
                    .and_then(|s| s.find('\n'))
                    .map(|i| node.end_byte() + i)
                    .unwrap_or(source.len());
                let context = source
                    .get(line_start..line_end)
                    .map(|s| s.trim().to_string())
                    .unwrap_or_default();

                let capture_name = query.capture_names()[capture.index as usize];

                let (ref_type, symbol, associated_type) = match capture_name {
                    "method.receiver" => {
                        let method_name = Self::find_method_name_for_receiver(
                            &node,
                            source,
                            language,
                            ast_recursion_limit,
                        );
                        if let Some(method_name) = method_name {
                            // Use language-specific handler to find receiver type, or fall back to text
                            let type_name = Self::find_receiver_type(&node, source, language)
                                .or_else(|| Some(text.to_string()));

                            if let Some(type_name) = type_name {
                                (
                                    ReferenceType::MethodDefinition,
                                    method_name,
                                    Some(type_name),
                                )
                            } else {
                                continue;
                            }
                        } else {
                            continue;
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

    fn find_method_name_for_receiver(
        receiver_node: &tree_sitter::Node,
        source: &str,
        language: &str,
        ast_recursion_limit: Option<usize>,
    ) -> Option<String> {
        use crate::developer::analyze::languages;

        languages::get_language_info(language)
            .and_then(|info| info.find_method_for_receiver_handler)
            .and_then(|handler| handler(receiver_node, source, ast_recursion_limit))
    }

    fn find_receiver_type(
        receiver_node: &tree_sitter::Node,
        source: &str,
        language: &str,
    ) -> Option<String> {
        use crate::developer::analyze::languages;

        languages::get_language_info(language)
            .and_then(|info| info.find_receiver_type_handler)
            .and_then(|handler| handler(receiver_node, source))
    }

    fn find_containing_function(
        node: &tree_sitter::Node,
        source: &str,
        language: &str,
    ) -> Option<String> {
        use crate::developer::analyze::languages;

        let info = languages::get_language_info(language)?;

        let mut current = *node;

        while let Some(parent) = current.parent() {
            let kind = parent.kind();

            // Check if this is a function-like node
            if info.function_node_kinds.contains(&kind) {
                // Two-step extraction process:
                // 1. Try language-specific extraction for special cases (e.g., Rust impl blocks, Swift init/deinit)
                // 2. Fall back to generic extraction using standard identifier node kinds
                // This pattern allows languages to override default behavior when needed
                if let Some(handler) = info.extract_function_name_handler {
                    if let Some(name) = handler(&parent, source, kind) {
                        return Some(name);
                    }
                }

                // Standard extraction: find first child matching expected identifier kinds
                if let Some(name) =
                    Self::extract_text_from_child(&parent, source, info.function_name_kinds)
                {
                    return Some(name);
                }
            }

            current = parent;
        }

        None
    }

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
