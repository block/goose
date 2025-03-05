// Create a new file called test.txt with the content 'Hello, World!

use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric};
use crate::register_evaluation;
use crate::work_dir::WorkDir;
use async_trait::async_trait;
use goose::message::MessageContent;
use mcp_core::role::Role;
use serde_json::{self, Value};

#[derive(Debug)]
pub struct ComputerControllerScript {}

impl ComputerControllerScript {
    pub fn new() -> Self {
        ComputerControllerScript {}
    }
}

#[async_trait]
impl Evaluation for ComputerControllerScript {
    async fn run(
        &self,
        mut agent: Box<dyn BenchAgent>,
        _work_dir: &mut WorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        let mut metrics = Vec::new();

        // Send the prompt to list files
        let messages = agent.prompt("Make a beep sound".to_string());
        let messages = messages.await?;

        let valid_tool_call = messages.iter().any(|msg| {
            // Check if it's an assistant message
            msg.role == Role::Assistant &&
            // Check if any content item is a tool request for creating a file
            msg.content.iter().any(|content| {
                if let MessageContent::ToolRequest(tool_req) = content {
                    if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                        // Check tool name is correct
                        if tool_call.name != "computercontroller__computer_control" {
                            return false;
                        }

                        // Parse the arguments as JSON
                        if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
                            // Check all required parameters match exactly
                            args.get("script").and_then(Value::as_str).is_some_and(|s| s.contains("beep"))
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
            "Running os scripts".to_string(),
            EvaluationMetric::Boolean(valid_tool_call),
        ));
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "computercontroller_script"
    }

    fn required_extensions(&self) -> Vec<String> {
        vec!["computercontroller".to_string()]
    }
}

register_evaluation!("computercontroller", ComputerControllerScript);
