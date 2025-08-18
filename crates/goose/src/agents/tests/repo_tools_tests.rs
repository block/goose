#![cfg(feature = "repo-index")]

use crate::agents::repo_tools;
use serde_json::json;

#[tokio::test]
async fn test_repo_build_cached_flow() {
    let root = std::env::current_dir().unwrap();
    let first = repo_tools::handle_repo_build(json!({
        "root": root,
        "langs": ["rust"],
        "force": true
    })).await.expect("build ok");
    assert_eq!(first["status"], "built");

    let second = repo_tools::handle_repo_build(json!({
        "root": root,
        "langs": ["rust"],
        "force": false
    })).await.expect("cached ok");
    assert_eq!(second["status"], "cached");
}

#[tokio::test]
async fn test_repo_search_and_stats() {
    let root = std::env::current_dir().unwrap();
    // Build (force) to ensure fresh index
    repo_tools::handle_repo_build(json!({
        "root": root,
        "langs": ["rust"],
        "force": true
    })).await.unwrap();

    let res = repo_tools::handle_repo_query(json!({
        "root": root,
        "query": "Agent",
        "limit": 5,
        "show_score": true
    })).await.unwrap();
    assert!(res["results"].as_array().unwrap().len() > 0, "Expected some results for Agent");

    let stats = repo_tools::handle_repo_stats(json!({
        "root": root
    })).await.unwrap();
    assert!(stats["files"].as_u64().unwrap() > 0);
    assert!(stats["entities"].as_u64().unwrap() > 0);
}
