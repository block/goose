use ignore::gitignore::Gitignore;
use rayon::prelude::*;
use rmcp::model::{ErrorCode, ErrorData};
use std::path::{Path, PathBuf};

use crate::developer::analyze::types::{AnalysisResult, EntryType};
use crate::developer::lang;

/// Handles file system traversal with ignore patterns
pub struct FileTraverser<'a> {
    ignore_patterns: &'a Gitignore,
}

impl<'a> FileTraverser<'a> {
    /// Create a new file traverser with the given ignore patterns
    pub fn new(ignore_patterns: &'a Gitignore) -> Self {
        Self { ignore_patterns }
    }

    /// Check if a path should be ignored
    pub fn is_ignored(&self, path: &Path) -> bool {
        let ignored = self.ignore_patterns.matched(path, false).is_ignore();
        if ignored {
            tracing::trace!("Path {:?} is ignored", path);
        }
        ignored
    }

    /// Validate that a path exists and is not ignored
    pub fn validate_path(&self, path: &Path) -> Result<(), ErrorData> {
        // Check if path is ignored
        if self.is_ignored(path) {
            return Err(ErrorData::new(
                ErrorCode::INVALID_PARAMS,
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
                ErrorCode::INVALID_PARAMS,
                format!("Path '{}' does not exist", path.display()),
                None,
            ));
        }

        Ok(())
    }

    /// Collect all files for focused analysis
    pub fn collect_files_for_focused(
        &self,
        path: &Path,
        max_depth: u32,
    ) -> Result<Vec<PathBuf>, ErrorData> {
        tracing::debug!(
            "Collecting files from {:?} with max_depth {}",
            path,
            max_depth
        );

        if max_depth == 0 {
            tracing::warn!("Unlimited depth traversal requested for {:?}", path);
        }

        let files = self.collect_files_recursive(path, 0, max_depth)?;

        tracing::info!("Collected {} files from {:?}", files.len(), path);
        Ok(files)
    }

    /// Recursively collect files
    fn collect_files_recursive(
        &self,
        path: &Path,
        current_depth: u32,
        max_depth: u32,
    ) -> Result<Vec<PathBuf>, ErrorData> {
        let mut files = Vec::new();

        // Check if we're at a file (base case)
        if path.is_file() {
            let lang = lang::get_language_identifier(path);
            if !lang.is_empty() {
                tracing::trace!("Including file {:?} (language: {})", path, lang);
                files.push(path.to_path_buf());
            }
            return Ok(files);
        }

        // max_depth of 0 means unlimited depth
        // current_depth starts at 0, max_depth is the number of directory levels to traverse
        if max_depth > 0 && current_depth >= max_depth {
            tracing::trace!("Reached max depth {} at {:?}", max_depth, path);
            return Ok(files);
        }

        let entries = std::fs::read_dir(path).map_err(|e| {
            tracing::error!("Failed to read directory {:?}: {}", path, e);
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
            if self.is_ignored(&entry_path) {
                continue;
            }

            if entry_path.is_file() {
                // Only include supported file types
                let lang = lang::get_language_identifier(&entry_path);
                if !lang.is_empty() {
                    tracing::trace!("Including file {:?} (language: {})", entry_path, lang);
                    files.push(entry_path);
                }
            } else if entry_path.is_dir() {
                // Recurse into subdirectory
                let mut sub_files =
                    self.collect_files_recursive(&entry_path, current_depth + 1, max_depth)?;
                files.append(&mut sub_files);
            }
        }

        Ok(files)
    }

    /// Collect directory results for analysis with parallel processing
    pub fn collect_directory_results<F>(
        &self,
        path: &Path,
        max_depth: u32,
        analyze_file: F,
    ) -> Result<Vec<(PathBuf, EntryType)>, ErrorData>
    where
        F: Fn(&Path) -> Result<AnalysisResult, ErrorData> + Sync,
    {
        tracing::debug!("Collecting directory results from {:?}", path);

        // First collect all files to analyze
        let files_to_analyze = self.collect_files_recursive(path, 0, max_depth)?;

        // Then analyze them in parallel using Rayon
        let results: Result<Vec<_>, ErrorData> = files_to_analyze
            .par_iter()
            .map(|file_path| {
                analyze_file(file_path).map(|result| (file_path.clone(), EntryType::File(result)))
            })
            .collect();

        results
    }
}
