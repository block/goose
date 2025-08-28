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

#[derive(Debug)]
pub struct SimpleRepoCloneTest {}

impl SimpleRepoCloneTest {
    pub fn new() -> Self {
        SimpleRepoCloneTest {}
    }
}

#[async_trait]
impl Evaluation for SimpleRepoCloneTest {
    async fn run(
        &self,
        agent: &mut BenchAgent,
        _work_dir: &mut BenchmarkWorkDir,
    ) -> anyhow::Result<Vec<(String, EvalMetricValue)>> {
        // Send the prompt to clone the repo and add a test
        let (messages, perf_metrics) = collect_baseline_metrics(
            agent,
            "Clone the Git repository https://github.com/michaelneale/mcp-read-pdf to a temporary location. \
            Then add a new test file that verifies the PDF reading functionality. The test should \
            check if the PDF content can be read and processed correctly.".to_string(),
        ).await;

        // Convert HashMap to Vec for our metrics
        let mut metrics = metrics_hashmap_to_vec(perf_metrics);

        // Check for git clone operation
        let git_clone_executed = messages.iter().any(|msg| {
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
                                let command = args.get("command").and_then(Value::as_str);
                                command.is_some_and(|cmd| {
                                    cmd.contains("git clone")
                                        && cmd.contains("michaelneale/mcp-read-pdf")
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

        // Check for exploring the repository structure
        let repo_explored = messages.iter().any(|msg| {
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
                                let command = args.get("command").and_then(Value::as_str);
                                command.is_some_and(|cmd| {
                                    (cmd.contains("ls")
                                        || cmd.contains("find")
                                        || cmd.contains("rg"))
                                        && cmd.contains("mcp-read-pdf")
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

        // Check for file creation to add a test
        let test_added = messages.iter().any(|msg| {
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
                                let command = args.get("command").and_then(Value::as_str);
                                let file_text = args.get("file_text").and_then(Value::as_str);
                                let path = args.get("path").and_then(Value::as_str);

                                command == Some("write")
                                    && path.is_some_and(|p| {
                                        p.contains("test")
                                            || p.ends_with(".py")
                                            || p.ends_with(".js")
                                            || p.ends_with(".ts")
                                    })
                                    && file_text.is_some_and(|text| {
                                        text.contains("test")
                                            || text.contains("assert")
                                            || text.contains("expect")
                                            || text.contains("should")
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

        // Check if the agent ran the test
        let test_executed = messages.iter().any(|msg| {
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
                                let command = args.get("command").and_then(Value::as_str);
                                command.is_some_and(|cmd| {
                                    cmd.contains("test")
                                        || cmd.contains("pytest")
                                        || cmd.contains("jest")
                                        || cmd.contains("mocha")
                                        || (cmd.contains("node") && cmd.contains("test"))
                                        || (cmd.contains("python") && cmd.contains("test"))
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

        // Add metrics
        metrics.push((
            "Git repo cloned".to_string(),
            EvalMetricValue::Boolean(git_clone_executed),
        ));
        metrics.push((
            "Repository explored".to_string(),
            EvalMetricValue::Boolean(repo_explored),
        ));
        metrics.push((
            "Test file added".to_string(),
            EvalMetricValue::Boolean(test_added),
        ));
        metrics.push((
            "Test executed".to_string(),
            EvalMetricValue::Boolean(test_executed),
        ));
        metrics.push((
            "Complete task".to_string(),
            EvalMetricValue::Boolean(git_clone_executed && test_added),
        ));

        metrics.push((
            "score".to_string(),
            EvalMetricValue::Float(
                ((git_clone_executed as u8) + (test_added as u8) + (test_executed as u8)) as f64
                    / 3.0,
            ),
        ));

        Ok(metrics)
    }

    fn name(&self) -> &str {
        "simple_repo_clone_test"
    }

    fn required_extensions(&self) -> ExtensionRequirements {
        ExtensionRequirements {
            builtin: vec!["developer".to_string()],
            external: Vec::new(),
            remote: Vec::new(),
        }
    }
}

register_evaluation!(SimpleRepoCloneTest);
