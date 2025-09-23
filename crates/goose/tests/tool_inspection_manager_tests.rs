use goose::tool_inspection::{InspectionAction, InspectionResult, ToolInspectionManager, ToolInspector};
use goose::conversation::message::{Message, ToolRequest};
use anyhow::{anyhow, Result};
use async_trait::async_trait;

struct MockInspectorOk {
    name: &'static str,
    results: Vec<InspectionResult>,
}

struct MockInspectorErr {
    name: &'static str,
}

#[async_trait]
impl ToolInspector for MockInspectorOk {
    fn name(&self) -> &'static str { self.name }
    fn as_any(&self) -> &dyn std::any::Any { self }
    async fn inspect(&self, _tool_requests: &[ToolRequest], _messages: &[Message]) -> Result<Vec<InspectionResult>> {
        Ok(self.results.clone())
    }
}

#[async_trait]
impl ToolInspector for MockInspectorErr {
    fn name(&self) -> &'static str { self.name }
    fn as_any(&self) -> &dyn std::any::Any { self }
    async fn inspect(&self, _tool_requests: &[ToolRequest], _messages: &[Message]) -> Result<Vec<InspectionResult>> {
        Err(anyhow!("simulated failure"))
    }
}

#[tokio::test]
async fn test_inspect_tools_aggregates_and_handles_errors() {
    // Arrange: create a manager with one successful and one failing inspector
    let ok_results = vec![
        InspectionResult {
            tool_request_id: "req_1".to_string(),
            action: InspectionAction::Allow,
            reason: "looks safe".to_string(),
            confidence: 0.95,
            inspector_name: "ok".to_string(),
            finding_id: None,
        },
        InspectionResult {
            tool_request_id: "req_2".to_string(),
            action: InspectionAction::RequireApproval(Some("double check".to_string())),
            reason: "needs user confirmation".to_string(),
            confidence: 0.7,
            inspector_name: "ok".to_string(),
            finding_id: Some("FND-123".to_string()),
        },
    ];

    let mut manager = ToolInspectionManager::new();
    manager.add_inspector(Box::new(MockInspectorOk { name: "ok", results: ok_results.clone() }));
    manager.add_inspector(Box::new(MockInspectorErr { name: "err" }));

    // No specific input is required for this aggregation behavior
    let tool_requests: Vec<ToolRequest> = vec![];
    let messages: Vec<Message> = vec![];

    // Act
    let results = manager.inspect_tools(&tool_requests, &messages).await.expect("inspect_tools should not fail when one inspector errors");

    // Assert: results from the successful inspector are returned; failing inspector is ignored
    assert_eq!(results.len(), 2, "Should aggregate results from successful inspectors only");
    // Also verify inspector_names() order/presence
    let names = manager.inspector_names();
    assert_eq!(names, vec!["ok", "err"], "Inspector names should reflect registration order");

    // Verify that specific actions are preserved
    assert!(results.iter().any(|r| matches!(r.action, InspectionAction::Allow)));
    assert!(results.iter().any(|r| matches!(r.action, InspectionAction::RequireApproval(_))));
}
