use crate::eval_suites::{BenchAgent, EvaluationMetric};
use goose::message::{Message, MessageContent};
use std::collections::HashMap;
use std::time::Instant;

/// Helper function to measure execution time of agent.prompt()
pub async fn measure_prompt_execution_time(
    agent: &mut Box<dyn BenchAgent>,
    prompt: String,
) -> (Vec<Message>, HashMap<String, EvaluationMetric>) {
    // Initialize metrics map
    let mut metrics = HashMap::new();

    // Start timer
    let start_time = Instant::now();

    // Execute prompt
    let messages = match agent.prompt(prompt).await {
        Ok(msgs) => msgs,
        Err(e) => {
            metrics.insert(
                "prompt_error".to_string(),
                EvaluationMetric::String(format!("Error: {}", e)),
            );
            Vec::new()
        }
    };

    // Calculate execution time
    let execution_time = start_time.elapsed();
    metrics.insert(
        "prompt_execution_time_seconds".to_string(),
        EvaluationMetric::Float(execution_time.as_secs_f64()),
    );

    // Count tool calls
    let (total_tool_calls, tool_calls_by_name) = count_tool_calls(&messages);
    metrics.insert(
        "total_tool_calls".to_string(),
        EvaluationMetric::Integer(total_tool_calls),
    );

    // Add tool calls by name metrics
    for (tool_name, count) in tool_calls_by_name {
        metrics.insert(
            format!("tool_calls_{}", tool_name),
            EvaluationMetric::Integer(count),
        );
    }

    (messages, metrics)
}

/// Count all tool calls in messages and categorize by tool name
fn count_tool_calls(messages: &[Message]) -> (i64, HashMap<String, i64>) {
    let mut total_count = 0;
    let mut counts_by_name = HashMap::new();

    for message in messages {
        for content in &message.content {
            if let MessageContent::ToolRequest(tool_req) = content {
                if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                    total_count += 1;

                    // Count by name
                    *counts_by_name.entry(tool_call.name.clone()).or_insert(0) += 1;
                }
            }
        }
    }

    (total_count, counts_by_name)
}

/// Convert HashMap of metrics to Vec
pub fn metrics_hashmap_to_vec(
    metrics: HashMap<String, EvaluationMetric>,
) -> Vec<(String, EvaluationMetric)> {
    metrics.into_iter().collect()
}
