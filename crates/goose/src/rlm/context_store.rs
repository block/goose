//! Context storage for RLM mode
//!
//! Stores large contexts to the filesystem and provides methods for
//! reading slices and calculating chunk boundaries.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use tokio::fs;
use tokio::io::{AsyncReadExt, AsyncSeekExt};

/// Metadata about stored context
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ContextMetadata {
    /// Total length in characters
    pub length: usize,
    /// Path to the context file
    pub path: PathBuf,
    /// Number of chunks based on default chunk size
    pub chunk_count: usize,
    /// Chunk boundaries as (start, end) pairs
    pub chunk_boundaries: Vec<(usize, usize)>,
}

/// Stores and manages large context for RLM processing
pub struct ContextStore {
    session_dir: PathBuf,
    context_file: PathBuf,
    chunk_size: usize,
}

impl ContextStore {
    /// Default chunk size (~500K characters)
    pub const DEFAULT_CHUNK_SIZE: usize = 500_000;

    /// Create a new context store for a session
    pub fn new(session_dir: PathBuf) -> Self {
        let context_file = session_dir.join("rlm_context.txt");
        Self {
            session_dir,
            context_file,
            chunk_size: Self::DEFAULT_CHUNK_SIZE,
        }
    }

    /// Create a context store with a custom chunk size
    pub fn with_chunk_size(session_dir: PathBuf, chunk_size: usize) -> Self {
        let context_file = session_dir.join("rlm_context.txt");
        Self {
            session_dir,
            context_file,
            chunk_size,
        }
    }

    /// Store context to file and return metadata
    pub async fn store_context(&self, content: &str) -> Result<ContextMetadata> {
        // Ensure directory exists
        fs::create_dir_all(&self.session_dir)
            .await
            .context("Failed to create session directory")?;

        // Write content to file
        fs::write(&self.context_file, content)
            .await
            .context("Failed to write context file")?;

        let length = content.len();
        let chunk_boundaries = self.calculate_chunk_boundaries(length);
        let chunk_count = chunk_boundaries.len();

        Ok(ContextMetadata {
            length,
            path: self.context_file.clone(),
            chunk_count,
            chunk_boundaries,
        })
    }

    /// Read the entire context from storage
    pub async fn read_context(&self) -> Result<String> {
        fs::read_to_string(&self.context_file)
            .await
            .context("Failed to read context file")
    }

    /// Read a slice of the context (by character position)
    pub async fn read_slice(&self, start: usize, end: usize) -> Result<String> {
        let content = self.read_context().await?;

        let start = start.min(content.len());
        let end = end.min(content.len());

        if start >= end {
            return Ok(String::new());
        }

        // Handle character boundaries properly for UTF-8
        let chars: Vec<char> = content.chars().collect();
        let start = start.min(chars.len());
        let end = end.min(chars.len());

        Ok(chars[start..end].iter().collect())
    }

    /// Read a slice by byte position (more efficient for large files)
    pub async fn read_slice_bytes(&self, start: usize, end: usize) -> Result<String> {
        let mut file = fs::File::open(&self.context_file)
            .await
            .context("Failed to open context file")?;

        let file_len = file.metadata().await?.len() as usize;
        let start = start.min(file_len);
        let end = end.min(file_len);

        if start >= end {
            return Ok(String::new());
        }

        file.seek(std::io::SeekFrom::Start(start as u64)).await?;

        let mut buffer = vec![0u8; end - start];
        file.read_exact(&mut buffer).await?;

        String::from_utf8(buffer).context("Invalid UTF-8 in context slice")
    }

    /// Get metadata for the stored context
    pub async fn get_metadata(&self) -> Result<ContextMetadata> {
        let content = self.read_context().await?;
        let length = content.len();
        let chunk_boundaries = self.calculate_chunk_boundaries(length);
        let chunk_count = chunk_boundaries.len();

        Ok(ContextMetadata {
            length,
            path: self.context_file.clone(),
            chunk_count,
            chunk_boundaries,
        })
    }

    /// Calculate chunk boundaries for a given content length
    fn calculate_chunk_boundaries(&self, length: usize) -> Vec<(usize, usize)> {
        if length == 0 {
            return vec![];
        }

        let mut boundaries = Vec::new();
        let mut start = 0;

        while start < length {
            let end = (start + self.chunk_size).min(length);
            boundaries.push((start, end));
            start = end;
        }

        boundaries
    }

    /// Check if context file exists
    pub async fn exists(&self) -> bool {
        fs::try_exists(&self.context_file).await.unwrap_or(false)
    }

    /// Delete the context file
    pub async fn clear(&self) -> Result<()> {
        if self.exists().await {
            fs::remove_file(&self.context_file)
                .await
                .context("Failed to remove context file")?;
        }
        Ok(())
    }

    /// Get the path to the context file
    pub fn context_path(&self) -> &PathBuf {
        &self.context_file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_store_and_read_context() {
        let temp_dir = TempDir::new().unwrap();
        let store = ContextStore::new(temp_dir.path().to_path_buf());

        let content = "Hello, World! This is a test context.";
        let metadata = store.store_context(content).await.unwrap();

        assert_eq!(metadata.length, content.len());
        assert!(metadata.chunk_count >= 1);

        let read_content = store.read_context().await.unwrap();
        assert_eq!(read_content, content);
    }

    #[tokio::test]
    async fn test_read_slice() {
        let temp_dir = TempDir::new().unwrap();
        let store = ContextStore::new(temp_dir.path().to_path_buf());

        let content = "0123456789ABCDEF";
        store.store_context(content).await.unwrap();

        let slice = store.read_slice(5, 10).await.unwrap();
        assert_eq!(slice, "56789");

        // Test bounds
        let slice = store.read_slice(0, 100).await.unwrap();
        assert_eq!(slice, content);

        let slice = store.read_slice(100, 200).await.unwrap();
        assert_eq!(slice, "");
    }

    #[tokio::test]
    async fn test_chunk_boundaries() {
        let temp_dir = TempDir::new().unwrap();
        let store = ContextStore::with_chunk_size(temp_dir.path().to_path_buf(), 10);

        let content = "a".repeat(25);
        let metadata = store.store_context(&content).await.unwrap();

        assert_eq!(metadata.chunk_count, 3);
        assert_eq!(metadata.chunk_boundaries, vec![(0, 10), (10, 20), (20, 25)]);
    }

    #[tokio::test]
    async fn test_empty_context() {
        let temp_dir = TempDir::new().unwrap();
        let store = ContextStore::new(temp_dir.path().to_path_buf());

        let metadata = store.store_context("").await.unwrap();
        assert_eq!(metadata.length, 0);
        assert_eq!(metadata.chunk_count, 0);
        assert!(metadata.chunk_boundaries.is_empty());
    }

    #[tokio::test]
    async fn test_clear_context() {
        let temp_dir = TempDir::new().unwrap();
        let store = ContextStore::new(temp_dir.path().to_path_buf());

        store.store_context("test content").await.unwrap();
        assert!(store.exists().await);

        store.clear().await.unwrap();
        assert!(!store.exists().await);
    }

    #[tokio::test]
    async fn test_utf8_handling() {
        let temp_dir = TempDir::new().unwrap();
        let store = ContextStore::new(temp_dir.path().to_path_buf());

        // Test with multi-byte UTF-8 characters
        let content = "Hello, 世界! 🎉";
        store.store_context(content).await.unwrap();

        let read_content = store.read_context().await.unwrap();
        assert_eq!(read_content, content);

        // Test slicing with UTF-8
        let slice = store.read_slice(0, 7).await.unwrap();
        assert_eq!(slice, "Hello, ");
    }
}
