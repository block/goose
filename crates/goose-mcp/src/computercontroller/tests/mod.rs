use super::*;
use std::path::PathBuf;
use serde_json::json;

#[tokio::test]
async fn test_pdf_text_extraction() {
    let router = ComputerControllerRouter::new();
    let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/computercontroller/tests/data/test.pdf");

    let result = router.pdf_tool(json!({
        "path": test_pdf_path.to_str().unwrap(),
        "operation": "extract_text"
    })).await;

    assert!(result.is_ok(), "PDF text extraction should succeed");
    let content = result.unwrap();
    assert!(!content.is_empty(), "Extracted text should not be empty");
    assert!(content[0].as_text().unwrap().contains("Page 1"), "Should contain page marker");
}

#[tokio::test]
async fn test_pdf_metadata() {
    let router = ComputerControllerRouter::new();
    let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/computercontroller/tests/data/test.pdf");

    let result = router.pdf_tool(json!({
        "path": test_pdf_path.to_str().unwrap(),
        "operation": "get_metadata"
    })).await;

    assert!(result.is_ok(), "PDF metadata extraction should succeed");
    let content = result.unwrap();
    assert!(!content.is_empty(), "Metadata should not be empty");
    assert!(content[0].as_text().unwrap().contains("PDF Metadata"), "Should contain metadata header");
}

#[tokio::test]
async fn test_pdf_invalid_path() {
    let router = ComputerControllerRouter::new();
    let result = router.pdf_tool(json!({
        "path": "nonexistent.pdf",
        "operation": "extract_text"
    })).await;

    assert!(result.is_err(), "Should fail with invalid path");
}

#[tokio::test]
async fn test_pdf_invalid_operation() {
    let router = ComputerControllerRouter::new();
    let test_pdf_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src/computercontroller/tests/data/test.pdf");

    let result = router.pdf_tool(json!({
        "path": test_pdf_path.to_str().unwrap(),
        "operation": "invalid_operation"
    })).await;

    assert!(result.is_err(), "Should fail with invalid operation");
}