use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{
    collect_baseline_metrics, metrics_hashmap_to_vec, write_response_to_file, EvalMetricValue,
    Evaluation, ExtensionRequirements,
};
use crate::register_evaluation;
use async_trait::async_trait;

pub struct RestaurantResearch {}

impl RestaurantResearch {
    pub fn new() -> Self {
        RestaurantResearch {}
    }

    fn check_markdown_bullets(&self, text: &str) -> bool {
        // Check if there's at least one bullet point and proper markdown formatting
        text.contains("- ") || text.contains("* ")
    }

    fn count_bullet_points(&self, text: &str) -> i64 {
        // Count total bullet points (either - or * style)
        let dash_bullets = text.matches("- ").count();
        let star_bullets = text.matches("* ").count();
        (dash_bullets + star_bullets) as i64
    }
}

#[async_trait]
impl Evaluation for RestaurantResearch {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        println!("RestaurantResearch - run");

        // Collect baseline metrics (execution time, token usage, tool calls)
        let (response, perf_metrics) = collect_baseline_metrics(
            agent,
            "Search the internet for and provide a current, detailed list of the best Sichuanese restaurants specifically in the East Village neighborhood of NYC. Format your response in Markdown using bullet points (either - or *) for each restaurant. For each restaurant include:
- Restaurant name and what they're known for
- Signature dishes
- Atmosphere/setting
- Any relevant details about reservations or dining experience
- What distinguishes them from others

Present the information in order of significance or quality. Focus specifically on Sichuanese establishments, not general Chinese restaurants. If you encounter a page you cannot access, try another one. Do not ask me for confirmation just conduct the searches yourself until you find the needed information. Remember to use your tools if applicable.".to_string()
        ).await;

        // Write response to file and get the text content
        let response_text = match write_response_to_file(
            response.messages(),
            run_loc,
            "restaurant_research_output.txt",
        ) {
            Ok(text) => text,
            Err(e) => {
                println!("Warning: Failed to write restaurant research output: {}", e);
                // If file write fails, still continue with the evaluation
                response
                    .last()
                    .map_or_else(String::new, |msg| msg.as_concat_text())
            }
        };

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

        // Check markdown formatting
        let has_markdown_bullets = self.check_markdown_bullets(&response_text);
        let bullet_count = self.count_bullet_points(&response_text);

        metrics.push((
            "valid_markdown_format".to_string(),
            EvalMetricValue::Boolean(has_markdown_bullets),
        ));
        metrics.push((
            "bullet_point_count".to_string(),
            EvalMetricValue::Integer(bullet_count),
        ));

        // Check if the fetch tool was used
        let used_fetch_tool = crate::eval_suites::used_tool(response.messages(), "fetch");
        metrics.push((
            "used_fetch_tool".to_string(),
            EvalMetricValue::Boolean(used_fetch_tool),
        ));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "restaurant_research"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: vec!["uvx mcp-server-fetch".to_string()],
            remote: Vec::new(),
        }
    }
}

register_evaluation!(RestaurantResearch);
