use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric, ExtensionRequirements};
use crate::register_evaluation;
use crate::work_dir::WorkDir;
use async_trait::async_trait;
use goose::message::MessageContent;
use mcp_core::content::Content;
use mcp_core::role::Role;
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
        mut agent: Box<dyn BenchAgent>,
        _work_dir: &mut WorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        let mut metrics = Vec::new();

        // Send the prompt to list files
        let messages = agent
            .prompt("Take a screenshot of the display 0 and describe what you see.".to_string())
            .await?;

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
                                if let Content::Image(image) = item {
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
            EvaluationMetric::Boolean(valid_tool_call && valid_response),
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
        }
    }
}

register_evaluation!("developer_image", DeveloperImage);
