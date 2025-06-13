use anyhow::{Context, Result};
use arrow::array::{FixedSizeListBuilder, StringArray};
use arrow::datatypes::{DataType, Field, Schema};
use chrono::Local;
use etcetera::base_strategy::{BaseStrategy, Xdg};
use futures_util::stream::TryStreamExt;
use ignore::WalkBuilder;
use lancedb::connect;
use lancedb::connection::Connection;
use lancedb::query::{ExecutableQuery, QueryBase};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FileRecord {
    pub file_path: String,
    pub content: String,
    pub file_type: String,
    pub chunk_index: i32,
    pub vector: Vec<f32>,
}

pub struct FileVectorDB {
    connection: Arc<RwLock<Connection>>,
    table_name: String,
}

impl FileVectorDB {
    pub async fn new(table_name: Option<String>) -> Result<Self> {
        let db_path = Self::get_db_path()?;

        // Ensure the directory exists
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent)
                .await
                .context("Failed to create database directory")?;
        }

        let connection = connect(db_path.to_str().unwrap())
            .execute()
            .await
            .context("Failed to connect to LanceDB")?;

        let file_db = Self {
            connection: Arc::new(RwLock::new(connection)),
            table_name: table_name.unwrap_or_else(|| "files".to_string()),
        };

        // Initialize the table if it doesn't exist
        file_db.init_table().await?;

        Ok(file_db)
    }

    fn get_db_path() -> Result<PathBuf> {
        let data_dir = Xdg::new()
            .context("Failed to determine base strategy")?
            .data_dir();

        Ok(data_dir.join("goose").join("file_db"))
    }

    async fn init_table(&self) -> Result<()> {
        let connection = self.connection.read().await;

        // Check if table exists
        let table_names = connection
            .table_names()
            .execute()
            .await
            .context("Failed to list tables")?;

        if !table_names.contains(&self.table_name) {
            // Create the table schema
            let schema = Arc::new(Schema::new(vec![
                Field::new("file_path", DataType::Utf8, false),
                Field::new("content", DataType::Utf8, false),
                Field::new("file_type", DataType::Utf8, false),
                Field::new("chunk_index", DataType::Int32, false),
                Field::new(
                    "vector",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        1536, // OpenAI embedding dimension
                    ),
                    false,
                ),
            ]));

            // Create empty table
            let file_paths = StringArray::from(vec![] as Vec<&str>);
            let contents = StringArray::from(vec![] as Vec<&str>);
            let file_types = StringArray::from(vec![] as Vec<&str>);
            let chunk_indices = arrow::array::Int32Array::from(vec![] as Vec<i32>);

            // Create empty fixed size list array for vectors
            let mut vectors_builder =
                FixedSizeListBuilder::new(arrow::array::Float32Builder::new(), 1536);
            let vectors = vectors_builder.finish();

            let batch = arrow::record_batch::RecordBatch::try_new(
                schema.clone(),
                vec![
                    Arc::new(file_paths),
                    Arc::new(contents),
                    Arc::new(file_types),
                    Arc::new(chunk_indices),
                    Arc::new(vectors),
                ],
            )
            .context("Failed to create record batch")?;

            drop(connection);
            let connection = self.connection.write().await;

            // Use the RecordBatch directly
            let reader = arrow::record_batch::RecordBatchIterator::new(
                vec![Ok(batch)].into_iter(),
                schema.clone(),
            );

            connection
                .create_table(&self.table_name, Box::new(reader))
                .execute()
                .await
                .map_err(|e| {
                    anyhow::anyhow!("Failed to create files table '{}': {}", self.table_name, e)
                })?;
        }

        Ok(())
    }

    pub async fn index_files(&self, files: Vec<FileRecord>) -> Result<()> {
        if files.is_empty() {
            return Ok(());
        }

        let file_paths: Vec<&str> = files.iter().map(|f| f.file_path.as_str()).collect();
        let contents: Vec<&str> = files.iter().map(|f| f.content.as_str()).collect();
        let file_types: Vec<&str> = files.iter().map(|f| f.file_type.as_str()).collect();
        let chunk_indices: Vec<i32> = files.iter().map(|f| f.chunk_index).collect();

        let vectors_data: Vec<Option<Vec<Option<f32>>>> = files
            .iter()
            .map(|f| Some(f.vector.iter().map(|&v| Some(v)).collect()))
            .collect();

        let schema = Arc::new(Schema::new(vec![
            Field::new("file_path", DataType::Utf8, false),
            Field::new("content", DataType::Utf8, false),
            Field::new("file_type", DataType::Utf8, false),
            Field::new("chunk_index", DataType::Int32, false),
            Field::new(
                "vector",
                DataType::FixedSizeList(
                    Arc::new(Field::new("item", DataType::Float32, true)),
                    1536,
                ),
                false,
            ),
        ]));

        let file_paths_array = StringArray::from(file_paths);
        let contents_array = StringArray::from(contents);
        let file_types_array = StringArray::from(file_types);
        let chunk_indices_array = arrow::array::Int32Array::from(chunk_indices);

        // Build vectors array
        let mut vectors_builder =
            FixedSizeListBuilder::new(arrow::array::Float32Builder::new(), 1536);
        for vector_opt in vectors_data {
            if let Some(vector) = vector_opt {
                let values = vectors_builder.values();
                for val_opt in vector {
                    if let Some(val) = val_opt {
                        values.append_value(val);
                    } else {
                        values.append_null();
                    }
                }
                vectors_builder.append(true);
            } else {
                vectors_builder.append(false);
            }
        }
        let vectors_array = vectors_builder.finish();

        let batch = arrow::record_batch::RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(file_paths_array),
                Arc::new(contents_array),
                Arc::new(file_types_array),
                Arc::new(chunk_indices_array),
                Arc::new(vectors_array),
            ],
        )
        .context("Failed to create record batch")?;

        let connection = self.connection.read().await;
        let table = connection
            .open_table(&self.table_name)
            .execute()
            .await
            .context("Failed to open files table")?;

        // Add batch to table using RecordBatchIterator
        let reader = arrow::record_batch::RecordBatchIterator::new(
            vec![Ok(batch)].into_iter(),
            schema.clone(),
        );

        table
            .add(Box::new(reader))
            .execute()
            .await
            .context("Failed to add files to table")?;

        Ok(())
    }

    pub async fn search_files(&self, query_vector: Vec<f32>, k: usize) -> Result<Vec<FileRecord>> {
        let connection = self.connection.read().await;

        let table = connection
            .open_table(&self.table_name)
            .execute()
            .await
            .context("Failed to open files table")?;

        let results = table
            .vector_search(query_vector)
            .context("Failed to create vector search")?
            .limit(k)
            .execute()
            .await
            .context("Failed to execute vector search")?;

        let batches: Vec<_> = results.try_collect().await?;

        let mut files = Vec::new();
        for batch in batches {
            let file_paths = batch
                .column_by_name("file_path")
                .context("Missing file_path column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid file_path column type")?;

            let contents = batch
                .column_by_name("content")
                .context("Missing content column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid content column type")?;

            let file_types = batch
                .column_by_name("file_type")
                .context("Missing file_type column")?
                .as_any()
                .downcast_ref::<StringArray>()
                .context("Invalid file_type column type")?;

            let chunk_indices = batch
                .column_by_name("chunk_index")
                .context("Missing chunk_index column")?
                .as_any()
                .downcast_ref::<arrow::array::Int32Array>()
                .context("Invalid chunk_index column type")?;

            for i in 0..batch.num_rows() {
                files.push(FileRecord {
                    file_path: file_paths.value(i).to_string(),
                    content: contents.value(i).to_string(),
                    file_type: file_types.value(i).to_string(),
                    chunk_index: chunk_indices.value(i),
                    vector: vec![], // We don't need to return the vector
                });
            }
        }
        Ok(files)
    }

    pub async fn clear_files(&self) -> Result<()> {
        let connection = self.connection.write().await;

        // Try to open the table first
        match connection.open_table(&self.table_name).execute().await {
            Ok(table) => {
                // Delete all records instead of dropping the table
                table
                    .delete("1=1") // This will match all records
                    .await
                    .context("Failed to delete all records")?;
            }
            Err(_) => {
                // If table doesn't exist, that's fine - we'll create it
            }
        }

        drop(connection);

        // Ensure table exists with correct schema
        self.init_table().await?;

        Ok(())
    }
}

pub fn generate_file_table_id() -> String {
    Local::now().format("%Y%m%d_%H%M%S").to_string()
}

/// Extract content from files in a directory, respecting gitignore patterns
pub fn extract_file_contents(
    directory: &Path,
    max_files: Option<usize>,
) -> Result<Vec<(PathBuf, String, String)>> {
    let mut files = Vec::new();
    let mut file_count = 0;
    let max_count = max_files.unwrap_or(1000); // Default limit

    let walker = WalkBuilder::new(directory)
        .hidden(false) // Include hidden files
        .git_ignore(true) // Respect .gitignore
        .git_global(true) // Respect global git ignore
        .git_exclude(true) // Respect .git/info/exclude
        .build();

    for result in walker {
        if file_count >= max_count {
            break;
        }

        let entry = result?;
        let path = entry.path();

        // Skip directories
        if !path.is_file() {
            continue;
        }

        // Get file extension for type detection
        let file_type = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("unknown")
            .to_string();

        // Skip binary files and very large files
        if is_likely_binary(&file_type) {
            continue;
        }

        // Read file content
        match std::fs::read_to_string(path) {
            Ok(content) => {
                // Skip very large files (>100KB)
                if content.len() > 100_000 {
                    continue;
                }

                files.push((path.to_path_buf(), content, file_type));
                file_count += 1;
            }
            Err(_) => {
                // Skip files that can't be read as text
                continue;
            }
        }
    }

    Ok(files)
}

/// Check if a file type is likely binary
fn is_likely_binary(file_type: &str) -> bool {
    matches!(
        file_type.to_lowercase().as_str(),
        "exe"
            | "dll"
            | "so"
            | "dylib"
            | "bin"
            | "jpg"
            | "jpeg"
            | "png"
            | "gif"
            | "bmp"
            | "ico"
            | "pdf"
            | "zip"
            | "tar"
            | "gz"
            | "7z"
            | "rar"
            | "mp3"
            | "mp4"
            | "avi"
            | "mov"
            | "wmv"
            | "flv"
            | "wav"
            | "ogg"
            | "woff"
            | "woff2"
            | "ttf"
            | "otf"
            | "eot"
    )
}

/// Chunk large text content into smaller pieces
pub fn chunk_content(content: &str, max_chunk_size: usize) -> Vec<String> {
    if content.len() <= max_chunk_size {
        return vec![content.to_string()];
    }

    let mut chunks = Vec::new();
    let lines: Vec<&str> = content.lines().collect();
    let mut current_chunk = String::new();

    for line in lines {
        // If adding this line would exceed the chunk size, start a new chunk
        if !current_chunk.is_empty() && current_chunk.len() + line.len() + 1 > max_chunk_size {
            chunks.push(current_chunk.trim().to_string());
            current_chunk = String::new();
        }

        if !current_chunk.is_empty() {
            current_chunk.push('\n');
        }
        current_chunk.push_str(line);
    }

    if !current_chunk.is_empty() {
        chunks.push(current_chunk.trim().to_string());
    }

    chunks
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[tokio::test]
    async fn test_file_vectordb_creation() {
        let db = FileVectorDB::new(Some("test_files_vectordb_creation".to_string()))
            .await
            .unwrap();
        db.clear_files().await.unwrap();
        assert_eq!(db.table_name, "test_files_vectordb_creation");
    }

    #[tokio::test]
    async fn test_file_vectordb_operations() -> Result<()> {
        // Create a new database instance with a unique table name
        let db = FileVectorDB::new(Some("test_file_vectordb_operations".to_string())).await?;

        // Clear any existing files
        db.clear_files().await?;

        // Create test file records
        let test_files = vec![
            FileRecord {
                file_path: "test1.rs".to_string(),
                content: "fn main() { println!(\"Hello, world!\"); }".to_string(),
                file_type: "rs".to_string(),
                chunk_index: 0,
                vector: vec![0.1; 1536], // Mock embedding vector
            },
            FileRecord {
                file_path: "test2.py".to_string(),
                content: "print('Hello, Python!')".to_string(),
                file_type: "py".to_string(),
                chunk_index: 0,
                vector: vec![0.2; 1536], // Different mock embedding vector
            },
        ];

        // Index the test files
        db.index_files(test_files).await?;

        // Search for files using a query vector similar to test1.rs
        let query_vector = vec![0.1; 1536];
        let results = db.search_files(query_vector, 2).await?;

        // Verify results
        assert_eq!(results.len(), 2, "Should find both files");
        assert_eq!(
            results[0].file_path, "test1.rs",
            "First result should be test1.rs"
        );
        assert_eq!(
            results[1].file_path, "test2.py",
            "Second result should be test2.py"
        );

        Ok(())
    }

    #[test]
    fn test_extract_file_contents() {
        let temp_dir = TempDir::new().unwrap();

        // Create some test files
        std::fs::write(temp_dir.path().join("test.txt"), "Hello, world!").unwrap();
        std::fs::write(temp_dir.path().join("test.rs"), "fn main() {}").unwrap();

        let files = extract_file_contents(temp_dir.path(), Some(10)).unwrap();

        assert_eq!(files.len(), 2);
        assert!(files
            .iter()
            .any(|(path, _, _)| path.file_name().unwrap() == "test.txt"));
        assert!(files
            .iter()
            .any(|(path, _, _)| path.file_name().unwrap() == "test.rs"));
    }

    #[test]
    fn test_chunk_content() {
        let content = "Line 1\nLine 2\nLine 3\nLine 4\nLine 5";
        let chunks = chunk_content(content, 15); // Small chunk size to force splitting

        assert!(
            chunks.len() > 1,
            "Content should be split into multiple chunks"
        );

        // Test content that fits in one chunk
        let small_content = "Small content";
        let small_chunks = chunk_content(small_content, 100);
        assert_eq!(small_chunks.len(), 1);
        assert_eq!(small_chunks[0], small_content);
    }

    #[test]
    fn test_is_likely_binary() {
        assert!(is_likely_binary("exe"));
        assert!(is_likely_binary("jpg"));
        assert!(is_likely_binary("pdf"));
        assert!(!is_likely_binary("txt"));
        assert!(!is_likely_binary("rs"));
        assert!(!is_likely_binary("py"));
    }
}
