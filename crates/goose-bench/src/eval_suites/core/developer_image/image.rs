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
pub struct DeveloperImage {}

impl DeveloperImage {
    pub fn new() -> Self {
        DeveloperImage {}
    }
}

#[async_trait]
impl Evaluation for DeveloperImage {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        _run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        // Send the prompt to list files
        let (messages, perf_metrics) = collect_baseline_metrics(
            agent,
            "Take a screenshot of the display 0 and describe what you see.".to_string(),
        )
        .await;

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

        // Check if the assistant makes appropriate tool calls and gets valid responses
        let mut valid_tool_call = false;
        let mut valid_response = false;

        for msg in messages.iter() {
            // Check for valid tool request
            if msg.role == Role::Assistant {
                for content in msg.content.iter() {
                    if let MessageContent::ToolRequest(tool_req) = content {
                        if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                            if let Ok(args) =
                                serde_json::from_value::<Value>(tool_call.arguments.clone())
                            {
                                if tool_call.name == "developer__screen_capture"
                                    && (args.get("display").and_then(Value::as_i64) == Some(0))
                                {
                                    valid_tool_call = true;
                                }
                            }
                        }
                    }
                }
            }

            // Check for valid tool response
            if msg.role == Role::User && valid_tool_call {
                for content in msg.content.iter() {
                    if let MessageContent::ToolResponse(tool_resp) = content {
                        if let Ok(result) = &tool_resp.tool_result {
                            // Check each item in the result list
                            for item in result {
                                if let Some(image) = item.as_image() {
                                    // Image content already contains mime_type and data
                                    if image.mime_type.starts_with("image/")
                                        && !image.data.is_empty()
                                    {
                                        valid_response = true;
                                        break; // Found a valid image, no need to check further
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        // Both the tool call and response must be valid
        metrics.push((
            "Take a screenshot and upload images".to_string(),
            EvalMetricValue::Boolean(valid_tool_call && valid_response),
        ));
        Ok(metrics)
    }

    fn name(&self) -> &str {
        "developer_image"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: Vec::new(),
            remote: Vec::new(),
        }
    }
}

register_evaluation!(DeveloperImage);
