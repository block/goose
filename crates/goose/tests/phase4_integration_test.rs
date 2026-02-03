//! Phase 4 Integration Tests: Advanced Agent Capabilities
//!
//! Tests ExecutionMode, StateGraph enhancements, PlanManager, and CriticManager

use anyhow::Result;
use goose::agents::state_graph::{ProjectType, StateGraph, StateGraphConfig};
use goose::agents::{
    AggregatedCritique, CriticManager, CritiqueContext, ExecutionMode, PlanManager,
};
use std::path::PathBuf;

/// Test ExecutionMode enum functionality
#[tokio::test]
async fn test_execution_mode_functionality() {
    // Test parsing
    assert_eq!(
        "freeform".parse::<ExecutionMode>().unwrap(),
        ExecutionMode::Freeform
    );
    assert_eq!(
        "structured".parse::<ExecutionMode>().unwrap(),
        ExecutionMode::Structured
    );
    assert_eq!(
        "free".parse::<ExecutionMode>().unwrap(),
        ExecutionMode::Freeform
    );
    assert_eq!(
        "struct".parse::<ExecutionMode>().unwrap(),
        ExecutionMode::Structured
    );
    assert_eq!(
        "graph".parse::<ExecutionMode>().unwrap(),
        ExecutionMode::Structured
    );

    // Test invalid parsing
    assert!("invalid".parse::<ExecutionMode>().is_err());

    // Test display
    assert_eq!(ExecutionMode::Freeform.to_string(), "freeform");
    assert_eq!(ExecutionMode::Structured.to_string(), "structured");

    // Test default
    assert_eq!(ExecutionMode::default(), ExecutionMode::Freeform);
}

/// Test PlanManager core functionality
#[tokio::test]
async fn test_plan_manager_core() {
    let mut manager = PlanManager::new();

    // Test initial state
    assert!(!manager.is_enabled());
    assert!(!manager.has_plan());

    // Test enable/disable
    manager.enable();
    assert!(manager.is_enabled());

    manager.disable();
    assert!(!manager.is_enabled());
}

/// Test CriticManager core functionality
#[tokio::test]
async fn test_critic_manager_core() {
    let manager = CriticManager::with_defaults();

    // Create basic critique context
    let context = CritiqueContext::new("Implement authentication")
        .with_modified_files(vec!["src/auth.rs".to_string()])
        .with_working_dir("/workspace");

    // Test critique execution
    let result: Result<AggregatedCritique> = manager.critique(&context).await;
    assert!(result.is_ok());

    if let Ok(critique) = result {
        // Verify critique structure exists and can be accessed
        let _total = critique.total_issues;
        let _blocking = critique.blocking_issues;
        let _results_count = critique.results.len();
    }
}

/// Test StateGraphConfig enhanced fields
#[tokio::test]
async fn test_state_graph_enhanced_config() {
    let config = StateGraphConfig {
        max_iterations: 5,
        max_fix_attempts: 2,
        test_command: Some("echo test".to_string()),
        working_dir: PathBuf::from("."),
        use_done_gate: true,
        project_type: Some(ProjectType::Rust),
    };

    let graph = StateGraph::new(config);

    // Verify StateGraph was created successfully with enhanced config
    // Config verification happens during construction
    assert_eq!(
        graph.current_state(),
        goose::agents::state_graph::GraphState::Code
    );
}

/// Test ProjectType enum
#[tokio::test]
async fn test_project_type_enum() {
    assert_eq!(ProjectType::default(), ProjectType::Rust);

    // Test all variants exist
    let _rust = ProjectType::Rust;
    let _node = ProjectType::Node;
    let _python = ProjectType::Python;
    let _custom = ProjectType::Custom;
}
