use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{
    collect_baseline_metrics, metrics_hashmap_to_vec, EvalMetricValue, Evaluation,
    ExtensionRequirements,
};
use crate::register_evaluation;
use async_trait::async_trait;
use goose::message::MessageContent;
use mcp_core::role::Role;
use serde_json::{self, Value};

pub struct GooseWiki {}

impl GooseWiki {
    pub fn new() -> Self {
        GooseWiki {}
    }
}

#[async_trait]
impl Evaluation for GooseWiki {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        _run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        println!("GooseWiki - run");

        // Collect baseline metrics (execution time, token usage, tool calls)
        let (messages, perf_metrics) = collect_baseline_metrics(
            agent,
            "Create a Wikipedia-style web page about Goose (Block's AI agent) in a new index.html file. The page should be a complete, well-structured HTML document with proper head and body sections. Use heading tags (h1, h2, h3) to organize the content into clear sections. Include comprehensive information about Goose organized in a way similar to how Wikipedia presents technical topics. Remember to use your tools if applicable.".to_string()
        ).await;

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

        // Check if the agent used the text editor tool to create index.html
        let valid_tool_call = messages.iter().any(|msg| {
            msg.role == Role::Assistant &&
            msg.content.iter().any(|content| {
                if let MessageContent::ToolRequest(tool_req) = content {
                    if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                        // Check tool name is correct
                        if tool_call.name != "developer__text_editor" {
                            return false;
                        }

                        // Parse the arguments as JSON
                        if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
                            // Check command is write and path contains index.html
                            args.get("command").and_then(Value::as_str) == Some("write") &&
                            args.get("path").and_then(Value::as_str).is_some_and(|s| s.contains("index.html")) &&
                            // Verify file_text contains basic HTML structure
                            args.get("file_text").and_then(Value::as_str).is_some_and(|s| {
                                s.contains("<html") && s.contains("</html>") &&
                                s.contains("<head") && s.contains("</head>") &&
                                s.contains("<body") && s.contains("</body>")
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
            "created_valid_html".to_string(),
            EvalMetricValue::Boolean(valid_tool_call),
        ));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "goose_wiki"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: Vec::new(),
            remote: Vec::new(),
        }
    }
}

register_evaluation!(GooseWiki);
