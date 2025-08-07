use crate::bench_session::BenchAgent;
use crate::eval_suites::EvalMetricValue;
use goose::conversation::message::{Message, MessageContent};
use goose::conversation::Conversation;
use std::collections::HashMap;
use std::time::Instant;

/// Collect baseline metrics including execution time, tool usage, and token count
pub async fn collect_baseline_metrics(
    agent: &mut BenchAgent,
    prompt: String,
) -> (Conversation, HashMap<String, EvalMetricValue>) {
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
                EvalMetricValue::String(format!("Error: {}", e)),
            );
            Conversation::new_unvalidated(Vec::new())
        }
    };

    // Calculate execution time
    let execution_time = start_time.elapsed();
    metrics.insert(
        "prompt_execution_time_seconds".to_string(),
        EvalMetricValue::Float(execution_time.as_secs_f64()),
    );

    // Count tool calls
    let (total_tool_calls, tool_calls_by_name) = count_tool_calls(messages.messages());
    metrics.insert(
        "total_tool_calls".to_string(),
        EvalMetricValue::Integer(total_tool_calls),
    );

    // Add tool calls by name metrics
    for (tool_name, count) in tool_calls_by_name {
        metrics.insert(
            format!("tool_calls_{}", tool_name),
            EvalMetricValue::Integer(count),
        );
    }

    // Get token usage information if available
    if let Some(token_count) = agent.get_token_usage().await {
        metrics.insert(
            "total_tokens".to_string(),
            EvalMetricValue::Integer(token_count as i64),
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
    metrics: HashMap<String, EvalMetricValue>,
) -> Vec<(String, EvalMetricValue)> {
    metrics.into_iter().collect()
}

/// Check if a specific tool was used in any of the messages
pub fn used_tool(messages: &[Message], tool_name: &str) -> bool {
    messages.iter().any(|msg| {
        msg.content.iter().any(|content| {
            if let MessageContent::ToolRequest(tool_req) = content {
                if let Ok(tool_call) = tool_req.tool_call.as_ref() {
                    tool_call.name.contains(tool_name)
                } else {
                    false
                }
            } else {
                false
            }
        })
    })
}
