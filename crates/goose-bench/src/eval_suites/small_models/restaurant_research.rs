use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric};
use crate::register_evaluation;
use crate::work_dir::WorkDir;
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
        mut agent: Box<dyn BenchAgent>,
        _: &mut WorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        println!("RestaurantResearch - run");
        let mut metrics = Vec::new();
        let response = agent.prompt("Search for and provide a current, detailed list of the best Sichuanese restaurants specifically in the East Village neighborhood of NYC. Format your response in Markdown using bullet points (either - or *) for each restaurant. For each restaurant include:
- Restaurant name and what they're known for
- Signature dishes
- Atmosphere/setting
- Any relevant details about reservations or dining experience
- What distinguishes them from others

Present the information in order of significance or quality. Focus specifically on Sichuanese establishments, not general Chinese restaurants.".to_string()).await?;

        // Get text content from the last message
        if let Some(last_msg) = response.last() {
            let text_content = last_msg.as_concat_text();
            let has_markdown_bullets = self.check_markdown_bullets(&text_content);
            let bullet_count = self.count_bullet_points(&text_content);

            metrics.push(("valid_markdown_format".to_string(), 
                EvaluationMetric::Boolean(has_markdown_bullets)));
            metrics.push(("bullet_point_count".to_string(), 
                EvaluationMetric::Integer(bullet_count)));
        }

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "restaurant_research"
    }

    fn required_extensions(&self) -> Vec<String> {
        vec!["developer".to_string(), "fetch".to_string()]
    }
}

register_evaluation!("small_models_fetch", RestaurantResearch);