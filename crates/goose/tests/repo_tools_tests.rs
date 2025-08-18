//! Integration tests for repo__build_index, repo__search, repo__stats tools.
//! These exercise the handlers directly (not full agent loop) to keep scope small.

#[cfg(feature = "repo-index")]
mod repo_tool_tests {
    use serde_json::json;
    use goose::agents::repo_tools;

    #[tokio::test]
    async fn build_then_stats_then_search() {
        // Use repository root (current crate workspace) as test root.
        // In CI this should be fine; build will skip if already cached.
        let root = std::env::current_dir().unwrap();
        let root_str = root.to_string_lossy().to_string();

        // Build index
    let build_res = repo_tools::handle_repo_build(json!({
            "root": root_str,
            "force": true,
            "langs": ["rust"]
        })).await.expect("build index");
        assert_eq!(build_res["status"], "built", "expected built status");
        assert!(build_res["files_indexed"].as_u64().unwrap() > 0, "should index some files");

        // Stats
    let stats = repo_tools::handle_repo_stats(json!({"root": build_res["root"].clone()})).await.expect("stats");
        assert!(stats["files"].as_u64().unwrap() > 0, "files count");
        assert!(stats["entities"].as_u64().unwrap() > 0, "entities count");

        // Simple search for a known symbol (RepoIndexService itself)
    let search = repo_tools::handle_repo_query(json!({
            "root": build_res["root"].clone(),
            "query": "RepoIndexService",
            "limit": 5,
            "show_score": true
        })).await.expect("search result");
        let results = search["results"].as_array().unwrap();
        assert!(!results.is_empty(), "should find at least one symbol");
    }
}
