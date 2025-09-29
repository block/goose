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

pub struct FlappyBird {}

impl FlappyBird {
    pub fn new() -> Self {
        FlappyBird {}
    }

    fn check_python_implementation(&self, content: &str) -> bool {
        content.contains("import pygame") &&
        content.contains("pygame.init()") &&
        content.contains("while") && // Game loop
        content.contains("pygame.event.get()") && // Event handling
        content.contains("def main") && // Main function
        content.contains("if __name__ == '__main__'") // Main guard
    }
}

#[async_trait]
impl Evaluation for FlappyBird {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        run_loc: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        println!("FlappyBird - run");

        // Collect baseline metrics (execution time, token usage, tool calls)
        let (messages, perf_metrics) = collect_baseline_metrics(
            agent,
            "Create a Flappy Bird game in Python. Structure the code with a main function and use the if __name__ == '__main__': idiom. You must use pygame. The background color should be a light blue color. Pressing SPACE multiple times will accelerate the bird. The bird's shape should be a red circle. Place on the bottom some land colored as dark yellow chosen. Make a score shown on the top right side. Increment if you pass pipes and don't hit them. Make randomly spaced dark green pipes with enough space. When you lose, show the best score. Make the text inside the screen. Pressing q or Esc will quit the game. Restarting is pressing SPACE again. When trying to run the game, make sure to use pyenv and create the environment in the current working directory. The final game should be written to a file named flappy_bird.py. Remember to use your tools if applicable.".to_string()
        ).await;

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

        // Check if the agent used the text editor tool correctly
        let valid_tool_call = messages.iter().any(|msg| {
            msg.role == Role::Assistant
                && msg.content.iter().any(|content| {
                    if let MessageContent::ToolRequest(tool_req) = content {
                        if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                            // Check tool name and basic parameters
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
                                        .is_some_and(|s| s.contains("flappy_bird.py"))
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

        // If tool was used correctly, check the actual file content
        let mut valid_implementation = false;
        if valid_tool_call {
            if let Ok(file_path) = run_loc.fs_get("flappy_bird.py".to_string()) {
                if let Ok(content) = fs::read_to_string(file_path) {
                    valid_implementation = self.check_python_implementation(&content);
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
        "flappy_bird"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: Vec::new(),
            remote: Vec::new(),
        }
    }
}

register_evaluation!(FlappyBird);
