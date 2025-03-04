use goose_mcp::computercontroller::ComputerControllerRouter;
use mcp_server::Router;
use serde_json::json;

#[tokio::test]
async fn test_pdf_metadata() {
    let router = ComputerControllerRouter::new();
    
    // Test metadata extraction
    let result = router
        .call_tool(
            "pdf_tool",
            json!({
                "path": "tests/data/test.pdf",
                "operation": "get_metadata"
            }),
        )
        .await;

    assert!(result.is_ok());
    let content = result.unwrap();
    assert!(!content.is_empty());
    
    // Print the content for manual verification
    println!("Metadata content: {:?}", content);
}

#[tokio::test]
async fn test_pdf_text_extraction() {
    let router = ComputerControllerRouter::new();
    
    // Test text extraction
    let result = router
        .call_tool(
            "pdf_tool",
            json!({
                "path": "tests/data/test.pdf",
                "operation": "extract_text"
            }),
        )
        .await;

    assert!(result.is_ok());
    let content = result.unwrap();
    assert!(!content.is_empty());
    
    // Print the content for manual verification
    println!("Extracted text: {:?}", content);
}