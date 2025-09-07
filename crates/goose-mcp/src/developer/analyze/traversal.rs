use ignore::gitignore::Gitignore;
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
    pub async fn collect_files_for_focused(
        &self,
        path: &Path,
        max_depth: u32,
    ) -> Result<Vec<PathBuf>, ErrorData> {
        tracing::debug!("Collecting files from {:?} with max_depth {}", path, max_depth);
        
        if max_depth == 0 {
            tracing::warn!("Unlimited depth traversal requested for {:?}", path);
        }
        
        let files = self.collect_files_recursive(path, 0, max_depth).await?;
        
        tracing::info!("Collected {} files from {:?}", files.len(), path);
        Ok(files)
    }

    /// Recursively collect files
    async fn collect_files_recursive(
        &self,
        path: &Path,
        current_depth: u32,
        max_depth: u32,
    ) -> Result<Vec<PathBuf>, ErrorData> {
        let mut files = Vec::new();

        // max_depth of 0 means unlimited depth
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
                let mut sub_files = Box::pin(self.collect_files_recursive(
                    &entry_path,
                    current_depth + 1,
                    max_depth,
                ))
                .await?;
                files.append(&mut sub_files);
            }
        }

        Ok(files)
    }

    /// Collect directory results for analysis
    pub async fn collect_directory_results<F, Fut>(
        &self,
        path: &Path,
        max_depth: u32,
        mut analyze_file: F,
    ) -> Result<Vec<(PathBuf, EntryType)>, ErrorData>
    where
        F: FnMut(&Path) -> Fut,
        Fut: std::future::Future<Output = Result<AnalysisResult, ErrorData>>,
    {
        tracing::debug!("Collecting directory results from {:?}", path);
        
        self.collect_directory_recursive(path, 0, max_depth, &mut analyze_file).await
    }

    /// Recursively collect directory results
    async fn collect_directory_recursive<F, Fut>(
        &self,
        path: &Path,
        depth: u32,
        max_depth: u32,
        analyze_file: &mut F,
    ) -> Result<Vec<(PathBuf, EntryType)>, ErrorData>
    where
        F: FnMut(&Path) -> Fut,
        Fut: std::future::Future<Output = Result<AnalysisResult, ErrorData>>,
    {
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
            if self.is_ignored(&entry_path) {
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
                                results.push((entry_path, EntryType::SymlinkFile(target)));
                            }
                        }
                        Err(_) => {
                            // Broken symlink - skip
                            tracing::trace!("Skipping broken symlink {:?}", entry_path);
                        }
                    }
                }
            } else if metadata.is_dir() {
                if max_depth > 0 && depth + 1 >= max_depth {
                    // At max depth, just mark as directory
                    results.push((entry_path, EntryType::Directory));
                } else {
                    // Recurse into subdirectory
                    let mut sub_results = Box::pin(self.collect_directory_recursive(
                        &entry_path,
                        depth + 1,
                        max_depth,
                        analyze_file,
                    ))
                    .await?;
                    results.append(&mut sub_results);
                }
            } else if metadata.is_file() {
                // Only analyze supported file types
                let lang = lang::get_language_identifier(&entry_path);
                if !lang.is_empty() {
                    match analyze_file(&entry_path).await {
                        Ok(result) => {
                            if result.function_count > 0 
                                || result.class_count > 0 
                                || result.line_count > 0 
                            {
                                results.push((entry_path, EntryType::File(result)));
                            }
                        }
                        Err(e) => {
                            tracing::warn!("Failed to analyze {:?}: {}", entry_path, e);
                            // Continue with other files
                        }
                    }
                }
            }
        }

        Ok(results)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_gitignore() -> Gitignore {
        let mut builder = ignore::gitignore::GitignoreBuilder::new(".");
        builder.add_line(None, "*.log").unwrap();
        builder.add_line(None, "node_modules/").unwrap();
        builder.build().unwrap()
    }

    #[tokio::test]
    async fn test_is_ignored() {
        let ignore = create_test_gitignore();
        let traverser = FileTraverser::new(&ignore);

        assert!(traverser.is_ignored(Path::new("test.log")));
        assert!(traverser.is_ignored(Path::new("node_modules/package.json")));
        assert!(!traverser.is_ignored(Path::new("src/main.rs")));
    }

    #[tokio::test]
    async fn test_validate_path() {
        let ignore = create_test_gitignore();
        let traverser = FileTraverser::new(&ignore);

        // Test non-existent path
        assert!(traverser.validate_path(Path::new("/nonexistent/path")).is_err());

        // Test ignored path
        assert!(traverser.validate_path(Path::new("test.log")).is_err());
    }

    #[tokio::test]
    async fn test_collect_files() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create test files
        fs::write(dir_path.join("test.rs"), "fn main() {}").unwrap();
        fs::write(dir_path.join("test.py"), "def main(): pass").unwrap();
        fs::write(dir_path.join("test.txt"), "not code").unwrap();
        
        // Create subdirectory with file
        let sub_dir = dir_path.join("src");
        fs::create_dir(&sub_dir).unwrap();
        fs::write(sub_dir.join("lib.rs"), "pub fn test() {}").unwrap();

        let ignore = Gitignore::empty();
        let traverser = FileTraverser::new(&ignore);

        let files = traverser.collect_files_for_focused(dir_path, 0).await.unwrap();

        // Should find .rs and .py files but not .txt
        assert_eq!(files.len(), 3);
        assert!(files.iter().any(|p| p.ends_with("test.rs")));
        assert!(files.iter().any(|p| p.ends_with("test.py")));
        assert!(files.iter().any(|p| p.ends_with("lib.rs")));
    }

    #[tokio::test]
    async fn test_max_depth() {
        let temp_dir = TempDir::new().unwrap();
        let dir_path = temp_dir.path();

        // Create nested structure
        let level1 = dir_path.join("level1");
        fs::create_dir(&level1).unwrap();
        fs::write(level1.join("file1.rs"), "").unwrap();

        let level2 = level1.join("level2");
        fs::create_dir(&level2).unwrap();
        fs::write(level2.join("file2.rs"), "").unwrap();

        let ignore = Gitignore::empty();
        let traverser = FileTraverser::new(&ignore);

        // With max_depth=1, should only find file1.rs
        let files = traverser.collect_files_for_focused(dir_path, 1).await.unwrap();
        assert_eq!(files.len(), 1);
        assert!(files[0].ends_with("file1.rs"));

        // With max_depth=2, should find both files
        let files = traverser.collect_files_for_focused(dir_path, 2).await.unwrap();
        assert_eq!(files.len(), 2);
    }
}
