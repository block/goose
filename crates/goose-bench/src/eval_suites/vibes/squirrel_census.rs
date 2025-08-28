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

pub struct SquirrelCensus {}

impl SquirrelCensus {
    pub fn new() -> Self {
        SquirrelCensus {}
    }

    fn check_analysis_results(&self, text: &str) -> (bool, bool, bool) {
        let text_lower = text.to_lowercase();
        let has_central_manhattan =
            text_lower.contains("central manhattan") && text.contains("174");
        let has_tompkins = text_lower.contains("tompkins square park") && text.contains("59");
        let has_gray = text_lower.contains("gray") || text_lower.contains("grey");
        (has_central_manhattan, has_tompkins, has_gray)
    }
}

#[async_trait]
impl Evaluation for SquirrelCensus {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        println!("SquirrelCensus - run");

        // Get the path to the squirrel data file
        let squirrel_data_path = match run_loc.fs_get("./assets/squirrel-data.csv".to_string()) {
            Ok(file) => file,
            Err(_) => return Err(anyhow::anyhow!("Could not find squirrel-data.csv file")),
        };

        println!("squirrel_data_path: {:?}", squirrel_data_path);

        // Collect baseline metrics (execution time, token usage, tool calls)
        let (messages, perf_metrics) = collect_baseline_metrics(
            agent,
            format!(
                "Create a Python script called analyze_squirrels.py that analyzes the CSV file at {}. Do not ask for any clarification or further instructions - proceed with the implementation as specified below.

The script should use pandas to answer these specific questions:
1. Which area (Area column) has the most squirrels spotted? For this area, what is the most common Primary Fur Color of squirrels?
2. Which specific park location (Park Name column) has the most squirrels spotted? For this location, what is the most common Primary Fur Color of squirrels?

The script should:
- Use pandas to read and analyze the data
- Print results in EXACTLY this format (including the markers):
  [AREA_RESULT] <area_name> - <count> squirrels spotted
  [AREA_COLOR] Most common fur color: <color> (<color_count> squirrels)
  [PARK_RESULT] <park_name> - <count> squirrels spotted
  [PARK_COLOR] Most common fur color: <color> (<color_count> squirrels)

After writing the script, run it using python3 and show the results. Do not ask for confirmation or further instructions. Remember to use your tools if applicable.", 
                squirrel_data_path.display()
            )
        ).await;

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

        // Check if agent wrote the Python script
        let wrote_script = messages.iter().any(|msg| {
            msg.role == Role::Assistant
                && msg.content.iter().any(|content| {
                    if let MessageContent::ToolRequest(tool_req) = content {
                        if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                            if tool_call.name != "developer__text_editor" {
                                return false;
                            }

                            if let Ok(args) =
                                serde_json::from_value::<Value>(tool_call.arguments.clone())
                            {
                                args.get("command").and_then(Value::as_str) == Some("write")
                                    && args
                                        .get("path")
                                        .and_then(Value::as_str)
                                        .is_some_and(|s| s.contains("analyze_squirrels.py"))
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

        // Check if agent ran the script
        let ran_script = messages.iter().any(|msg| {
            msg.role == Role::Assistant
                && msg.content.iter().any(|content| {
                    if let MessageContent::ToolRequest(tool_req) = content {
                        if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                            if tool_call.name != "developer__shell" {
                                return false;
                            }

                            if let Ok(args) =
                                serde_json::from_value::<Value>(tool_call.arguments.clone())
                            {
                                args.get("command")
                                    .and_then(Value::as_str)
                                    .is_some_and(|s| {
                                        s.contains("python") && s.contains("analyze_squirrels.py")
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

        // Check the last message for correct results
        let correct_results = if let Some(last_msg) = messages.last() {
            let text_content = last_msg.as_concat_text();
            let (has_central_manhattan, has_tompkins, has_gray) =
                self.check_analysis_results(&text_content);
            has_central_manhattan && has_tompkins && has_gray
        } else {
            false
        };

        metrics.push((
            "wrote_script".to_string(),
            EvalMetricValue::Boolean(wrote_script),
        ));
        metrics.push((
            "ran_script".to_string(),
            EvalMetricValue::Boolean(ran_script),
        ));

        metrics.push((
            "score".to_string(),
            EvalMetricValue::Float((correct_results as u8) as f64 / 1.0),
        ));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "squirrel_census"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: Vec::new(),
            remote: Vec::new(),
        }
    }
}

register_evaluation!(SquirrelCensus);
