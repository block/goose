use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric, ExtensionRequirements};
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
        _: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        println!("BlogSummary - run");
        let mut metrics = Vec::new();
        let response = agent.prompt("What are the top 5 most counterintuitive insights from this blog post? Format your response in Markdown with 5 numbered points (1. 2. 3. 4. 5.) https://huyenchip.com/2025/01/07/agents.html".to_string()).await?;

        // Get text content from the last message
        let has_markdown_list = if let Some(last_msg) = response.last() {
            self.check_markdown_numbered_list(&last_msg.as_concat_text())
        } else {
            false
        };

        metrics.push((
            "valid_markdown_format".to_string(),
            EvaluationMetric::Boolean(has_markdown_list),
        ));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "blog_summary"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: vec!["fetch".to_string()],
        }
    }
}

register_evaluation!("small_models_fetch", BlogSummary);
