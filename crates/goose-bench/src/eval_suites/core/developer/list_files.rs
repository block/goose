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
        agent: &mut BenchAgent,
        _run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        // Send the prompt to list files
        let (messages, perf_metrics) =
            collect_baseline_metrics(agent, "list the files in the current directory".to_string())
                .await;

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

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
            EvalMetricValue::Boolean(valid_tool_call),
        ));

        metrics.push((
            "score".to_string(),
            EvalMetricValue::Float((valid_tool_call as u8) as f64 / 1.0),
        ));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "developer_list_files"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: Vec::new(),
            remote: Vec::new(),
        }
    }
}

register_evaluation!(DeveloperListFiles);
