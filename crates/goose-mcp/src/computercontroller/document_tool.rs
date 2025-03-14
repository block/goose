use extractous::Extractor;
use mcp_core::{Content, ToolError};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

// Threshold for large text files (0.22MB - about 1/18 of the 4,194,304 bytes limit)
const LARGE_TEXT_THRESHOLD: usize = (2 * 1024 * 1024) / 9; // ~0.22MB in bytes

pub async fn document_tool(
    path: &str,
    operation: &str,
    cache_dir: &Path,
) -> Result<Vec<Content>, ToolError> {
    match operation {
        "get_text" => {
            // Extract text from a local file (PDF, DOCX, XLSX, etc.)
            extract_text_from_file(path, cache_dir)
        }
        "get_text_url" => {
            // Extract text from a URL
            extract_text_from_url(path, cache_dir)
        }
        _ => Err(ToolError::InvalidParameters(format!(
            "Invalid operation: {}. Valid operations are: 'get_text', 'get_text_url'",
            operation
        ))),
    }
}

fn extract_text_from_file(path: &str, cache_dir: &Path) -> Result<Vec<Content>, ToolError> {
    // Use extractous library for text extraction
    let extractor = Extractor::new();

    // Extract text from the file
    let (text, metadata) = extractor.extract_file_to_string(path).map_err(|e| {
        ToolError::ExecutionError(format!("Failed to extract text from file: {}", e))
    })?;

    process_extracted_text(text, metadata, path, cache_dir)
}

fn extract_text_from_url(url: &str, cache_dir: &Path) -> Result<Vec<Content>, ToolError> {
    // Validate that the input is actually a URL
    if !url.starts_with("http://") && !url.starts_with("https://") {
        return Err(ToolError::InvalidParameters(format!(
            "Invalid URL: {}. URL must start with http:// or https://",
            url
        )));
    }

    // Use extractous library for text extraction
    let extractor = Extractor::new();

    // Handle URL extraction
    let (mut stream_reader, metadata) = extractor.extract_url(url).map_err(|e| {
        ToolError::ExecutionError(format!("Failed to extract text from URL: {}", e))
    })?;

    // Convert StreamReader to String
    let mut text = String::new();
    stream_reader
        .read_to_string(&mut text)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to read text from URL: {}", e)))?;

    process_extracted_text(text, metadata, url, cache_dir)
}

fn process_extracted_text(
    text: String,
    metadata: std::collections::HashMap<String, Vec<String>>,
    source_path: &str,
    cache_dir: &Path,
) -> Result<Vec<Content>, ToolError> {
    // Check if the extracted text is large
    let text_size = text.len();
    if text_size > LARGE_TEXT_THRESHOLD {
        // Create a directory for large text files if it doesn't exist
        let large_text_dir = cache_dir.join("large_document_texts");
        fs::create_dir_all(&large_text_dir).map_err(|e| {
            ToolError::ExecutionError(format!("Failed to create directory for large text: {}", e))
        })?;

        // Create a filename based on the original document name
        let doc_path = PathBuf::from(source_path);
        let doc_filename = doc_path
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("unnamed_document");

        let text_file_path = large_text_dir.join(format!("{}.txt", doc_filename));

        // Write the text to a file
        fs::write(&text_file_path, &text).map_err(|e| {
            ToolError::ExecutionError(format!("Failed to write large text to file: {}", e))
        })?;

        // Format size in human-readable form
        let size_str = if text_size < 1024 * 1024 {
            format!("{:.2} KB", text_size as f64 / 1024.0)
        } else {
            format!("{:.2} MB", text_size as f64 / (1024.0 * 1024.0))
        };

        Ok(vec![Content::text(format!(
            "Large text extracted from document ({})\n\n\
            The extracted text is too large to display directly.\n\
            Text has been written to: {}\n\n\
            You can search through this file using ripgrep:\n\
            rg 'search term' {}\n\n\
            Or view portions of it:\n\
            head -n 50 {}\n\
            tail -n 50 {}\n\
            less {}",
            size_str,
            text_file_path.display(),
            text_file_path.display(),
            text_file_path.display(),
            text_file_path.display(),
            text_file_path.display()
        ))])
    } else {
        // Include metadata information in the output
        let metadata_info = if metadata.is_empty() {
            "Document Metadata: None\n\n".to_string()
        } else {
            let mut formatted_metadata = String::from("Document Metadata:\n");

            // Format each metadata entry
            for (key, values) in &metadata {
                formatted_metadata.push_str(&format!("  {}: ", key));

                // Single value case
                if values.len() == 1 {
                    formatted_metadata.push_str(&format!("{}\n", values[0]));
                    continue;
                }

                // Multiple values case
                formatted_metadata.push_str("[\n");
                for value in values {
                    formatted_metadata.push_str(&format!("    {}\n", value));
                }
                formatted_metadata.push_str("  ]\n");
            }

            formatted_metadata.push('\n');
            formatted_metadata
        };

        Ok(vec![Content::text(format!(
            "{}Extracted text from document:\n\n{}",
            metadata_info, text
        ))])
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_docx_text_extraction() {
        let test_docx_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/sample.docx");
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!(
            "Testing text extraction from DOCX: {}",
            test_docx_path.display()
        );

        let result = document_tool(test_docx_path.to_str().unwrap(), "get_text", &cache_dir).await;

        assert!(result.is_ok(), "DOCX text extraction should succeed");
        let content = result.unwrap();
        assert!(!content.is_empty(), "Extracted text should not be empty");
        let text = content[0].as_text().unwrap();
        println!("Extracted text:\n{}", text);
        assert!(
            text.contains("Document Metadata") || !text.is_empty(),
            "Should contain metadata or at least some text content"
        );
    }

    #[tokio::test]
    async fn test_url_text_extraction() {
        // Skip this test if we're not online
        // This is a simple test URL that should be stable
        let test_url = "https://example.com";
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!("Testing text extraction from URL: {}", test_url);

        let result = document_tool(test_url, "get_text_url", &cache_dir).await;

        // If the test fails due to network issues, just skip it
        if let Err(err) = &result {
            if err.to_string().contains("network") || err.to_string().contains("connection") {
                println!("Skipping URL extraction test due to network issues");
                return;
            }
        }

        assert!(result.is_ok(), "URL text extraction should succeed");
        let content = result.unwrap();
        assert!(!content.is_empty(), "Extracted text should not be empty");
        let text = content[0].as_text().unwrap();
        println!("Extracted text from URL:\n{}", text);
        assert!(
            text.contains("Example Domain"),
            "Should contain expected content from example.com"
        );
    }

    #[tokio::test]
    async fn test_document_invalid_path() {
        let cache_dir = tempfile::tempdir().unwrap().into_path();
        let result = document_tool("nonexistent.pdf", "get_text", &cache_dir).await;

        assert!(result.is_err(), "Should fail with invalid path");
    }

    #[tokio::test]
    async fn test_document_invalid_operation() {
        let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/test.pdf");
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        let result = document_tool(
            test_pdf_path.to_str().unwrap(),
            "invalid_operation",
            &cache_dir,
        )
        .await;

        assert!(result.is_err(), "Should fail with invalid operation");
    }

    #[tokio::test]
    async fn test_url_with_get_text() {
        let test_url = "https://example.com";
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        let result = document_tool(test_url, "get_text", &cache_dir).await;

        // This should fail since URLs should use get_text_url
        assert!(result.is_err(), "Using get_text with URL should fail");
    }

    #[tokio::test]
    async fn test_file_with_get_text_url() {
        let test_docx_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/sample.docx");
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        let result =
            document_tool(test_docx_path.to_str().unwrap(), "get_text_url", &cache_dir).await;

        // This should fail since local files should use get_text
        assert!(
            result.is_err(),
            "Using get_text_url with local file should fail"
        );
    }
}
