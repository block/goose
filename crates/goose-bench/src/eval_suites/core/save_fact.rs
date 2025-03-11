// Create a new file called test.txt with the content 'Hello, World!

use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric, ExtensionRequirements};
use crate::register_evaluation;
use async_trait::async_trait;
use goose::message::MessageContent;
use mcp_core::role::Role;
use serde_json::{self, Value};

#[derive(Debug)]
pub struct MemoryRememberMemory {}

impl MemoryRememberMemory {
    pub fn new() -> Self {
        MemoryRememberMemory {}
    }
}

#[async_trait]
impl Evaluation for MemoryRememberMemory {
    async fn run(
        &self,
        mut agent: Box<dyn BenchAgent>,
        _work_dir: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        let mut metrics = Vec::new();

        // Send the prompt to list files
        let messages = agent.prompt("Save this fact: The capital of France is Paris.".to_string());
        let messages = messages.await?;

        let valid_tool_call = messages.iter().any(|msg| {
            // Check if it's an assistant message
            msg.role == Role::Assistant &&
                // Check if any content item is a tool request for creating a file
                msg.content.iter().any(|content| {
                    if let MessageContent::ToolRequest(tool_req) = content {
                        if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                            // Check tool name is correct
                            if tool_call.name != "memory__remember_memory" {
                                return false;
                            }

                            // Parse the arguments as JSON
                            if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
                                // Check all required parameters match exactly
                                args.get("category").and_then(Value::as_str).is_some_and(|s| s.contains("fact")) &&
                                    args.get("data").and_then(Value::as_str) == Some("The capital of France is Paris.") &&
                                    args.get("is_global").and_then(Value::as_bool) == Some(true)
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
            "Saving facts".to_string(),
            EvaluationMetric::Boolean(valid_tool_call),
        ));
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "memory_remember_memory"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["memory".to_string()],
            external: Vec::new(),
        }
    }
}

register_evaluation!("memory", MemoryRememberMemory);
