use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric};
use crate::register_evaluation;
use async_trait::async_trait;
use mcp_core::role::Role;
use goose::message::MessageContent;
use serde_json;

#[derive(Debug)]
pub struct DeveloperListFiles {
}

impl DeveloperListFiles {
    pub fn new() -> Self {
        DeveloperListFiles {}
    }
}

#[async_trait]
impl Evaluation for DeveloperListFiles {
    async fn run(&self, mut agent: Box<dyn BenchAgent>) -> anyhow::Result<Vec<EvaluationMetric>> {
        let mut metrics = Vec::new();
        
        // Send the prompt to list files
        let messages = agent.prompt("list the files in the current directory".to_string()).await?;
        // println!("asdhflkahjsdflkasdfl");
        
        // Check if the assistant makes appropriate tool calls
        let valid_tool_call = messages.iter().any(|msg| {
            // Check if it's an assistant message
            msg.role == Role::Assistant && 
            // Check if any content item is a tool request for listing files
            msg.content.iter().any(|content| {
                if let MessageContent::ToolRequest(tool_req) = content {
                    // Check if the tool call is for shell with ls or rg --files
                    if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                        let args: String = serde_json::from_value(tool_call.arguments.clone())
                            .unwrap_or_default();
                        
                        tool_call.name == "developer__shell" &&                    
                        (args.to_lowercase().contains("ls ") || 
                         args.to_lowercase().contains("ls\n") ||
                         args.to_lowercase().contains("ls$") ||
                         args.contains("rg --files"))
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
        });
        
        metrics.push(EvaluationMetric::Boolean(valid_tool_call));
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "developer_list_files"
    }
}

register_evaluation!("core", DeveloperListFiles);