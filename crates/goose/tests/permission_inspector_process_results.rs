use goose::permission::permission_inspector::PermissionInspector;
use goose::tool_inspection::{InspectionAction, InspectionResult};
use goose::conversation::message::ToolRequest;
use mcp_core::ToolCall;
use serde_json::json;
use std::collections::HashSet;

// This test targets PermissionInspector::process_inspection_results.
// It verifies:
// - Baseline decisions come from "permission" inspector results (Allow / RequireApproval / Deny)
// - Missing permission results default to needs_approval
// - Non-permission inspectors can override with Deny or RequireApproval, but Allow does not override
#[test]
fn test_permission_inspector_process_inspection_results_baseline_and_overrides() {
    // Create a minimal PermissionInspector instance (its internal state isn't used by this method)
    let inspector = PermissionInspector::new(
        "smart".to_string(),
        HashSet::new(),
        HashSet::new(),
    );

    // Prepare three tool requests
    let req1 = ToolRequest {
        id: "req1".to_string(),
        tool_call: Ok(ToolCall::new("tool_a", json!({}))),
    };
    let req2 = ToolRequest {
        id: "req2".to_string(),
        tool_call: Ok(ToolCall::new("tool_b", json!({}))),
    };
    let req3 = ToolRequest {
        id: "req3".to_string(),
        tool_call: Ok(ToolCall::new("tool_c", json!({}))),
    };

    let remaining_requests = vec![req1.clone(), req2.clone(), req3.clone()];

    // Baseline (permission inspector) decisions:
    // - req1: Allow
    // - req2: RequireApproval (no message)
    // - req3: (no explicit permission result) → defaults to needs_approval
    let permission_results = vec![
        InspectionResult {
            tool_request_id: req1.id.clone(),
            action: InspectionAction::Allow,
            reason: "baseline allow".to_string(),
            confidence: 1.0,
            inspector_name: "permission".to_string(),
            finding_id: None,
        },
        InspectionResult {
            tool_request_id: req2.id.clone(),
            action: InspectionAction::RequireApproval(None),
            reason: "baseline ask".to_string(),
            confidence: 1.0,
            inspector_name: "permission".to_string(),
            finding_id: None,
        },
    ];

    // Overrides from other inspectors:
    // - req1: Deny by security → should override Allow and move to denied
    // - req2: Allow by security → should NOT override RequireApproval
    // - req3: Deny by repetition → should move from default needs_approval to denied
    let non_permission_results = vec![
        InspectionResult {
            tool_request_id: req1.id.clone(),
            action: InspectionAction::Deny,
            reason: "security denies".to_string(),
            confidence: 0.9,
            inspector_name: "security".to_string(),
            finding_id: Some("SEC-001".to_string()),
        },
        InspectionResult {
            tool_request_id: req2.id.clone(),
            action: InspectionAction::Allow,
            reason: "security allows".to_string(),
            confidence: 0.8,
            inspector_name: "security".to_string(),
            finding_id: None,
        },
        InspectionResult {
            tool_request_id: req3.id.clone(),
            action: InspectionAction::Deny,
            reason: "repetition denies".to_string(),
            confidence: 1.0,
            inspector_name: "repetition".to_string(),
            finding_id: Some("REP-001".to_string()),
        },
    ];

    let mut all_results = Vec::new();
    all_results.extend(permission_results);
    all_results.extend(non_permission_results);

    // Execute the processing
    let outcome = inspector.process_inspection_results(&remaining_requests, &all_results);

    // Validate outcomes
    // req1 → denied due to security override
    assert!(outcome.denied.iter().any(|r| r.id == req1.id));
    assert!(!outcome.approved.iter().any(|r| r.id == req1.id));
    assert!(!outcome.needs_approval.iter().any(|r| r.id == req1.id));

    // req2 → remains needs_approval (security Allow does not override)
    assert!(outcome.needs_approval.iter().any(|r| r.id == req2.id));
    assert!(!outcome.approved.iter().any(|r| r.id == req2.id));
    assert!(!outcome.denied.iter().any(|r| r.id == req2.id));

    // req3 → denied due to non-permission Deny overriding default needs_approval
    assert!(outcome.denied.iter().any(|r| r.id == req3.id));
    assert!(!outcome.approved.iter().any(|r| r.id == req3.id));
    assert!(!outcome.needs_approval.iter().any(|r| r.id == req3.id));
}
