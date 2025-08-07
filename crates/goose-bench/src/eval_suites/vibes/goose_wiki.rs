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
use std::fs;

pub struct GooseWiki {}

impl GooseWiki {
    pub fn new() -> Self {
        GooseWiki {}
    }

    fn check_html_implementation(&self, content: &str) -> bool {
        // Check for basic structure
        let has_basic_structure = content.contains("<html")
            && content.contains("</html>")
            && content.contains("<head")
            && content.contains("</head>")
            && content.contains("<body")
            && content.contains("</body>");

        // Check for Wikipedia-style content
        let has_wiki_elements = content.contains("<h1") && // Has headings
                              (content.contains("<h2") || content.contains("<h3")) && // Has subheadings
                              content.contains("Goose") && // Mentions Goose
                              content.contains("AI") && // Mentions AI
                              (content.contains("<p>") || content.contains("<div")); // Has paragraphs

        has_basic_structure && has_wiki_elements
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
            msg.role == Role::Assistant
                && msg.content.iter().any(|content| {
                    if let MessageContent::ToolRequest(tool_req) = content {
                        if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                            // Check tool name is correct
                            if tool_call.name != "developer__text_editor" {
                                return false;
                            }

                            // Parse the arguments as JSON
                            if let Ok(args) =
                                serde_json::from_value::<Value>(tool_call.arguments.clone())
                            {
                                // Only check command is write and correct filename
                                args.get("command").and_then(Value::as_str) == Some("write")
                                    && args
                                        .get("path")
                                        .and_then(Value::as_str)
                                        .is_some_and(|s| s.contains("index.html"))
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
            "used_write_tool".to_string(),
            EvalMetricValue::Boolean(valid_tool_call),
        ));

        let mut valid_implementation = false;
        // If tool was used correctly, check the actual file content
        if valid_tool_call {
            if let Ok(file_path) = _run_loc.fs_get("index.html".to_string()) {
                if let Ok(content) = fs::read_to_string(file_path) {
                    valid_implementation = self.check_html_implementation(&content);
                    metrics.push((
                        "valid_implementation".to_string(),
                        EvalMetricValue::Boolean(valid_implementation),
                    ));
                }
            }
        }

        metrics.push((
            "score".to_string(),
            EvalMetricValue::Float(
                ((valid_implementation as u8) + (valid_tool_call as u8)) as f64 / 2.0,
            ),
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
