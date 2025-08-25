use crate::conversation::message::{Message, ToolRequest};
use crate::providers::base::Provider;
use crate::tool_inspection::{InspectionAction, InspectionResult, ToolInspector};
use anyhow::Result;
use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    name: String,
    parameters: serde_json::Value,
}

impl ToolCall {
    pub fn new(name: String, parameters: serde_json::Value) -> Self {
        Self { name, parameters }
    }

    fn matches(&self, other: &ToolCall) -> bool {
        self.name == other.name && self.parameters == other.parameters
    }
}

#[derive(Debug)]
pub struct ToolMonitor {
    max_repetitions: Option<u32>,
    last_call: Option<ToolCall>,
    repeat_count: u32,
    call_counts: HashMap<String, u32>,
}

impl ToolMonitor {
    pub fn new(max_repetitions: Option<u32>) -> Self {
        Self {
            max_repetitions,
            last_call: None,
            repeat_count: 0,
            call_counts: HashMap::new(),
        }
    }

    pub fn check_tool_call(&mut self, tool_call: ToolCall) -> bool {
        let total_calls = self.call_counts.entry(tool_call.name.clone()).or_insert(0);
        *total_calls += 1;

        if self.max_repetitions.is_none() {
            self.last_call = Some(tool_call);
            self.repeat_count = 1;
            return true;
        }

        if let Some(last) = &self.last_call {
            if last.matches(&tool_call) {
                self.repeat_count += 1;
                if self.repeat_count > self.max_repetitions.unwrap() {
                    return false;
                }
            } else {
                self.repeat_count = 1;
            }
        } else {
            self.repeat_count = 1;
        }

        self.last_call = Some(tool_call);
        true
    }

    pub fn get_stats(&self) -> HashMap<String, u32> {
        self.call_counts.clone()
    }

    pub fn reset(&mut self) {
        self.last_call = None;
        self.repeat_count = 0;
        self.call_counts.clear();
    }
}

#[async_trait]
impl ToolInspector for ToolMonitor {
    fn name(&self) -> &'static str {
        "repetition_monitor"
    }

    async fn inspect(
        &self,
        tool_requests: &[ToolRequest],
        _messages: &[Message],
        _provider: Option<Arc<dyn Provider>>,
    ) -> Result<Vec<InspectionResult>> {
        let mut results = Vec::new();

        for tool_request in tool_requests {
            if let Ok(tool_call) = &tool_request.tool_call {
                let tool_call_info =
                    ToolCall::new(tool_call.name.clone(), tool_call.arguments.clone());

                // Create a temporary clone to check without modifying state
                let mut temp_monitor = ToolMonitor::new(self.max_repetitions);
                temp_monitor.last_call = self.last_call.clone();
                temp_monitor.repeat_count = self.repeat_count;
                temp_monitor.call_counts = self.call_counts.clone();

                if !temp_monitor.check_tool_call(tool_call_info) {
                    results.push(InspectionResult {
                        tool_request_id: tool_request.id.clone(),
                        action: InspectionAction::Deny,
                        reason: format!(
                            "Tool '{}' has exceeded maximum repetitions",
                            tool_call.name
                        ),
                        confidence: 1.0,
                        inspector_name: "repetition_monitor".to_string(),
                        finding_id: Some("REP-001".to_string()),
                    });
                } else {
                    results.push(InspectionResult {
                        tool_request_id: tool_request.id.clone(),
                        action: InspectionAction::Allow,
                        reason: "Tool repetition within limits".to_string(),
                        confidence: 1.0,
                        inspector_name: "repetition_monitor".to_string(),
                        finding_id: None,
                    });
                }
            }
        }

        Ok(results)
    }

    fn priority(&self) -> u32 {
        150 // Medium priority - runs after security
    }
}
