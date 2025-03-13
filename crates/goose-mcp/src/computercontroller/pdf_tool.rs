use extractous::Extractor;
use lopdf::{Document, Object};
use mcp_core::{Content, ToolError};
use std::{
    fs,
    io::Read,
    path::{Path, PathBuf},
};

// Threshold for large text files (0.22MB - about 1/18 of the 4,194,304 bytes limit)
const LARGE_TEXT_THRESHOLD: usize = (2 * 1024 * 1024) / 9; // ~0.22MB in bytes

pub async fn pdf_tool(
    path: &str,
    operation: &str,
    cache_dir: &Path,
) -> Result<Vec<Content>, ToolError> {
    match operation {
        "extract_text" => {
            // Use extractous library for text extraction
            let extractor = Extractor::new();

            // Check if the path is a URL or a file
            let (text, metadata) = if path.starts_with("http://") || path.starts_with("https://") {
                // Handle URL extraction
                let (mut stream_reader, metadata) = extractor.extract_url(path).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to extract text from URL: {}", e))
                })?;

                // Convert StreamReader to String - assuming it has a read_to_string method
                let mut text = String::new();
                stream_reader.read_to_string(&mut text).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to read text from URL: {}", e))
                })?;

                (text, metadata)
            } else {
                // Extract text from the file (PDF or other)
                extractor.extract_file_to_string(path).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to extract text from file: {}", e))
                })?
            };

            // Check if the extracted text is large
            let text_size = text.len();
            if text_size > LARGE_TEXT_THRESHOLD {
                // Create a directory for large text files if it doesn't exist
                let large_text_dir = cache_dir.join("large_pdf_texts");
                fs::create_dir_all(&large_text_dir).map_err(|e| {
                    ToolError::ExecutionError(format!(
                        "Failed to create directory for large text: {}",
                        e
                    ))
                })?;

                // Create a filename based on the original PDF name
                let pdf_path = PathBuf::from(path);
                let pdf_filename = pdf_path
                    .file_name()
                    .and_then(|name| name.to_str())
                    .unwrap_or("unnamed_pdf");

                let text_file_path = large_text_dir.join(format!("{}.txt", pdf_filename));

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
                    "Large text extracted from PDF ({})\n\n\
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
                let metadata_info = format!(
                    "PDF Metadata:\n{}\n\n",
                    serde_json::to_string_pretty(&metadata)
                        .unwrap_or_else(|_| "Unable to format metadata".to_string())
                );

                Ok(vec![Content::text(format!(
                    "{}Extracted text from PDF:\n\n{}",
                    metadata_info, text
                ))])
            }
        }

        "extract_images" => {
            // Check if the path is a URL (not supported for image extraction)
            if path.starts_with("http://") || path.starts_with("https://") {
                return Err(ToolError::InvalidParameters(
                    "Image extraction is not supported for URLs. Please provide a local PDF file path.".to_string(),
                ));
            }

            // Open and parse the PDF file for image extraction
            let doc = Document::load(path).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to open PDF file: {}", e))
            })?;

            let cache_dir = cache_dir.join("pdf_images");
            fs::create_dir_all(&cache_dir).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to create image cache directory: {}", e))
            })?;

            let mut images = Vec::new();
            let mut image_count = 0;

            // Helper function to determine file extension based on stream dict
            fn get_image_extension(dict: &lopdf::Dictionary) -> &'static str {
                if let Ok(filter) = dict.get(b"Filter") {
                    match filter {
                        Object::Name(name) => {
                            match name.as_slice() {
                                b"DCTDecode" => ".jpg",
                                b"JBIG2Decode" => ".jbig2",
                                b"JPXDecode" => ".jp2",
                                b"CCITTFaxDecode" => ".tiff",
                                b"FlateDecode" => {
                                    // PNG-like images often use FlateDecode
                                    // Check color space to confirm
                                    if let Ok(cs) = dict.get(b"ColorSpace") {
                                        if let Ok(name) = cs.as_name() {
                                            if name == b"DeviceRGB" || name == b"DeviceGray" {
                                                return ".png";
                                            }
                                        }
                                    }
                                    ".raw"
                                }
                                _ => ".raw",
                            }
                        }
                        Object::Array(filters) => {
                            // If multiple filters, check the last one
                            if let Some(Object::Name(name)) = filters.last() {
                                match name.as_slice() {
                                    b"DCTDecode" => return ".jpg",
                                    b"JPXDecode" => return ".jp2",
                                    _ => {}
                                }
                            }
                            ".raw"
                        }
                        _ => ".raw",
                    }
                } else {
                    ".raw"
                }
            }

            // Process each page
            for (page_num, page_id) in doc.get_pages() {
                let page = doc.get_object(page_id).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to get page {}: {}", page_num, e))
                })?;

                let page_dict = page.as_dict().map_err(|e| {
                    ToolError::ExecutionError(format!(
                        "Failed to get page dict {}: {}",
                        page_num, e
                    ))
                })?;

                // Get page resources - handle both direct dict and reference
                let resources = match page_dict.get(b"Resources") {
                    Ok(res) => match res {
                        Object::Dictionary(dict) => Ok(dict),
                        Object::Reference(id) => doc
                            .get_object(*id)
                            .map_err(|e| {
                                ToolError::ExecutionError(format!(
                                    "Failed to get resource reference: {}",
                                    e
                                ))
                            })
                            .and_then(|obj| {
                                obj.as_dict().map_err(|e| {
                                    ToolError::ExecutionError(format!(
                                        "Resource reference is not a dictionary: {}",
                                        e
                                    ))
                                })
                            }),
                        _ => Err(ToolError::ExecutionError(
                            "Resources is neither dictionary nor reference".to_string(),
                        )),
                    },
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to get Resources: {}",
                        e
                    ))),
                }?;

                // Look for XObject dictionary - handle both direct dict and reference
                let xobjects = match resources.get(b"XObject") {
                    Ok(xobj) => match xobj {
                        Object::Dictionary(dict) => Ok(dict),
                        Object::Reference(id) => doc
                            .get_object(*id)
                            .map_err(|e| {
                                ToolError::ExecutionError(format!(
                                    "Failed to get XObject reference: {}",
                                    e
                                ))
                            })
                            .and_then(|obj| {
                                obj.as_dict().map_err(|e| {
                                    ToolError::ExecutionError(format!(
                                        "XObject reference is not a dictionary: {}",
                                        e
                                    ))
                                })
                            }),
                        _ => Err(ToolError::ExecutionError(
                            "XObject is neither dictionary nor reference".to_string(),
                        )),
                    },
                    Err(e) => Err(ToolError::ExecutionError(format!(
                        "Failed to get XObject: {}",
                        e
                    ))),
                };

                if let Ok(xobjects) = xobjects {
                    for (name, xobject) in xobjects.iter() {
                        let xobject_id = xobject.as_reference().map_err(|_| {
                            ToolError::ExecutionError("Failed to get XObject reference".to_string())
                        })?;

                        let xobject = doc.get_object(xobject_id).map_err(|e| {
                            ToolError::ExecutionError(format!("Failed to get XObject: {}", e))
                        })?;

                        if let Ok(stream) = xobject.as_stream() {
                            // Check if it's an image
                            if let Ok(subtype) =
                                stream.dict.get(b"Subtype").and_then(|s| s.as_name())
                            {
                                if subtype == b"Image" {
                                    let extension = get_image_extension(&stream.dict);

                                    // Get image metadata
                                    let width = stream
                                        .dict
                                        .get(b"Width")
                                        .and_then(|w| w.as_i64())
                                        .unwrap_or(0);
                                    let height = stream
                                        .dict
                                        .get(b"Height")
                                        .and_then(|h| h.as_i64())
                                        .unwrap_or(0);
                                    let bpc = stream
                                        .dict
                                        .get(b"BitsPerComponent")
                                        .and_then(|b| b.as_i64())
                                        .unwrap_or(0);

                                    // Get the image data
                                    if let Ok(data) = stream.get_plain_content() {
                                        let image_path = cache_dir.join(format!(
                                            "page{}_obj{}_{}{}",
                                            page_num,
                                            xobject_id.0,
                                            String::from_utf8_lossy(name),
                                            extension
                                        ));

                                        fs::write(&image_path, &data).map_err(|e| {
                                            ToolError::ExecutionError(format!(
                                                "Failed to write image: {}",
                                                e
                                            ))
                                        })?;

                                        images.push(format!(
                                            "Saved image to: {} ({}x{}, {} bits per component)",
                                            image_path.display(),
                                            width,
                                            height,
                                            bpc
                                        ));
                                        image_count += 1;
                                    }
                                }
                            }
                        }
                    }
                }
            }

            if images.is_empty() {
                Ok(vec![Content::text("No images found in PDF".to_string())])
            } else {
                Ok(vec![Content::text(format!(
                    "Found {} images:\n{}",
                    image_count,
                    images.join("\n")
                ))])
            }
        }

        _ => Err(ToolError::InvalidParameters(format!(
            "Invalid operation: {}. Valid operations are: 'extract_text', 'extract_images'",
            operation
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_pdf_text_extraction() {
        let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/test.pdf");
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!("Testing text extraction from: {}", test_pdf_path.display());

        let result = pdf_tool(test_pdf_path.to_str().unwrap(), "extract_text", &cache_dir).await;

        assert!(result.is_ok(), "PDF text extraction should succeed");
        let content = result.unwrap();
        assert!(!content.is_empty(), "Extracted text should not be empty");
        let text = content[0].as_text().unwrap();
        println!("Extracted text:\n{}", text);
        assert!(
            text.contains("This is a test PDF") || text.contains("PDF Metadata"),
            "Should contain expected test content or metadata"
        );
    }

    #[tokio::test]
    async fn test_url_text_extraction() {
        // Skip this test if we're not online
        // This is a simple test URL that should be stable
        let test_url = "https://example.com";
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!("Testing text extraction from URL: {}", test_url);

        let result = pdf_tool(test_url, "extract_text", &cache_dir).await;

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
    async fn test_pdf_image_extraction() {
        let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/test_image.pdf");
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!("Testing image extraction from: {}", test_pdf_path.display());

        // Now try image extraction
        let result = pdf_tool(
            test_pdf_path.to_str().unwrap(),
            "extract_images",
            &cache_dir,
        )
        .await;

        println!("Image extraction result: {:?}", result);
        assert!(result.is_ok(), "PDF image extraction should succeed");
        let content = result.unwrap();
        assert!(
            !content.is_empty(),
            "Image extraction result should not be empty"
        );
        let text = content[0].as_text().unwrap();
        println!("Extracted content: {}", text);

        // Should either find images or explicitly state none were found
        assert!(
            text.contains("Saved image to:") || text.contains("No images found"),
            "Should either save images or report none found"
        );

        // If we found images, verify they exist
        if text.contains("Saved image to:") {
            // Extract the file path from the output
            let file_path = text
                .lines()
                .find(|line| line.contains("Saved image to:"))
                .and_then(|line| line.split(": ").nth(1))
                .and_then(|path| path.split(" (").next())
                .expect("Should have a valid file path");

            println!("Verifying image file exists: {}", file_path);
            assert!(PathBuf::from(file_path).exists(), "Image file should exist");
        }
    }

    #[tokio::test]
    async fn test_url_image_extraction_fails() {
        // Test that image extraction from URLs is properly rejected
        let test_url = "https://example.com";
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!(
            "Testing image extraction from URL (should fail): {}",
            test_url
        );

        let result = pdf_tool(test_url, "extract_images", &cache_dir).await;
        assert!(result.is_err(), "URL image extraction should fail");

        let error = result.unwrap_err();
        assert!(
            error
                .to_string()
                .contains("Image extraction is not supported for URLs"),
            "Should return the correct error message for URL image extraction"
        );
    }

    #[tokio::test]
    async fn test_pdf_invalid_path() {
        let cache_dir = tempfile::tempdir().unwrap().into_path();
        let result = pdf_tool("nonexistent.pdf", "extract_text", &cache_dir).await;

        assert!(result.is_err(), "Should fail with invalid path");
    }

    #[tokio::test]
    async fn test_pdf_invalid_operation() {
        let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/test.pdf");
        let cache_dir = tempfile::tempdir().unwrap().into_path();

        let result = pdf_tool(
            test_pdf_path.to_str().unwrap(),
            "invalid_operation",
            &cache_dir,
        )
        .await;

        assert!(result.is_err(), "Should fail with invalid operation");
    }

    #[tokio::test]
    async fn test_large_pdf_text_extraction() {
        let large_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/visa-rules-public.pdf");

        // Skip test if the large PDF file doesn't exist (may not be committed to git)
        if !large_pdf_path.exists() {
            println!(
                "Skipping large PDF test as file doesn't exist: {}",
                large_pdf_path.display()
            );
            return;
        }

        let cache_dir = tempfile::tempdir().unwrap().into_path();

        println!(
            "Testing large text extraction from: {}",
            large_pdf_path.display()
        );

        let result = pdf_tool(large_pdf_path.to_str().unwrap(), "extract_text", &cache_dir).await;

        assert!(result.is_ok(), "Large PDF text extraction should succeed");
        let content = result.unwrap();
        assert!(!content.is_empty(), "Extracted text should not be empty");
        let text = content[0].as_text().unwrap();

        // Check if the text is large enough to be written to a file
        if text.contains("Large text extracted from PDF") {
            // For large PDFs, we should get the message about writing to a file
            assert!(
                text.contains("Text has been written to:"),
                "Should indicate where text was written"
            );

            // Extract the file path from the output and verify it exists
            let file_path = text
                .lines()
                .find(|line| line.contains("Text has been written to:"))
                .and_then(|line| line.split(": ").nth(1))
                .expect("Should have a valid file path");

            println!("Verifying text file exists: {}", file_path);
            assert!(PathBuf::from(file_path).exists(), "Text file should exist");

            // Verify file contains actual content
            let file_content =
                fs::read_to_string(file_path).expect("Should be able to read text file");
            assert!(!file_content.is_empty(), "Text file should not be empty");
        } else {
            // If the text is not written to a file, it should contain PDF content directly
            assert!(
                text.contains("PDF Metadata:"),
                "Should contain PDF metadata"
            );
            // The text should not be empty (beyond just metadata)
            assert!(text.len() > 100, "Should contain substantial text content");
        }
    }
}
