pub mod cache;
pub mod formatter;
pub mod graph;
pub mod languages;
pub mod parser;
pub mod traversal;
pub mod types;

#[cfg(test)]
mod tests;

use ignore::gitignore::Gitignore;
use rmcp::model::{CallToolResult, ErrorCode, ErrorData};
use std::path::{Path, PathBuf};

use crate::developer::lang;

use self::cache::AnalysisCache;
use self::formatter::Formatter;
use self::graph::CallGraph;
use self::parser::{ElementExtractor, ParserManager};
use self::traversal::FileTraverser;
use self::types::{AnalysisMode, AnalysisResult, AnalyzeParams, FocusedAnalysisData};

/// Helper to safely lock a mutex with poison recovery
/// The recovery function is called on the mutex contents if the lock was poisoned
pub(crate) fn lock_or_recover<T, F>(
    mutex: &std::sync::Mutex<T>,
    recovery: F,
) -> std::sync::MutexGuard<'_, T>
where
    F: FnOnce(&mut T),
{
    mutex.lock().unwrap_or_else(|poisoned| {
        let mut guard = poisoned.into_inner();
        recovery(&mut guard);
        tracing::warn!("Recovered from poisoned lock");
        guard
    })
}

/// Code analyzer with caching and tree-sitter parsing
#[derive(Clone)]
pub struct CodeAnalyzer {
    parser_manager: ParserManager,
    cache: AnalysisCache,
}

impl Default for CodeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl CodeAnalyzer {
    pub fn new() -> Self {
        tracing::debug!("Initializing CodeAnalyzer");
        Self {
            parser_manager: ParserManager::new(),
            cache: AnalysisCache::new(100),
        }
    }

    pub fn analyze(
        &self,
        params: AnalyzeParams,
        path: PathBuf,
        ignore_patterns: &Gitignore,
    ) -> Result<CallToolResult, ErrorData> {
        tracing::info!("Starting analysis of {:?} with params {:?}", path, params);

        let traverser = FileTraverser::new(ignore_patterns);

        traverser.validate_path(&path)?;

        let mode = self.determine_mode(&params, &path);

        tracing::debug!("Using analysis mode: {:?}", mode);

        let mut output = match mode {
            AnalysisMode::Focused => self.analyze_focused(&path, &params, &traverser)?,
            AnalysisMode::Semantic => {
                if path.is_file() {
                    let result = self.analyze_file(&path, &mode, &params)?;
                    Formatter::format_analysis_result(&path, &result, &mode)
                } else {
                    self.analyze_directory(&path, &params, &traverser, &mode)?
                }
            }
            AnalysisMode::Structure => {
                if path.is_file() {
                    let result = self.analyze_file(&path, &mode, &params)?;
                    Formatter::format_analysis_result(&path, &result, &mode)
                } else {
                    self.analyze_directory(&path, &params, &traverser, &mode)?
                }
            }
        };

        // If focus is specified with non-focused mode, filter results
        if let Some(focus) = &params.focus {
            if mode != AnalysisMode::Focused {
                output = Formatter::filter_by_focus(&output, focus);
            }
        }

        const OUTPUT_LIMIT: usize = 1000;
        if !params.force {
            let line_count = output.lines().count();
            if line_count > OUTPUT_LIMIT {
                let warning = format!(
                    "LARGE OUTPUT WARNING\n\n\
                    The analysis would produce {} lines (~{} tokens).\n\
                    This exceeds the {} line limit.\n\n\
                    To proceed anyway, add 'force: true' to your parameters:\n\
                    analyze path=\"{}\" force=true{}\n\n\
                    Or narrow your scope by:\n\
                    • Analyzing a subdirectory instead\n\
                    • Using focus mode: focus=\"symbol_name\"\n\
                    • Reducing depth: max_depth=1",
                    line_count,
                    line_count * 10, // rough token estimate
                    OUTPUT_LIMIT,
                    path.display(),
                    if let Some(f) = &params.focus {
                        format!(" focus=\"{}\"", f)
                    } else {
                        String::new()
                    }
                );
                return Ok(CallToolResult::success(vec![rmcp::model::Content::text(
                    warning,
                )]));
            }
        }

        tracing::info!("Analysis complete");
        Ok(CallToolResult::success(Formatter::format_results(output)))
    }

    fn determine_mode(&self, params: &AnalyzeParams, path: &Path) -> AnalysisMode {
        if params.focus.is_some() {
            return AnalysisMode::Focused;
        }

        if path.is_file() {
            AnalysisMode::Semantic
        } else {
            AnalysisMode::Structure
        }
    }

    fn analyze_file(
        &self,
        path: &Path,
        mode: &AnalysisMode,
        params: &AnalyzeParams,
    ) -> Result<AnalysisResult, ErrorData> {
        tracing::debug!("Analyzing file {:?} in {:?} mode", path, mode);

        let metadata = std::fs::metadata(path).map_err(|e| {
            tracing::error!("Failed to get file metadata for {:?}: {}", path, e);
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!("Failed to get metadata for '{}': {}", path.display(), e),
                None,
            )
        })?;

        let modified = metadata.modified().map_err(|e| {
            ErrorData::new(
                ErrorCode::INTERNAL_ERROR,
                format!(
                    "Failed to get modification time for '{}': {}",
                    path.display(),
                    e
                ),
                None,
            )
        })?;

        if let Some(cached) = self.cache.get(&path.to_path_buf(), modified, mode) {
            tracing::trace!("Using cached result for {:?}", path);
            return Ok(cached);
        }

        let content = match std::fs::read_to_string(path) {
            Ok(content) => content,
            Err(e) => {
                tracing::trace!("Skipping binary/non-UTF-8 file {:?}: {}", path, e);
                return Ok(AnalysisResult::empty(0));
            }
        };

        let line_count = content.lines().count();

        let language = lang::get_language_identifier(path);
        if language.is_empty() {
            tracing::trace!("Unsupported file type: {:?}", path);
            return Ok(AnalysisResult::empty(line_count));
        }

        // Check if we support this language for parsing
        // A language is supported if it has query definitions
        let language_supported = languages::get_language_info(language)
            .map(|info| !info.element_query.is_empty())
            .unwrap_or(false);

        if !language_supported {
            tracing::trace!("Language {} not supported for parsing", language);
            return Ok(AnalysisResult::empty(line_count));
        }

        let tree = self.parser_manager.parse(&content, language)?;

        let depth = mode.as_str();
        let mut result = ElementExtractor::extract_with_depth(
            &tree,
            &content,
            language,
            depth,
            params.ast_recursion_limit,
        )?;

        result.line_count = line_count;

        self.cache
            .put(path.to_path_buf(), modified, mode, result.clone());

        Ok(result)
    }

    fn analyze_directory(
        &self,
        path: &Path,
        params: &AnalyzeParams,
        traverser: &FileTraverser<'_>,
        mode: &AnalysisMode,
    ) -> Result<String, ErrorData> {
        tracing::debug!("Analyzing directory {:?} in {:?} mode", path, mode);

        let mode = *mode;

        let results = traverser.collect_directory_results(path, params.max_depth, |file_path| {
            self.analyze_file(file_path, &mode, params)
        })?;

        Ok(Formatter::format_directory_structure(
            path,
            &results,
            params.max_depth,
        ))
    }

    fn analyze_focused(
        &self,
        path: &Path,
        params: &AnalyzeParams,
        traverser: &FileTraverser<'_>,
    ) -> Result<String, ErrorData> {
        let focus_symbol = params.focus.as_ref().ok_or_else(|| {
            ErrorData::new(
                ErrorCode::INVALID_PARAMS,
                "Focused mode requires 'focus' parameter to specify the symbol to track"
                    .to_string(),
                None,
            )
        })?;

        tracing::info!("Running focused analysis for symbol '{}'", focus_symbol);

        let files_to_analyze = if path.is_file() {
            vec![path.to_path_buf()]
        } else {
            traverser.collect_files_for_focused(path, params.max_depth)?
        };

        tracing::debug!(
            "Analyzing {} files for focused analysis",
            files_to_analyze.len()
        );

        use rayon::prelude::*;
        let all_results: Result<Vec<_>, _> = files_to_analyze
            .par_iter()
            .map(|file_path| {
                self.analyze_file(file_path, &AnalysisMode::Semantic, params)
                    .map(|result| (file_path.clone(), result))
            })
            .collect();
        let all_results = all_results?;

        let graph = CallGraph::build_from_results(&all_results);

        let incoming_chains = if params.follow_depth > 0 {
            graph.find_incoming_chains(focus_symbol, params.follow_depth)
        } else {
            vec![]
        };

        let outgoing_chains = if params.follow_depth > 0 {
            graph.find_outgoing_chains(focus_symbol, params.follow_depth)
        } else {
            vec![]
        };

        let definitions = graph
            .definitions
            .get(focus_symbol)
            .cloned()
            .unwrap_or_default();

        let focus_data = FocusedAnalysisData {
            focus_symbol,
            follow_depth: params.follow_depth,
            files_analyzed: &files_to_analyze,
            definitions: &definitions,
            incoming_chains: &incoming_chains,
            outgoing_chains: &outgoing_chains,
        };

        let mut output = Formatter::format_focused_output(&focus_data);

        if path.is_file() {
            let hint = "NOTE: Focus mode works best with directory paths. \
                        Use a parent directory in the path for cross-file analysis.\n\n";
            output = format!("{}{}", hint, output);
        }

        Ok(output)
    }
}
