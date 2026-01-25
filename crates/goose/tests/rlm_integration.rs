//! Integration tests for RLM (Recursive Language Models) functionality
//!
//! These tests verify the RLM extension tools and auto-detection work correctly.

use std::sync::Arc;

use goose::agents::extension::ExtensionConfig;
use goose::agents::rlm_extension::EXTENSION_NAME as RLM_EXTENSION_NAME;
use goose::agents::{Agent, AgentConfig};
use goose::config::permission::PermissionManager;
use goose::config::GooseMode;
use goose::rlm::context_store::ContextStore;
use goose::rlm::test_utils::{generate_multi_document_context, generate_needle_haystack};
use goose::rlm::{is_rlm_candidate, RlmConfig};
use goose::session::SessionManager;
use tempfile::TempDir;

/// Helper to set up an agent with RLM extension enabled
async fn setup_agent_with_rlm() -> (Agent, String, TempDir) {
    let temp_dir = TempDir::new().unwrap();
    let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
    let permission_manager = Arc::new(PermissionManager::new(temp_dir.path().to_path_buf()));
    let config = AgentConfig::new(session_manager, permission_manager, None, GooseMode::Auto);

    let agent = Agent::with_config(config);

    // Add the RLM extension
    let rlm_config = ExtensionConfig::Platform {
        name: RLM_EXTENSION_NAME.to_string(),
        description: "RLM tools for processing large contexts".to_string(),
        bundled: Some(true),
        available_tools: vec![],
    };

    agent
        .add_extension(rlm_config)
        .await
        .expect("Failed to add RLM extension");

    let session_id = "test-rlm-session".to_string();
    (agent, session_id, temp_dir)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_rlm_extension_tools_available() {
        let (agent, session_id, _temp_dir) = setup_agent_with_rlm().await;
        let tools = agent.list_tools(&session_id, None).await;

        // Check that all RLM tools are available
        let rlm_tool_names = [
            "rlm_get_context_metadata",
            "rlm_read_context_slice",
            "rlm_query",
            "rlm_store_variable",
            "rlm_get_variable",
            "rlm_list_variables",
            "rlm_finalize",
        ];

        for tool_name in rlm_tool_names {
            let full_name = format!("rlm__{}", tool_name);
            let tool = tools.iter().find(|t| &*t.name == full_name);
            assert!(
                tool.is_some(),
                "RLM tool '{}' should be available",
                full_name
            );
        }
    }

    #[tokio::test]
    async fn test_rlm_extension_tool_schemas() {
        let (agent, session_id, _temp_dir) = setup_agent_with_rlm().await;
        let tools = agent.list_tools(&session_id, None).await;

        // Check rlm_read_context_slice has start/end parameters
        let slice_tool = tools
            .iter()
            .find(|t| &*t.name == "rlm__rlm_read_context_slice")
            .expect("rlm_read_context_slice should exist");

        let props = slice_tool
            .input_schema
            .get("properties")
            .expect("Should have properties");
        assert!(props.get("start").is_some(), "Should have 'start' param");
        assert!(props.get("end").is_some(), "Should have 'end' param");

        // Check rlm_query has prompt, start, end parameters
        let query_tool = tools
            .iter()
            .find(|t| &*t.name == "rlm__rlm_query")
            .expect("rlm_query should exist");

        let props = query_tool
            .input_schema
            .get("properties")
            .expect("Should have properties");
        assert!(props.get("prompt").is_some(), "Should have 'prompt' param");
        assert!(props.get("start").is_some(), "Should have 'start' param");
        assert!(props.get("end").is_some(), "Should have 'end' param");

        // Check rlm_finalize has answer parameter
        let finalize_tool = tools
            .iter()
            .find(|t| &*t.name == "rlm__rlm_finalize")
            .expect("rlm_finalize should exist");

        let props = finalize_tool
            .input_schema
            .get("properties")
            .expect("Should have properties");
        assert!(props.get("answer").is_some(), "Should have 'answer' param");
    }

    #[test]
    fn test_rlm_candidate_detection() {
        let config = RlmConfig::default();

        // Small content should not trigger RLM
        let small = "a".repeat(50_000);
        assert!(
            !is_rlm_candidate(&small, &config),
            "50K chars should not trigger RLM"
        );

        // Large content should trigger RLM
        let large = "a".repeat(150_000);
        assert!(
            is_rlm_candidate(&large, &config),
            "150K chars should trigger RLM"
        );

        // Disabled config should never trigger
        let disabled = RlmConfig {
            enabled: false,
            ..Default::default()
        };
        assert!(
            !is_rlm_candidate(&large, &disabled),
            "Disabled RLM should not trigger"
        );
    }

    #[test]
    fn test_custom_threshold() {
        let config = RlmConfig {
            enabled: true,
            context_threshold: 10_000,
            ..Default::default()
        };

        let small = "a".repeat(5_000);
        let large = "a".repeat(15_000);

        assert!(!is_rlm_candidate(&small, &config));
        assert!(is_rlm_candidate(&large, &config));
    }

    #[tokio::test]
    async fn test_context_store_needle_in_haystack() {
        let temp_dir = TempDir::new().unwrap();
        let store = ContextStore::new(temp_dir.path().to_path_buf());

        // Generate a large context with a needle
        let needle = "SECRET_CODE_XYZ123";
        let context = generate_needle_haystack(500_000, needle, 0.6);

        // Store the context
        let metadata = store.store_context(&context).await.unwrap();
        assert!(metadata.length >= 500_000);
        assert!(metadata.chunk_count > 0);

        // Find the needle position
        let needle_pos = context.find(needle).expect("Needle should be in context");

        // Read a slice that contains the needle
        let slice_start = needle_pos.saturating_sub(1000);
        let slice_end = (needle_pos + needle.len() + 1000).min(context.len());

        let slice = store.read_slice(slice_start, slice_end).await.unwrap();
        assert!(
            slice.contains(needle),
            "Slice should contain the needle '{}'",
            needle
        );
    }

    #[tokio::test]
    async fn test_context_store_chunk_boundaries() {
        let temp_dir = TempDir::new().unwrap();
        let chunk_size = 100_000;
        let store = ContextStore::with_chunk_size(temp_dir.path().to_path_buf(), chunk_size);

        // Create context spanning multiple chunks
        let context = "x".repeat(350_000);
        let metadata = store.store_context(&context).await.unwrap();

        assert_eq!(metadata.chunk_count, 4);
        assert_eq!(metadata.chunk_boundaries[0], (0, 100_000));
        assert_eq!(metadata.chunk_boundaries[1], (100_000, 200_000));
        assert_eq!(metadata.chunk_boundaries[2], (200_000, 300_000));
        assert_eq!(metadata.chunk_boundaries[3], (300_000, 350_000));
    }

    #[tokio::test]
    async fn test_context_store_multi_document() {
        let temp_dir = TempDir::new().unwrap();
        let store = ContextStore::new(temp_dir.path().to_path_buf());

        // Generate multi-document context
        let (context, facts) = generate_multi_document_context(20, 5000);

        // Store and verify
        let metadata = store.store_context(&context).await.unwrap();
        assert!(metadata.length > 0);

        // Read back the full context
        let read_back = store.read_context().await.unwrap();
        assert_eq!(read_back, context);

        // Verify all facts are present
        for (key, value) in &facts {
            assert!(
                read_back.contains(key),
                "Context should contain key '{}'",
                key
            );
            assert!(
                read_back.contains(value),
                "Context should contain value '{}'",
                value
            );
        }
    }

    #[tokio::test]
    async fn test_rlm_extension_not_loaded_by_default() {
        let temp_dir = TempDir::new().unwrap();
        let session_manager = Arc::new(SessionManager::new(temp_dir.path().to_path_buf()));
        let permission_manager = Arc::new(PermissionManager::new(temp_dir.path().to_path_buf()));
        let config = AgentConfig::new(session_manager, permission_manager, None, GooseMode::Auto);

        let agent = Agent::with_config(config);
        let session_id = "test-session";

        // Without explicitly adding RLM, the tools should not be available
        let tools = agent.list_tools(session_id, None).await;
        let rlm_tools: Vec<_> = tools
            .iter()
            .filter(|t| t.name.to_string().starts_with("rlm__"))
            .collect();

        assert!(
            rlm_tools.is_empty(),
            "RLM tools should not be loaded by default"
        );
    }

    #[test]
    fn test_rlm_config_defaults() {
        let config = RlmConfig::default();

        assert!(config.enabled);
        assert_eq!(config.context_threshold, 100_000);
        assert_eq!(config.chunk_size, 500_000);
        assert_eq!(config.max_iterations, 50);
        assert_eq!(config.max_recursion_depth, 1);
    }
}
