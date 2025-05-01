use crate::bench_session::BenchAgent;
use crate::bench_work_dir::BenchmarkWorkDir;
use crate::eval_suites::{
  collect_baseline_metrics, metrics_hashmap_to_vec, write_response_to_file, EvalMetricValue, Evaluation, ExtensionRequirements
};
use crate::register_evaluation;
use async_trait::async_trait;

pub struct PdfSummaryEvaluation {}

impl PdfSummaryEvaluation {
    pub fn new() -> Self {
        PdfSummaryEvaluation {}
    }
}

#[async_trait]
impl Evaluation for PdfSummaryEvaluation {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        println!("PdfSummaryEvaluation - run");

        // The prompt to summarize the PDF article about Google's monorepo
        let prompt = "There is a pdf about the google monorepo, in the `../../../assets/` directory which I believe is one layer in a sub directory. Please summarize it for me.".to_string();

        // Collect baseline metrics with the prompt
        let (messages, perf_metrics) = collect_baseline_metrics(
            agent,
            prompt
        ).await;


        // println!("messages {:?}", messages);
        // println!("metrics {:?}", perf_metrics);
        
        // Write response to file
        let _response_text =
            match write_response_to_file(&messages, run_loc, "pdf_summary_output.txt") {
                Ok(text) => text,
                Err(e) => {
                    println!("Warning: Failed to write pdf_summary_output: {}", e);
                    // If file write fails, still continue with the evaluation
                    messages
                        .last()
                        .map_or_else(String::new, |msg| msg.as_concat_text())
                }
            };

        // Prepare metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

        // Simple success indicator for now - we'll add validation later
        metrics.push(("completed_summary".to_string(), EvalMetricValue::Boolean(true)));
        metrics.push(("summary_size_gt_zero".to_string(), EvalMetricValue::Boolean(_response_text.len() > 0)));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "pdf_summary"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
      ExtensionRequirements {
          builtin: vec!["developer".to_string()],
          external: Vec::new(),
          remote: Vec::new(),
      }
  }
}

register_evaluation!(PdfSummaryEvaluation);