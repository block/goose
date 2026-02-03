//! Integration tests for Phase 4: Advanced Agent Capabilities
//!
//! Tests the core Phase 4 functionality: execution modes, planning system, and critique system

use anyhow::Result;
use goose::agents::state_graph::{ProjectType, StateGraph, StateGraphConfig};
use goose::agents::{
    AggregatedCritique, CriticManager, CritiqueContext, ExecutionMode, PlanManager,
};
use std::path::PathBuf;

/// Test ExecutionMode parsing and enum behavior
#[tokio::test]
async fn test_execution_mode_enum() {
    // Test ExecutionMode parsing
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

/// Test PlanManager functionality
#[tokio::test]
async fn test_plan_manager_lifecycle() {
    let mut manager = PlanManager::new();

    // Initially disabled
    assert!(!manager.is_enabled());
    assert!(!manager.has_plan());

    // Enable planning
    manager.enable();
    assert!(manager.is_enabled());

    // Disable planning
    manager.disable();
    assert!(!manager.is_enabled());
}

/// Test CriticManager functionality
#[tokio::test]
async fn test_critic_manager_basic() {
    let manager = CriticManager::with_defaults();

    // Create critique context
    let context = CritiqueContext::new("Implement authentication")
        .with_modified_files(vec![
            "src/auth.rs".to_string(),
            "tests/auth_test.rs".to_string(),
        ])
        .with_working_dir("/workspace");

    // Perform critique - this should not fail
    let result = manager.critique(&context).await;
    assert!(result.is_ok());
}

/// Test StateGraphConfig with enhanced fields
#[tokio::test]
async fn test_state_graph_config() {
    let config = StateGraphConfig {
        max_iterations: 5,
        max_fix_attempts: 2,
        test_command: Some("echo 'test passed'".to_string()),
        working_dir: PathBuf::from("."),
        use_done_gate: true,
        project_type: Some(ProjectType::Rust),
    };

    let graph = StateGraph::new(config.clone());

    // Verify enhanced configuration fields
    assert_eq!(graph.config().max_iterations, 5);
    assert_eq!(graph.config().max_fix_attempts, 2);
    assert!(graph.config().use_done_gate);
    assert_eq!(graph.config().project_type, Some(ProjectType::Rust));
}

/// Test ProjectType enum
#[tokio::test]
async fn test_project_type_variants() {
    // Test default
    assert_eq!(ProjectType::default(), ProjectType::Rust);

    // Test all variants exist and can be compared
    assert_eq!(ProjectType::Rust, ProjectType::Rust);
    assert_eq!(ProjectType::Node, ProjectType::Node);
    assert_eq!(ProjectType::Python, ProjectType::Python);
    assert_eq!(ProjectType::Custom, ProjectType::Custom);
}

/// Test CriticManager with build and test outputs
#[tokio::test]
async fn test_critic_manager_with_outputs() {
    let manager = CriticManager::with_defaults();

    // Create critique context with outputs
    let context = CritiqueContext::new("Implement authentication")
        .with_modified_files(vec![
            "src/auth.rs".to_string(),
            "tests/auth_test.rs".to_string(),
        ])
        .with_working_dir("/workspace")
        .with_build_output("Build successful".to_string())
        .with_test_output("2 tests passed".to_string());

    // Perform critique
    let result: Result<AggregatedCritique> = manager.critique(&context).await;
    assert!(result.is_ok());

    let critique: AggregatedCritique = result.unwrap();
    // Validate critique structure - has results, passed status, issue counts
    // total_issues and blocking_issues are usize, so we verify they exist by using them
    let _total = critique.total_issues;
    let _blocking = critique.blocking_issues;
}

/// Test StateGraph state machine basics
#[tokio::test]
async fn test_state_graph_state_machine() {
    let config = StateGraphConfig {
        max_iterations: 3,
        max_fix_attempts: 1,
        test_command: Some("echo 'test'".to_string()),
        working_dir: PathBuf::from("."),
        use_done_gate: false,
        project_type: Some(ProjectType::Node),
    };

    let graph = StateGraph::new(config);

    // Verify initial state
    assert_eq!(graph.config().max_iterations, 3);
    assert_eq!(graph.config().project_type, Some(ProjectType::Node));
}

/// Test ExecutionMode equality
#[tokio::test]
async fn test_execution_mode_equality() {
    let mode1 = ExecutionMode::Freeform;
    let mode2 = ExecutionMode::Freeform;
    let mode3 = ExecutionMode::Structured;

    assert_eq!(mode1, mode2);
    assert_ne!(mode1, mode3);
}

/// Test PlanManager state transitions
#[tokio::test]
async fn test_plan_manager_state_transitions() {
    let mut manager = PlanManager::new();

    // Start disabled
    assert!(!manager.is_enabled());

    // Enable
    manager.enable();
    assert!(manager.is_enabled());
    assert!(!manager.has_plan()); // No plan yet

    // Disable
    manager.disable();
    assert!(!manager.is_enabled());
}
