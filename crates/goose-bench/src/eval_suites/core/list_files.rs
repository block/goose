use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric};
use crate::register_evaluation;
use crate::work_dir::WorkDir;
use async_trait::async_trait;
use goose::message::MessageContent;
use mcp_core::role::Role;
use serde_json::{self, Value};

#[derive(Debug)]
pub struct DeveloperListFiles {}

impl DeveloperListFiles {
    pub fn new() -> Self {
        DeveloperListFiles {}
    }
}

#[async_trait]
impl Evaluation for DeveloperListFiles {
    async fn run(
        &self,
        mut agent: Box<dyn BenchAgent>,
        _work_dir: &mut WorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        let mut metrics = Vec::new();

        // Send the prompt to list files
        let messages = agent
            .prompt("list the files in the current directory".to_string())
            .await?;
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
                            // Parse arguments as JSON Value first
                            if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
                                tool_call.name == "developer__shell" &&
                                    args.get("command")
                                        .and_then(Value::as_str).is_some_and(|cmd| {
                                        cmd.contains("ls ") ||
                                            cmd.contains("ls\n") ||
                                            cmd.contains("ls$") ||
                                            cmd.contains("rg --files")
                                    })
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
            "Using the shell command tool".to_string(),
            EvaluationMetric::Boolean(valid_tool_call),
        ));
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "developer_list_files"
    }

    fn required_extensions(&self) -> Vec<String> {
        vec!["developer".to_string()]
    }
}

register_evaluation!("developer", DeveloperListFiles);
