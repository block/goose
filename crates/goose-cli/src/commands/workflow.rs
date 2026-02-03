use anyhow::Result;
use goose::agents::ExecutionMode;
use goose::agents::{
    AgentOrchestrator, OrchestratorConfig, WorkflowEngine, WorkflowExecutionConfig,
};
use goose::approval::presets::ApprovalPreset;
use serde_json::json;
use std::collections::HashMap;
use std::str::FromStr;
use std::time::Duration;
use tokio::time::sleep;
use uuid::Uuid;

use crate::cli::WorkflowCommand;

pub async fn handle_workflow_command(command: WorkflowCommand) -> Result<()> {
    match command {
        WorkflowCommand::Execute {
            template,
            working_dir,
            language,
            framework,
            environment,
            skip_tasks,
            timeout_overrides,
            parameters,
            approval_policy,
            execution_mode,
        } => {
            handle_workflow_execute(
                template,
                working_dir,
                language,
                framework,
                environment,
                skip_tasks,
                timeout_overrides,
                parameters,
                approval_policy,
                execution_mode,
            )
            .await
        }
        WorkflowCommand::List { format, verbose } => handle_workflow_list(format, verbose).await,
        WorkflowCommand::Info { template } => handle_workflow_info(template).await,
        WorkflowCommand::Status {
            workflow_id,
            follow,
        } => handle_workflow_status(workflow_id, follow).await,
        WorkflowCommand::Executions { format, limit } => {
            handle_workflow_executions(format, limit).await
        }
    }
}

async fn handle_workflow_execute(
    template: String,
    working_dir: Option<String>,
    language: Option<String>,
    framework: Option<String>,
    environment: Option<String>,
    skip_tasks: Vec<String>,
    timeout_overrides: Vec<(String, String)>,
    parameters: Vec<(String, String)>,
    approval_policy: Option<String>,
    execution_mode: Option<String>,
) -> Result<()> {
    println!("🚀 Executing enterprise workflow: {}", template);

    // Parse approval policy
    let approval = approval_policy
        .as_deref()
        .unwrap_or("safe")
        .parse::<ApprovalPreset>()?;

    // Parse execution mode
    let exec_mode = execution_mode
        .as_deref()
        .unwrap_or("freeform")
        .parse::<ExecutionMode>()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    // Create orchestrator configuration
    let orchestrator_config = OrchestratorConfig {
        max_concurrent_workflows: 5,
        max_concurrent_tasks: 20,
        task_timeout: Duration::from_secs(3600),
        retry_attempts: 3,
        approval_policy: approval,
        enable_parallel_execution: true,
        task_queue_size: 100,
    };

    // Initialize orchestrator and workflow engine
    println!("📋 Initializing multi-agent orchestrator...");
    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(orchestrator.into()).await?;

    // Build task overrides for skipped tasks and timeout overrides
    let mut task_overrides = HashMap::new();

    // Add skip task overrides
    for task_name in skip_tasks {
        task_overrides.insert(
            task_name,
            goose::agents::TaskOverride {
                skip: true,
                timeout: None,
                custom_config: HashMap::new(),
            },
        );
    }

    // Add timeout overrides
    for (task_name, timeout_str) in timeout_overrides {
        let timeout_secs: u64 = timeout_str
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid timeout value: {}", timeout_str))?;

        task_overrides.insert(
            task_name,
            goose::agents::TaskOverride {
                skip: false,
                timeout: Some(Duration::from_secs(timeout_secs)),
                custom_config: HashMap::new(),
            },
        );
    }

    // Build workflow parameters
    let mut workflow_params = HashMap::new();
    for (key, value) in parameters {
        workflow_params.insert(key, json!(value));
    }

    // Create workflow execution configuration
    let config = WorkflowExecutionConfig {
        working_dir: working_dir.unwrap_or_else(|| {
            std::env::current_dir()
                .unwrap_or_else(|_| "/workspace".into())
                .to_string_lossy()
                .to_string()
        }),
        language,
        framework,
        environment: environment.unwrap_or_else(|| "development".to_string()),
        parameters: workflow_params,
        task_overrides,
    };

    println!("🔧 Configuration:");
    println!("  • Template: {}", template);
    println!("  • Working Directory: {}", config.working_dir);
    if let Some(lang) = &config.language {
        println!("  • Language: {}", lang);
    }
    if let Some(fw) = &config.framework {
        println!("  • Framework: {}", fw);
    }
    println!("  • Environment: {}", config.environment);
    println!("  • Execution Mode: {:?}", exec_mode);
    println!("  • Approval Policy: {:?}", approval);

    // Execute the workflow
    println!("\n🎯 Starting workflow execution...");
    let workflow_id = workflow_engine.execute_workflow(&template, config).await?;

    println!("✅ Workflow started with ID: {}", workflow_id);
    println!("📊 Monitoring workflow progress...\n");

    // Monitor workflow progress
    let mut last_status: Option<goose::agents::WorkflowExecutionStatus> = None;
    loop {
        let status = workflow_engine.get_execution_status(workflow_id).await;

        if last_status != status {
            if let Some(ref current_status) = status {
                match current_status {
                    goose::agents::WorkflowExecutionStatus::Preparing => {
                        println!("🔄 Status: Preparing workflow and initializing tasks...");
                    }
                    goose::agents::WorkflowExecutionStatus::Running => {
                        println!("▶️  Status: Workflow is actively executing...");
                    }
                    goose::agents::WorkflowExecutionStatus::Paused => {
                        println!("⏸️  Status: Workflow execution paused");
                    }
                    goose::agents::WorkflowExecutionStatus::Completed => {
                        println!("🎉 Status: Workflow completed successfully!");
                        break;
                    }
                    goose::agents::WorkflowExecutionStatus::Failed => {
                        println!("❌ Status: Workflow execution failed");
                        if let Ok(details) = workflow_engine.get_failure_details(workflow_id).await
                        {
                            println!("Error details: {:?}", details);
                        }
                        break;
                    }
                    goose::agents::WorkflowExecutionStatus::Cancelled => {
                        println!("🚫 Status: Workflow execution cancelled");
                        break;
                    }
                }
            }
            last_status = status;
        }

        // Show detailed task progress
        if let Ok(tasks) = workflow_engine.get_workflow_tasks(workflow_id).await {
            for task in tasks {
                match task.status {
                    goose::agents::TaskStatus::InProgress => {
                        println!(
                            "  🔄 {}: Running ({}%)",
                            task.name, task.progress_percentage
                        );
                    }
                    goose::agents::TaskStatus::Completed => {
                        println!("  ✅ {}: Completed", task.name);
                    }
                    goose::agents::TaskStatus::Failed => {
                        println!("  ❌ {}: Failed", task.name);
                    }
                    goose::agents::TaskStatus::Retrying => {
                        println!("  🔄 {}: Retrying...", task.name);
                    }
                    goose::agents::TaskStatus::Skipped => {
                        println!("  ⏭️  {}: Skipped", task.name);
                    }
                    goose::agents::TaskStatus::Pending
                    | goose::agents::TaskStatus::Blocked
                    | goose::agents::TaskStatus::Cancelled => {
                        // Only show pending/blocked/cancelled tasks occasionally to avoid spam
                    }
                }
            }
        }

        if workflow_engine.is_complete(workflow_id).await? {
            break;
        }

        sleep(Duration::from_secs(5)).await;
    }

    // Get final results
    if let Ok(result) = workflow_engine.get_workflow_result(workflow_id).await {
        println!("\n📋 Workflow Results:");
        println!("  • Status: {:?}", result.status);
        println!("  • Total Duration: {:?}", result.total_duration);
        println!("  • Tasks Completed: {}", result.completed_tasks);
        println!("  • Tasks Failed: {}", result.failed_tasks);

        if let Some(artifacts) = result.artifacts {
            println!("  • Generated Artifacts: {}", artifacts.len());
            for artifact in artifacts {
                println!("    - {}: {}", artifact.artifact_type, artifact.path);
            }
        }
    }

    println!("\n🏁 Workflow execution completed!");
    Ok(())
}

async fn handle_workflow_list(format: String, verbose: bool) -> Result<()> {
    println!("📋 Available Enterprise Workflow Templates:\n");

    // Create a temporary workflow engine to get templates
    let orchestrator_config = OrchestratorConfig::default();
    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(orchestrator.into()).await?;

    let templates = workflow_engine.list_templates().await;

    if format == "json" {
        let json_output = serde_json::to_string_pretty(&templates)?;
        println!("{}", json_output);
        return Ok(());
    }

    for template in templates {
        println!("🚀 **{}**", template.name);
        println!("   Description: {}", template.description);
        println!("   Category: {:?}", template.category);
        println!("   Complexity: {:?}", template.complexity);
        println!("   Duration: {:?}", template.estimated_duration);
        println!("   Tasks: {}", template.tasks.len());

        if verbose {
            println!("   Task Details:");
            for task in template.tasks {
                println!(
                    "     • {} ({:?}) - {}",
                    task.name, task.role, task.description
                );
                println!(
                    "       Duration: {:?}, Priority: {:?}",
                    task.estimated_duration, task.priority
                );
                if !task.dependencies.is_empty() {
                    println!("       Dependencies: {:?}", task.dependencies);
                }
            }
        }
        println!();
    }

    println!("💡 Usage: goose workflow execute <template_name> [options]");
    Ok(())
}

async fn handle_workflow_info(template: String) -> Result<()> {
    println!("📋 Workflow Template: {}\n", template);

    // Create a temporary workflow engine to get template details
    let orchestrator_config = OrchestratorConfig::default();
    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(orchestrator.into()).await?;

    if let Some(template_info) = workflow_engine.get_template(&template).await {
        println!("🚀 **{}**", template_info.name);
        println!("📝 Description: {}", template_info.description);
        println!("🏷️  Category: {:?}", template_info.category);
        println!("⚡ Complexity: {:?}", template_info.complexity);
        println!(
            "⏱️  Estimated Duration: {:?}",
            template_info.estimated_duration
        );
        println!("📊 Total Tasks: {}", template_info.tasks.len());

        println!("\n📋 Task Breakdown:");
        for (i, task) in template_info.tasks.iter().enumerate() {
            println!("{}. **{}** ({:?})", i + 1, task.name, task.role);
            println!("   Description: {}", task.description);
            println!(
                "   Duration: {:?} | Priority: {:?}",
                task.estimated_duration, task.priority
            );

            if !task.dependencies.is_empty() {
                println!("   Dependencies: {:?}", task.dependencies);
            }

            if !task.required_skills.is_empty() {
                println!("   Skills: {:?}", task.required_skills);
            }

            if !task.validation_criteria.is_empty() {
                println!("   Validation: {:?}", task.validation_criteria);
            }
            println!();
        }

        println!("💡 Example Usage:");
        println!("   goose workflow execute {} \\", template);
        println!("     --language rust \\");
        println!("     --framework axum \\");
        println!("     --environment production \\");
        println!("     --approval-policy safe");
    } else {
        println!("❌ Template '{}' not found", template);
        println!("\n📋 Available templates:");
        println!("   • fullstack_webapp");
        println!("   • microservice");
        println!("   • comprehensive_testing");
        println!("\n💡 Use 'goose workflow list' to see all available templates");
    }

    Ok(())
}

async fn handle_workflow_status(workflow_id: String, follow: bool) -> Result<()> {
    let uuid =
        Uuid::from_str(&workflow_id).map_err(|_| anyhow::anyhow!("Invalid workflow ID format"))?;

    println!("📊 Workflow Status: {}\n", workflow_id);

    // Create a temporary workflow engine to check status
    let orchestrator_config = OrchestratorConfig::default();
    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(orchestrator.into()).await?;

    if follow {
        println!("👀 Following workflow progress (Ctrl+C to stop)...\n");

        let mut last_status: Option<goose::agents::WorkflowExecutionStatus> = None;
        loop {
            let status = workflow_engine.get_execution_status(uuid).await;

            if last_status != status {
                let timestamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC");
                println!("[{}] Status: {:?}", timestamp, status);
                last_status = status.clone();
            }

            // Show task details
            if let Ok(tasks) = workflow_engine.get_workflow_tasks(uuid).await {
                for task in tasks {
                    match task.status {
                        goose::agents::TaskStatus::InProgress => {
                            println!("  🔄 {}: {}%", task.name, task.progress_percentage);
                        }
                        goose::agents::TaskStatus::Completed => {
                            println!("  ✅ {}: Completed", task.name);
                        }
                        goose::agents::TaskStatus::Failed => {
                            println!("  ❌ {}: Failed", task.name);
                        }
                        _ => {}
                    }
                }
            }

            if workflow_engine.is_complete(uuid).await? {
                println!("\n🏁 Workflow completed!");
                break;
            }

            sleep(Duration::from_secs(3)).await;
        }
    } else {
        // Single status check
        let status = workflow_engine.get_execution_status(uuid).await;
        println!("Status: {:?}", status);

        if let Ok(tasks) = workflow_engine.get_workflow_tasks(uuid).await {
            println!("\n📋 Task Status:");
            for task in tasks {
                let status_icon = match task.status {
                    goose::agents::TaskStatus::Pending => "⏳",
                    goose::agents::TaskStatus::InProgress => "🔄",
                    goose::agents::TaskStatus::Completed => "✅",
                    goose::agents::TaskStatus::Failed => "❌",
                    goose::agents::TaskStatus::Skipped => "⏭️",
                    goose::agents::TaskStatus::Retrying => "🔄",
                    goose::agents::TaskStatus::Blocked => "🚫",
                    goose::agents::TaskStatus::Cancelled => "🚫",
                };
                println!("  {} {}: {:?}", status_icon, task.name, task.status);
            }
        }

        if let Ok(result) = workflow_engine.get_workflow_result(uuid).await {
            println!("\n📊 Execution Summary:");
            println!("  • Duration: {:?}", result.total_duration);
            println!("  • Completed: {}", result.completed_tasks);
            println!("  • Failed: {}", result.failed_tasks);
        }
    }

    Ok(())
}

async fn handle_workflow_executions(format: String, limit: Option<usize>) -> Result<()> {
    println!("📋 Workflow Executions\n");

    // Create a temporary workflow engine to get executions
    let orchestrator_config = OrchestratorConfig::default();
    let orchestrator = AgentOrchestrator::with_config(orchestrator_config).await?;
    let workflow_engine = WorkflowEngine::new(orchestrator.into()).await?;

    let executions = workflow_engine.list_executions(limit).await?;

    if format == "json" {
        let json_output = serde_json::to_string_pretty(&executions)?;
        println!("{}", json_output);
        return Ok(());
    }

    if executions.is_empty() {
        println!("📭 No workflow executions found");
        println!("💡 Start a workflow with: goose workflow execute <template_name>");
        return Ok(());
    }

    for execution in executions {
        let status_icon = match execution.status {
            goose::agents::WorkflowExecutionStatus::Preparing => "🔄",
            goose::agents::WorkflowExecutionStatus::Running => "▶️",
            goose::agents::WorkflowExecutionStatus::Paused => "⏸️",
            goose::agents::WorkflowExecutionStatus::Completed => "✅",
            goose::agents::WorkflowExecutionStatus::Failed => "❌",
            goose::agents::WorkflowExecutionStatus::Cancelled => "🚫",
        };

        println!(
            "{} **{}** ({})",
            status_icon, execution.template_name, execution.id
        );
        println!(
            "   Started: {}",
            execution.start_time.format("%Y-%m-%d %H:%M:%S UTC")
        );
        if let Some(end_time) = execution.end_time {
            println!("   Ended: {}", end_time.format("%Y-%m-%d %H:%M:%S UTC"));
            println!("   Duration: {:?}", end_time - execution.start_time);
        }
        println!("   Status: {:?}", execution.status);
        println!(
            "   Tasks: {} completed, {} failed",
            execution.completed_tasks, execution.failed_tasks
        );
        println!();
    }

    println!("💡 Use 'goose workflow status <workflow_id>' for detailed status");
    Ok(())
}
