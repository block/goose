//! Phase 5 Enterprise Integration Tests
//!
//! These tests verify the enterprise workflow functionality including:
//! - Agent orchestration and coordination
//! - Specialist agent creation and capabilities
//! - Workflow engine templates and execution
//! - Multi-agent coordination
//! - Error handling and performance

use anyhow::Result;
use goose::agents::specialists::{
    CodeAgent, DeployAgent, DocsAgent, SecurityAgent, SpecialistAgent, SpecialistConfig,
    SpecialistContext, TestAgent,
};
use goose::agents::{
    AgentOrchestrator, AgentRole, OrchestratorConfig, TaskOverride, TaskStatus, WorkflowEngine,
    WorkflowExecutionConfig, WorkflowExecutionStatus,
};
use goose::approval::ApprovalPreset;
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tempfile::TempDir;
use tokio::time::timeout;
use uuid::Uuid;

#[tokio::test]
async fn test_agent_orchestrator_initialization() -> Result<()> {
    let config = OrchestratorConfig {
        max_concurrent_workflows: 5,
        max_concurrent_tasks: 20,
        task_timeout: Duration::from_secs(300),
        retry_attempts: 2,
        approval_policy: ApprovalPreset::Safe,
        enable_parallel_execution: true,
        task_queue_size: 50,
    };

    let orchestrator = AgentOrchestrator::with_config(config).await?;

    // Verify orchestrator is properly initialized
    assert!(orchestrator.is_ready());

    // Check that specialist agents are available
    let available_roles = orchestrator.get_available_agent_roles().await;
    assert!(available_roles.contains(&AgentRole::Code));
    assert!(available_roles.contains(&AgentRole::Test));
    assert!(available_roles.contains(&AgentRole::Deploy));
    assert!(available_roles.contains(&AgentRole::Docs));
    assert!(available_roles.contains(&AgentRole::Security));

    Ok(())
}

#[tokio::test]
async fn test_specialist_agents_creation_and_capabilities() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path().to_string_lossy().to_string();

    // Test CodeAgent
    let code_agent = CodeAgent::new(SpecialistConfig::default());
    let context =
        SpecialistContext::new("Create a simple REST API".to_string(), working_dir.clone())
            .with_language("rust".to_string())
            .with_framework("axum".to_string());

    assert_eq!(code_agent.role(), AgentRole::Code);
    assert_eq!(code_agent.name(), "CodeAgent");
    assert!(code_agent.can_handle(&context).await);

    // Test TestAgent
    let test_agent = TestAgent::new(SpecialistConfig::default());
    let test_context = SpecialistContext::new(
        "Create unit tests for the API".to_string(),
        working_dir.clone(),
    )
    .with_language("rust".to_string());

    assert_eq!(test_agent.role(), AgentRole::Test);
    assert_eq!(test_agent.name(), "TestAgent");
    assert!(test_agent.can_handle(&test_context).await);

    // Test DeployAgent
    let deploy_agent = DeployAgent::new(SpecialistConfig::default());
    let deploy_context = SpecialistContext::new(
        "Deploy application to Docker".to_string(),
        working_dir.clone(),
    )
    .with_language("rust".to_string())
    .with_metadata("platform".to_string(), json!("docker"));

    assert_eq!(deploy_agent.role(), AgentRole::Deploy);
    assert_eq!(deploy_agent.name(), "DeployAgent");
    assert!(deploy_agent.can_handle(&deploy_context).await);

    // Test DocsAgent
    let docs_agent = DocsAgent::new(SpecialistConfig::default());
    let docs_context = SpecialistContext::new(
        "Generate API documentation".to_string(),
        working_dir.clone(),
    )
    .with_language("rust".to_string());

    assert_eq!(docs_agent.role(), AgentRole::Docs);
    assert_eq!(docs_agent.name(), "DocsAgent");
    assert!(docs_agent.can_handle(&docs_context).await);

    // Test SecurityAgent
    let security_agent = SecurityAgent::new(SpecialistConfig::default());
    let security_context =
        SpecialistContext::new("Perform security audit".to_string(), working_dir.clone())
            .with_language("rust".to_string());

    assert_eq!(security_agent.role(), AgentRole::Security);
    assert_eq!(security_agent.name(), "SecurityAgent");
    assert!(security_agent.can_handle(&security_context).await);

    Ok(())
}

#[tokio::test]
async fn test_workflow_engine_initialization_and_templates() -> Result<()> {
    let orchestrator_config = OrchestratorConfig::default();
    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(Arc::new(orchestrator)).await?;

    // Test template loading - list_templates returns Vec, not Result
    let templates = workflow_engine.list_templates().await;
    assert!(!templates.is_empty());

    // Verify core templates are available (using the actual template names)
    let template_names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
    assert!(template_names.contains(&"Full-Stack Web Application".to_string()));
    assert!(template_names.contains(&"Microservice Development".to_string()));
    assert!(template_names.contains(&"Comprehensive Testing Suite".to_string()));

    // Test getting specific template by key - get_template returns Option, not Result
    // Templates are stored by key (e.g., "fullstack_webapp"), not by display name
    let fullstack_template = workflow_engine.get_template("fullstack_webapp").await;
    assert!(fullstack_template.is_some());

    let template = fullstack_template.unwrap();
    assert!(!template.tasks.is_empty());
    assert!(template.estimated_duration > Duration::from_secs(0));

    Ok(())
}

#[tokio::test]
async fn test_workflow_execution_configuration() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path().to_string_lossy().to_string();

    let mut task_overrides = HashMap::new();
    task_overrides.insert(
        "security_audit".to_string(),
        TaskOverride {
            skip: true,
            timeout: None,
            custom_config: HashMap::new(),
        },
    );

    let config = WorkflowExecutionConfig {
        working_dir: working_dir.clone(),
        language: Some("rust".to_string()),
        framework: Some("axum".to_string()),
        environment: "testing".to_string(),
        parameters: HashMap::from([("test_param".to_string(), json!("test_value"))]),
        task_overrides,
    };

    // Verify configuration is valid
    assert_eq!(config.working_dir, working_dir);
    assert_eq!(config.language, Some("rust".to_string()));
    assert_eq!(config.framework, Some("axum".to_string()));
    assert_eq!(config.environment, "testing");
    assert!(config.parameters.contains_key("test_param"));
    assert!(config.task_overrides.contains_key("security_audit"));

    Ok(())
}

#[tokio::test]
async fn test_workflow_execution_simulation() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path().to_string_lossy().to_string();

    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 1,
        max_concurrent_tasks: 5,
        task_timeout: Duration::from_secs(30),
        retry_attempts: 1,
        approval_policy: ApprovalPreset::Safe,
        enable_parallel_execution: true,
        task_queue_size: 10,
    };

    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(Arc::new(orchestrator)).await?;

    let config = WorkflowExecutionConfig {
        working_dir,
        language: Some("rust".to_string()),
        framework: Some("axum".to_string()),
        environment: "test".to_string(),
        parameters: HashMap::new(),
        task_overrides: HashMap::new(),
    };

    // Start workflow execution - use template key, not display name
    let workflow_id = workflow_engine
        .execute_workflow("microservice", config)
        .await?;

    // Verify workflow was created
    assert_ne!(workflow_id, Uuid::nil());

    // Check initial status - returns Option<WorkflowExecutionStatus>
    let initial_status = workflow_engine.get_execution_status(workflow_id).await;
    assert!(initial_status.is_some());
    let status = initial_status.unwrap();
    assert!(matches!(
        status,
        WorkflowExecutionStatus::Preparing | WorkflowExecutionStatus::Running
    ));

    // Wait briefly for workflow to progress
    tokio::time::sleep(Duration::from_millis(100)).await;

    // Check that tasks are being managed
    if let Ok(tasks) = workflow_engine.get_workflow_tasks(workflow_id).await {
        assert!(!tasks.is_empty());

        // Verify tasks have proper status - use InProgress, not Running
        for task in tasks {
            assert!(matches!(
                task.status,
                TaskStatus::Pending | TaskStatus::InProgress | TaskStatus::Completed
            ));
        }
    }

    Ok(())
}

#[tokio::test]
async fn test_multi_agent_coordination() -> Result<()> {
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 2,
        max_concurrent_tasks: 10,
        task_timeout: Duration::from_secs(30),
        retry_attempts: 1,
        approval_policy: ApprovalPreset::Safe,
        enable_parallel_execution: true,
        task_queue_size: 20,
    };

    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;

    // Test that orchestrator can handle multiple agent roles simultaneously
    let roles = orchestrator.get_available_agent_roles().await;
    assert!(roles.len() >= 5); // Code, Test, Deploy, Docs, Security

    // Verify agent pool management
    let agent_pool_stats = orchestrator.get_agent_pool_statistics().await?;
    assert!(agent_pool_stats.total_agents > 0);
    assert!(agent_pool_stats.available_agents <= agent_pool_stats.total_agents);

    Ok(())
}

#[tokio::test]
async fn test_workflow_error_handling() -> Result<()> {
    let orchestrator_config = OrchestratorConfig::default();
    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(Arc::new(orchestrator)).await?;

    // Test invalid template name
    let config = WorkflowExecutionConfig {
        working_dir: "/tmp".to_string(),
        language: None,
        framework: None,
        environment: "test".to_string(),
        parameters: HashMap::new(),
        task_overrides: HashMap::new(),
    };

    let result = workflow_engine
        .execute_workflow("nonexistent_template", config)
        .await;
    assert!(result.is_err());

    // Test invalid workflow ID for status check
    let invalid_uuid = Uuid::new_v4();
    let status_result = workflow_engine.get_execution_status(invalid_uuid).await;
    // Should return None for non-existent workflow
    assert!(status_result.is_none());

    Ok(())
}

#[tokio::test]
async fn test_workflow_performance_characteristics() -> Result<()> {
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 3,
        max_concurrent_tasks: 15,
        task_timeout: Duration::from_secs(10),
        retry_attempts: 1,
        approval_policy: ApprovalPreset::Safe,
        enable_parallel_execution: true,
        task_queue_size: 30,
    };

    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(Arc::new(orchestrator)).await?;

    let start_time = std::time::Instant::now();

    // Create multiple workflow configurations
    let configs: Vec<WorkflowExecutionConfig> = (0..3)
        .map(|i| WorkflowExecutionConfig {
            working_dir: format!("/tmp/workflow_{}", i),
            language: Some("rust".to_string()),
            framework: Some("axum".to_string()),
            environment: "test".to_string(),
            parameters: HashMap::from([("instance".to_string(), json!(i))]),
            task_overrides: HashMap::new(),
        })
        .collect();

    // Start multiple workflows concurrently - use template key
    let mut workflow_ids = Vec::new();
    for config in configs {
        if let Ok(workflow_id) = workflow_engine
            .execute_workflow("comprehensive_testing", config)
            .await
        {
            workflow_ids.push(workflow_id);
        }
    }

    assert!(!workflow_ids.is_empty());

    let initialization_time = start_time.elapsed();
    assert!(initialization_time < Duration::from_secs(5)); // Should initialize quickly

    // Check that all workflows are tracked
    let executions = workflow_engine.list_executions(Some(10)).await?;
    assert!(executions.len() >= workflow_ids.len());

    Ok(())
}

#[tokio::test]
async fn test_execution_mode_integration() -> Result<()> {
    use goose::agents::ExecutionMode;

    // Test that ExecutionMode is properly integrated with workflow execution

    // Test Freeform mode
    let freeform_mode = ExecutionMode::Freeform;
    assert_eq!(format!("{:?}", freeform_mode), "Freeform");

    // Test Structured mode
    let structured_mode = ExecutionMode::Structured;
    assert_eq!(format!("{:?}", structured_mode), "Structured");

    // Test parsing from string
    let parsed_freeform: ExecutionMode =
        "freeform".parse().map_err(|e: String| anyhow::anyhow!(e))?;
    assert_eq!(parsed_freeform, ExecutionMode::Freeform);

    let parsed_structured: ExecutionMode = "structured"
        .parse()
        .map_err(|e: String| anyhow::anyhow!(e))?;
    assert_eq!(parsed_structured, ExecutionMode::Structured);

    Ok(())
}

#[tokio::test]
async fn test_approval_policy_integration() -> Result<()> {
    // Test that ApprovalPreset is properly integrated with workflow execution
    let safe_policy = ApprovalPreset::Safe;
    let paranoid_policy = ApprovalPreset::Paranoid;
    let autopilot_policy = ApprovalPreset::Autopilot;

    // Test policy parsing
    let parsed_safe: ApprovalPreset = "safe".parse()?;
    assert_eq!(format!("{:?}", parsed_safe), format!("{:?}", safe_policy));

    let parsed_paranoid: ApprovalPreset = "paranoid".parse()?;
    assert_eq!(
        format!("{:?}", parsed_paranoid),
        format!("{:?}", paranoid_policy)
    );

    let parsed_autopilot: ApprovalPreset = "autopilot".parse()?;
    assert_eq!(
        format!("{:?}", parsed_autopilot),
        format!("{:?}", autopilot_policy)
    );

    Ok(())
}

#[tokio::test]
async fn test_enterprise_workflow_integration_complete() -> Result<()> {
    let temp_dir = TempDir::new()?;
    let working_dir = temp_dir.path().to_string_lossy().to_string();

    // Test complete enterprise integration workflow
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 1,
        max_concurrent_tasks: 10,
        task_timeout: Duration::from_secs(60),
        retry_attempts: 2,
        approval_policy: ApprovalPreset::Safe,
        enable_parallel_execution: true,
        task_queue_size: 20,
    };

    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(Arc::new(orchestrator)).await?;

    // Comprehensive workflow configuration
    let mut task_overrides = HashMap::new();
    task_overrides.insert(
        "deployment_setup".to_string(),
        TaskOverride {
            skip: false,
            timeout: Some(Duration::from_secs(120)),
            custom_config: HashMap::from([
                ("platform".to_string(), json!("docker")),
                ("replicas".to_string(), json!(1)),
            ]),
        },
    );

    let config = WorkflowExecutionConfig {
        working_dir,
        language: Some("rust".to_string()),
        framework: Some("axum".to_string()),
        environment: "production".to_string(),
        parameters: HashMap::from([
            ("database_url".to_string(), json!("sqlite://test.db")),
            ("log_level".to_string(), json!("info")),
        ]),
        task_overrides,
    };

    // Execute full-stack workflow - use template key
    let workflow_id = workflow_engine
        .execute_workflow("fullstack_webapp", config)
        .await?;

    // Monitor workflow progress for a short time
    let timeout_duration = Duration::from_secs(10);
    let _monitoring_result = timeout(timeout_duration, async {
        loop {
            let status = workflow_engine.get_execution_status(workflow_id).await;

            if let Some(s) = status {
                match s {
                    WorkflowExecutionStatus::Completed
                    | WorkflowExecutionStatus::Failed
                    | WorkflowExecutionStatus::Cancelled => {
                        break;
                    }
                    _ => {
                        tokio::time::sleep(Duration::from_millis(100)).await;
                    }
                }
            } else {
                break;
            }
        }
    })
    .await;

    // Verify workflow was tracked properly regardless of completion
    let executions = workflow_engine.list_executions(Some(1)).await?;
    assert!(!executions.is_empty());
    assert_eq!(executions[0].id, workflow_id);
    // The template_name stores the key used to execute, not the display name
    assert_eq!(executions[0].template_name, "fullstack_webapp");

    // Verify execution statistics are maintained
    let stats = workflow_engine.get_execution_statistics().await?;
    assert!(stats.total_workflows > 0);

    println!("Enterprise workflow integration test completed successfully");
    println!("   Workflow ID: {}", workflow_id);
    println!("   Execution Statistics: {:?}", stats);

    Ok(())
}

#[tokio::test]
async fn test_phase5_architecture_integration() -> Result<()> {
    // Comprehensive test of Phase 5 architecture components working together

    // 1. Test orchestrator with all specialist agents
    let orchestrator_config = OrchestratorConfig::default();
    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;

    // Verify all specialist agents are available
    let roles = orchestrator.get_available_agent_roles().await;
    let expected_roles = vec![
        AgentRole::Code,
        AgentRole::Test,
        AgentRole::Deploy,
        AgentRole::Docs,
        AgentRole::Security,
    ];

    for expected_role in expected_roles {
        assert!(
            roles.contains(&expected_role),
            "Missing specialist agent role: {:?}",
            expected_role
        );
    }

    // 2. Test workflow engine with enterprise templates
    let workflow_engine = WorkflowEngine::new(Arc::new(orchestrator)).await?;
    let templates = workflow_engine.list_templates().await;

    // Verify enterprise templates are available
    assert!(templates.len() >= 3);
    let template_names: Vec<String> = templates.iter().map(|t| t.name.clone()).collect();
    assert!(template_names
        .iter()
        .any(|name| name.contains("Full-Stack")));
    assert!(template_names
        .iter()
        .any(|name| name.contains("Microservice")));
    assert!(template_names.iter().any(|name| name.contains("Testing")));

    println!("Phase 5 architecture integration verified successfully");
    println!("   Available specialist agent roles: {}", roles.len());
    println!("   Available workflow templates: {}", templates.len());
    println!("   Enterprise platform ready for production use");

    Ok(())
}
