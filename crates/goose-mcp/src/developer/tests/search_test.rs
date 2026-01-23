use crate::developer::rmcp_developer::SearchParams;
use crate::DeveloperServer;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::CallToolResult;

fn extract_text(result: &CallToolResult) -> String {
    result
        .content
        .iter()
        .filter_map(|c| c.raw.as_text().map(|t| t.text.clone()))
        .collect::<Vec<_>>()
        .join("\n")
}

async fn run_search(query: &str) -> CallToolResult {
    let server = DeveloperServer::new();
    server
        .search(Parameters(SearchParams {
            query: query.to_string(),
        }))
        .await
        .unwrap()
}

#[tokio::test]
#[ignore]
async fn test_search_returns_results_from_all_sources() {
    let text = extract_text(&run_search("rust tokio spawn").await);
    assert!(text.contains("[code]") || text.contains("[issue]") || text.contains("[reddit]"));
    assert!(text.contains("fetch:"));
}

#[tokio::test]
#[ignore]
async fn test_search_handles_empty_results_gracefully() {
    let text = extract_text(&run_search("xyzzy123nonexistent456gibberish").await);
    assert!(
        text.contains("No results found") || text.contains("Search unavailable"),
        "Should handle no results gracefully"
    );
}
