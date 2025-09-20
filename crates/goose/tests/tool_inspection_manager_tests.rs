use goose::tool_inspection::{InspectionAction, InspectionResult, ToolInspectionManager, ToolInspector};
use goose::conversation::message::{Message, ToolRequest};
use anyhow::Result;
use async_trait::async_trait;

struct MockInspector {
    name: &'static str,
    results: Vec<InspectionResult>,
    enabled: bool,
}

#[async_trait]
impl ToolInspector for MockInspector {
    fn name(&self) -> &'static str {
        self.name
    }

    fn is_enabled(&self) -> bool {
        self.enabled
    }

    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    async fn inspect(
        &self,
        _tool_requests: &[ToolRequest],
        _messages: &[Message],
    ) -> Result<Vec<InspectionResult>> {
        Ok(self.results.clone())
    }
}

#[tokio::test]
async fn test_inspect_tools_aggregates_results_and_skips_disabled() {
    // Prepare a manager with two enabled inspectors and one disabled inspector
    let mut manager = ToolInspectionManager::new();

    let inspector1 = MockInspector {
        name: "mock1",
        enabled: true,
        results: vec![InspectionResult {
            tool_request_id: "req_1".to_string(),
            action: InspectionAction::Allow,
            reason: "ok".to_string(),
            confidence: 0.9,
            inspector_name: "mock1".to_string(),
            finding_id: None,
        }],
    };

    let inspector2 = MockInspector {
        name: "mock2",
        enabled: true,
        results: vec![
            InspectionResult {
                tool_request_id: "req_2".to_string(),
                action: InspectionAction::RequireApproval(Some("needs review".to_string())),
                reason: "review".to_string(),
                confidence: 0.8,
                inspector_name: "mock2".to_string(),
                finding_id: Some("F-001".to_string()),
            },
            InspectionResult {
                tool_request_id: "req_3".to_string(),
                action: InspectionAction::Deny,
                reason: "deny".to_string(),
                confidence: 1.0,
                inspector_name: "mock2".to_string(),
                finding_id: None,
            },
        ],
    };

    let disabled = MockInspector {
        name: "disabled",
        enabled: false,
        results: vec![InspectionResult {
            tool_request_id: "req_disabled".to_string(),
            action: InspectionAction::Allow,
            reason: "should not appear".to_string(),
            confidence: 0.5,
            inspector_name: "disabled".to_string(),
            finding_id: None,
        }],
    };

    manager.add_inspector(Box::new(inspector1));
    manager.add_inspector(Box::new(inspector2));
    manager.add_inspector(Box::new(disabled));

    // Call inspect with empty inputs; our mock ignores them and returns predefined results
    let results = manager.inspect_tools(&[], &[]).await.unwrap();

    // We should have results from the two enabled inspectors only (1 + 2 = 3)
    assert_eq!(results.len(), 3);

    // Ensure all expected request IDs are present and the disabled one is not
    let ids: Vec<String> = results.into_iter().map(|r| r.tool_request_id).collect();
    assert!(ids.contains(&"req_1".to_string()));
    assert!(ids.contains(&"req_2".to_string()));
    assert!(ids.contains(&"req_3".to_string()));
    assert!(!ids.contains(&"req_disabled".to_string()));
}
