use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{
    collect_baseline_metrics, copy_session_to_cwd, metrics_hashmap_to_vec, write_response_to_file,
    BenchAgent, Evaluation, EvaluationMetric, ExtensionRequirements,
};
use crate::register_evaluation;
use async_trait::async_trait;

pub struct BlogSummary {}

impl BlogSummary {
    pub fn new() -> Self {
        BlogSummary {}
    }

    fn check_markdown_numbered_list(&self, text: &str) -> bool {
        // Check if all numbers 1-5 exist in markdown numbered list format
        (1..=5).all(|n| text.contains(&format!("{}.", n)))
    }
}

#[async_trait]
impl Evaluation for BlogSummary {
    async fn run(
        &self,
        mut agent: Box<dyn BenchAgent>,
        work_dir: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        println!("BlogSummary - run");

        // Collect baseline metrics (execution time, token usage, tool calls)
        let (response, perf_metrics) = collect_baseline_metrics(
            &mut agent,
            "What are the top 5 most counterintuitive insights from this blog post? Format your response in Markdown with 5 numbered points (1. 2. 3. 4. 5.) https://huyenchip.com/2025/01/07/agents.html".to_string()
        ).await;

        // Write response to file and get the text content
        let response_text =
            match write_response_to_file(&response, work_dir, "blog_summary_output.txt") {
                Ok(text) => text,
                Err(e) => {
                    println!("Warning: Failed to write blog summary output: {}", e);
                    // If file write fails, still continue with the evaluation
                    response
                        .last()
                        .map_or_else(String::new, |msg| msg.as_concat_text())
                }
            };

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

        // Check if the content follows the markdown numbered list format
        let has_markdown_list = self.check_markdown_numbered_list(&response_text);
        metrics.push((
            "valid_markdown_format".to_string(),
            EvaluationMetric::Boolean(has_markdown_list),
        ));

        // Check if the fetch tool was used
        let used_fetch_tool = crate::eval_suites::used_tool(&response, "fetch");
        metrics.push((
            "used_fetch_tool".to_string(),
            EvaluationMetric::Boolean(used_fetch_tool),
        ));

        // Copy the session file to the current working directory
        if let Err(e) = copy_session_to_cwd() {
            println!("Warning: Failed to copy session file: {}", e);
        } else {
            println!("Successfully copied session file to current directory");
        }

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "blog_summary"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: vec!["uvx mcp-server-fetch".to_string()],
            remote: Vec::new(),
        }
    }
}

register_evaluation!(BlogSummary);
