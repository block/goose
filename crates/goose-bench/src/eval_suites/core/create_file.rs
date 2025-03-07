// Create a new file called test.txt with the content 'Hello, World!

use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric, ExtensionRequirements};
use crate::register_evaluation;
use crate::work_dir::WorkDir;
use async_trait::async_trait;
use goose::message::MessageContent;
use mcp_core::role::Role;
use serde_json::{self, Value};

#[derive(Debug)]
pub struct DeveloperCreateFile {}

impl DeveloperCreateFile {
    pub fn new() -> Self {
        DeveloperCreateFile {}
    }
}

#[async_trait]
impl Evaluation for DeveloperCreateFile {
    async fn run(
        &self,
        mut agent: Box<dyn BenchAgent>,
        _work_dir: &mut WorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        let mut metrics = Vec::new();

        // Send the prompt to create and read file
        let messages = agent.prompt("Create a new file called test.txt in the current directory with the content 'Hello, World!'. Then read the contents of the new file to confirm.".to_string()).await?;

        // Check for write operation
        let write_tool_call = messages.iter().any(|msg| {
            // Check if it's an assistant message
            msg.role == Role::Assistant &&
            // Check if any content item is a tool request for creating a file
            msg.content.iter().any(|content| {
                if let MessageContent::ToolRequest(tool_req) = content {
                    if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                        // Check tool name is correct
                        if tool_call.name != "developer__text_editor" {
                            return false;
                        }

                        // Parse the arguments as JSON
                        if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
                            // Check all required parameters match exactly
                            args.get("command").and_then(Value::as_str) == Some("write") &&
                            args.get("path").and_then(Value::as_str).is_some_and(|s| s.contains("test.txt")) &&
                            args.get("file_text").and_then(Value::as_str) == Some("Hello, World!")
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
        });

        // Check for read operation
        let read_tool_call = messages.iter().any(|msg| {
            // Check if it's an assistant message
            msg.role == Role::Assistant &&
            // Check if any content item is a tool request for reading a file
            msg.content.iter().any(|content| {
                if let MessageContent::ToolRequest(tool_req) = content {
                    if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                        // Check tool name is correct
                        if tool_call.name != "developer__text_editor" {
                            return false;
                        }

                        // Parse the arguments as JSON
                        if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
                            // Check all required parameters match exactly
                            args.get("command").and_then(Value::as_str) == Some("view") &&
                            args.get("path").and_then(Value::as_str).is_some_and(|s| s.contains("test.txt"))
                        } else {
                            false
                        }
                    } else {
                        false
                    }
                } else {
                    false
                }
            })
        });

        metrics.push((
            "Create file".to_string(),
            EvaluationMetric::Boolean(write_tool_call),
        ));
        metrics.push((
            "Read file".to_string(),
            EvaluationMetric::Boolean(read_tool_call),
        ));
        metrics.push((
            "Complete create and read".to_string(),
            EvaluationMetric::Boolean(write_tool_call && read_tool_call),
        ));
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "developer_create_read_file"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: Vec::new(),
        }
    }
}

register_evaluation!("developer", DeveloperCreateFile);
