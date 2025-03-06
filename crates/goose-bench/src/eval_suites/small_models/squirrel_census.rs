use crate::eval_suites::{BenchAgent, Evaluation, EvaluationMetric};
use crate::register_evaluation;
use crate::work_dir::WorkDir;
use async_trait::async_trait;
use goose::message::MessageContent;
use mcp_core::role::Role;
use serde_json::{self, Value};

pub struct SquirrelCensus {}

impl SquirrelCensus {
    pub fn new() -> Self {
        SquirrelCensus {}
    }

    fn check_analysis_results(&self, text: &str) -> (bool, bool, bool) {
        let has_central_manhattan = text.contains("CENTRAL MANHATTAN") && text.contains("174");
        let has_tompkins = text.contains("Tompkins Square Park") && text.contains("59");
        let has_gray = text.to_lowercase().contains("gray") || text.to_lowercase().contains("grey");
        (has_central_manhattan, has_tompkins, has_gray)
    }
}

#[async_trait]
impl Evaluation for SquirrelCensus {
    async fn run(
        &self,
        mut agent: Box<dyn BenchAgent>,
        work_dir: &mut WorkDir,
    ) -> anyhow::Result<Vec<(String, EvaluationMetric)>> {
        println!("SquirrelCensus - run");
        let mut metrics = Vec::new();

        // Get the path to the squirrel data file
        let squirrel_data_path = work_dir.path.join("assets").join("squirrel-data.csv");
        if !squirrel_data_path.exists() {
            return Err(anyhow::anyhow!("Could not find squirrel-data.csv file"));
        }
        
        let messages = agent.prompt(format!(
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

After writing the script, run it using python3 and show the results. Do not ask for confirmation or further instructions.", 
            squirrel_data_path.display()
        )).await?;

        // Check if agent wrote the Python script
        let wrote_script = messages.iter().any(|msg| {
            msg.role == Role::Assistant &&
            msg.content.iter().any(|content| {
                if let MessageContent::ToolRequest(tool_req) = content {
                    if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                        if tool_call.name != "developer__text_editor" {
                            return false;
                        }

                        if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
                            args.get("command").and_then(Value::as_str) == Some("write") &&
                            args.get("path").and_then(Value::as_str).is_some_and(|s| s.contains("analyze_squirrels.py"))
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
            msg.role == Role::Assistant &&
            msg.content.iter().any(|content| {
                if let MessageContent::ToolRequest(tool_req) = content {
                    if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                        if tool_call.name != "developer__shell" {
                            return false;
                        }

                        if let Ok(args) = serde_json::from_value::<Value>(tool_call.arguments.clone()) {
                            args.get("command").and_then(Value::as_str).is_some_and(|s| 
                                s.contains("python") && s.contains("analyze_squirrels.py"))
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
            let msg_text = last_msg.to_string();
            let (has_central_manhattan, has_tompkins, has_gray) = self.check_analysis_results(&msg_text);
            has_central_manhattan && has_tompkins && has_gray
        } else {
            false
        };

        metrics.push(("wrote_script".to_string(), 
            EvaluationMetric::Boolean(wrote_script)));
        metrics.push(("ran_script".to_string(), 
            EvaluationMetric::Boolean(ran_script)));
        metrics.push(("correct_results".to_string(),
            EvaluationMetric::Boolean(correct_results)));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "squirrel_census"
    }

    fn required_extensions(&self) -> Vec<String> {
        vec!["developer".to_string()]
    }
}

register_evaluation!("small_models", SquirrelCensus);