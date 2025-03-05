use docx_rs::*;
use mcp_core::{Content, ToolError};
use std::{fs, io::Cursor};

pub async fn docx_tool(
    path: &str,
    operation: &str,
    content: Option<&str>,
) -> Result<Vec<Content>, ToolError> {
    match operation {
        "extract_text" => {
            let file = fs::read(path)
                .map_err(|e| ToolError::ExecutionError(format!("Failed to read DOCX file: {}", e)))?;
            
            let docx = read_docx(&file)
                .map_err(|e| ToolError::ExecutionError(format!("Failed to parse DOCX file: {}", e)))?;

            let mut text = String::new();
            let mut structure = Vec::new();
            let mut current_level = None;

            // Extract document structure and text
            for element in &docx.document.children {
                if let DocumentChild::Paragraph(p) = element {
                    // Check for heading style
                    if let Some(style) = &p.property.style {
                        if style.val.starts_with("Heading") {
                            current_level = Some(style.val.clone());
                            structure.push(format!("{}: ", style.val));
                        }
                    }

                    // Extract text from runs
                    let para_text: String = p.children
                        .iter()
                        .filter_map(|r| {
                            if let ParagraphChild::Run(run) = r {
                                Some(run.children.iter().filter_map(|rc| {
                                    if let RunChild::Text(t) = rc {
                                        Some(t.text.clone())
                                    } else {
                                        None
                                    }
                                }).collect::<Vec<_>>().join(""))
                            } else {
                                None
                            }
                        })
                        .collect::<Vec<_>>()
                        .join("");

                    if !para_text.trim().is_empty() {
                        if let Some(_) = current_level {
                            structure.last_mut().map(|s| s.push_str(&para_text));
                            current_level = None;
                        }
                        text.push_str(&para_text);
                        text.push('\n');
                    }
                }
            }

            let result = if !structure.is_empty() {
                format!(
                    "Document Structure:\n{}\n\nFull Text:\n{}",
                    structure.join("\n"),
                    text
                )
            } else {
                format!("Extracted Text:\n{}", text)
            };

            Ok(vec![Content::text(result)])
        }

        "update_doc" => {
            let content = content.ok_or_else(|| {
                ToolError::InvalidParameters("Content parameter required for update_doc".to_string())
            })?;

            let mut doc = Docx::new();
            
            // Split content into paragraphs and add them
            for para in content.split('\n') {
                if !para.trim().is_empty() {
                    doc = doc.add_paragraph(
                        Paragraph::new().add_run(
                            Run::new().add_text(para)
                        )
                    );
                }
            }

            let mut buf = Vec::new();
            {
                let mut cursor = Cursor::new(&mut buf);
                doc.build().pack(&mut cursor)
                    .map_err(|e| ToolError::ExecutionError(format!("Failed to build DOCX: {}", e)))?;
            }

            fs::write(path, &buf)
                .map_err(|e| ToolError::ExecutionError(format!("Failed to write DOCX file: {}", e)))?;

            Ok(vec![Content::text(format!("Successfully wrote content to {}", path))])
        }

        _ => Err(ToolError::InvalidParameters(format!(
            "Invalid operation: {}. Valid operations are: 'extract_text', 'update_doc'",
            operation
        )))
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

        println!("Testing text extraction from: {}", test_docx_path.display());

        let result = docx_tool(test_docx_path.to_str().unwrap(), "extract_text", None).await;

        assert!(result.is_ok(), "DOCX text extraction should succeed");
        let content = result.unwrap();
        assert!(!content.is_empty(), "Extracted text should not be empty");
        let text = content[0].as_text().unwrap();
        println!("Extracted text:\n{}", text);
        assert!(!text.trim().is_empty(), "Extracted text should not be empty");
    }

    #[tokio::test]
    async fn test_docx_update() {
        let test_output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/test_output.docx");
        
        let test_content = "Test Heading\nThis is a test paragraph.\n\nAnother paragraph with some content.";

        let result = docx_tool(
            test_output_path.to_str().unwrap(),
            "update_doc",
            Some(test_content),
        )
        .await;

        assert!(result.is_ok(), "DOCX update should succeed");
        assert!(test_output_path.exists(), "Output file should exist");

        // Now try to read it back
        let result = docx_tool(test_output_path.to_str().unwrap(), "extract_text", None).await;
        assert!(result.is_ok(), "Should be able to read back the written file");
        let content = result.unwrap();
        let text = content[0].as_text().unwrap();
        assert!(text.contains("Test Heading"), "Should contain written content");
        assert!(text.contains("test paragraph"), "Should contain written content");

        // Clean up
        fs::remove_file(test_output_path).unwrap();
    }

    #[tokio::test]
    async fn test_docx_invalid_path() {
        let result = docx_tool("nonexistent.docx", "extract_text", None).await;
        assert!(result.is_err(), "Should fail with invalid path");
    }

    #[tokio::test]
    async fn test_docx_invalid_operation() {
        let test_docx_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/sample.docx");

        let result = docx_tool(
            test_docx_path.to_str().unwrap(),
            "invalid_operation",
            None,
        )
        .await;

        assert!(result.is_err(), "Should fail with invalid operation");
    }

    #[tokio::test]
    async fn test_docx_update_without_content() {
        let test_output_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("src/computercontroller/tests/data/test_output.docx");

        let result = docx_tool(
            test_output_path.to_str().unwrap(),
            "update_doc",
            None,
        )
        .await;

        assert!(result.is_err(), "Should fail without content");
    }
}