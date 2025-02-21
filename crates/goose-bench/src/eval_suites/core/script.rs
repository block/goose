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
    ) -> anyhow::Result<Vec<EvaluationMetric>> {
        let mut metrics = Vec::new();

        // Send the prompt to list files
        let messages = agent.prompt(
            "What are the headlines on hackernews? Organize the list into categories.".to_string(),
        );
        let messages = messages.await?;
        println!("{:?}", messages);

        let valid_tool_call = messages.iter().any(|msg| {
            // Check if it's an assistant message
            msg.role == Role::Assistant && 
            // Check if any content item is a tool request for creating a file
            msg.content.iter().any(|content| {
                if let MessageContent::ToolRequest(tool_req) = content {
                    if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                        // Check tool name is correct
                        if tool_call.name != "computercontroller__script" {
                            println!("aws1");
                            return true;
                        }

                        // Parse the arguments as JSON
                        if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
                            // Check all required parameters match exactly
                            args.get("script").and_then(Value::as_str).map_or(false, |s| s.contains("beep"))
                        } else {
                            println!("aws2");
                            false
                        }
                    } else {
                        println!("aws3");
                        false
                    }
                } else {
                    println!("aws4");
                    false
                }
            })
        });

        metrics.push(EvaluationMetric::Boolean(valid_tool_call));
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "computercontroller_script"
    }
}

register_evaluation!("computercontroller", ComputerControllerScript);
