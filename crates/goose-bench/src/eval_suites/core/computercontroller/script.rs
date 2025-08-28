// Create a new file called test.txt with the content 'Hello, World!

use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{
    collect_baseline_metrics, metrics_hashmap_to_vec, EvalMetricValue, Evaluation,
    ExtensionRequirements,
};
use crate::register_evaluation;
use async_trait::async_trait;
use goose::conversation::message::MessageContent;
use rmcp::model::Role;
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
        agent: &mut BenchAgent,
        _run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        // Send the prompt to list files
        let (messages, perf_metrics) =
            collect_baseline_metrics(agent, "Make a beep sound".to_string()).await;

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

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
            EvalMetricValue::Boolean(valid_tool_call),
        ));
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "computercontroller_script"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["computercontroller".to_string()],
            external: Vec::new(),
            remote: Vec::new(),
        }
    }
}

register_evaluation!(ComputerControllerScript);
