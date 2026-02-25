use sacp::schema::{
    PermissionOption, PermissionOptionKind, RequestPermissionOutcome, RequestPermissionRequest,
    RequestPermissionResponse, SelectedPermissionOutcome, ToolCallStatus,
};

#[derive(Clone, Debug)]
pub struct PermissionMapping {
    pub allow_option_id: Option<String>,
    pub reject_option_id: Option<String>,
    pub rejected_tool_status: ToolCallStatus,
}

impl Default for PermissionMapping {
    fn default() -> Self {
        Self {
            allow_option_id: None,
            reject_option_id: None,
            rejected_tool_status: ToolCallStatus::Failed,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PermissionDecision {
    AllowAlways,
    AllowOnce,
    RejectAlways,
    RejectOnce,
    Cancel,
}

impl PermissionDecision {
    pub(crate) fn should_record_rejection(self) -> bool {
        matches!(
            self,
            PermissionDecision::RejectAlways
                | PermissionDecision::RejectOnce
                | PermissionDecision::Cancel
        )
    }
}

pub fn map_permission_response(
    mapping: &PermissionMapping,
    request: &RequestPermissionRequest,
    decision: PermissionDecision,
) -> RequestPermissionResponse {
    let selected_id = match decision {
        PermissionDecision::AllowAlways => select_option_id(
            &request.options,
            &mapping.allow_option_id,
            PermissionOptionKind::AllowAlways,
        )
        .or_else(|| {
            select_option_id(
                &request.options,
                &mapping.allow_option_id,
                PermissionOptionKind::AllowOnce,
            )
        }),
        PermissionDecision::AllowOnce => select_option_id(
            &request.options,
            &mapping.allow_option_id,
            PermissionOptionKind::AllowOnce,
        )
        .or_else(|| {
            select_option_id(
                &request.options,
                &mapping.allow_option_id,
                PermissionOptionKind::AllowAlways,
            )
        }),
        PermissionDecision::RejectAlways => select_option_id(
            &request.options,
            &mapping.reject_option_id,
            PermissionOptionKind::RejectAlways,
        )
        .or_else(|| {
            select_option_id(
                &request.options,
                &mapping.reject_option_id,
                PermissionOptionKind::RejectOnce,
            )
        }),
        PermissionDecision::RejectOnce => select_option_id(
            &request.options,
            &mapping.reject_option_id,
            PermissionOptionKind::RejectOnce,
        )
        .or_else(|| {
            select_option_id(
                &request.options,
                &mapping.reject_option_id,
                PermissionOptionKind::RejectAlways,
            )
        }),
        PermissionDecision::Cancel => None,
    };

    if let Some(option_id) = selected_id {
        RequestPermissionResponse::new(RequestPermissionOutcome::Selected(
            SelectedPermissionOutcome::new(option_id),
        ))
    } else {
        RequestPermissionResponse::new(RequestPermissionOutcome::Cancelled)
    }
}

fn select_option_id(
    options: &[PermissionOption],
    preferred_id: &Option<String>,
    kind: PermissionOptionKind,
) -> Option<String> {
    if let Some(preferred_id) = preferred_id {
        let preferred = sacp::schema::PermissionOptionId::new(preferred_id.clone());
        if options.iter().any(|opt| opt.option_id == preferred) {
            return Some(preferred_id.clone());
        }
    }

    options
        .iter()
        .find(|opt| opt.kind == kind)
        .map(|opt| opt.option_id.0.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use sacp::schema::{PermissionOptionId, ToolCallId, ToolCallUpdate, ToolCallUpdateFields};
    use test_case::test_case;

    fn make_request(options: Vec<PermissionOption>) -> RequestPermissionRequest {
        let tool_call =
            ToolCallUpdate::new(ToolCallId::new("tool-1"), ToolCallUpdateFields::default());
        RequestPermissionRequest::new("session-1", tool_call, options)
    }

    fn option(id: &str, kind: PermissionOptionKind) -> PermissionOption {
        PermissionOption::new(
            PermissionOptionId::new(id.to_string()),
            id.to_string(),
            kind,
        )
    }

    #[test_case(
        Some("allow"),
        None,
        PermissionDecision::AllowOnce,
        "allow",
        true;
        "allow_uses_preferred_id"
    )]
    #[test_case(
        None,
        None,
        PermissionDecision::AllowAlways,
        "allow_always",
        false;
        "allow_always_prefers_kind"
    )]
    #[test_case(
        Some("missing"),
        None,
        PermissionDecision::AllowOnce,
        "allow_once",
        false;
        "allow_falls_back_to_kind"
    )]
    #[test_case(
        None,
        Some("reject"),
        PermissionDecision::RejectOnce,
        "reject",
        true;
        "reject_uses_preferred_id"
    )]
    #[test_case(
        None,
        Some("missing"),
        PermissionDecision::RejectOnce,
        "reject_once",
        false;
        "reject_falls_back_to_kind"
    )]
    fn test_permission_mapping(
        allow_option_id: Option<&str>,
        reject_option_id: Option<&str>,
        decision: PermissionDecision,
        expected_id: &str,
        include_preferred: bool,
    ) {
        let mut options = vec![
            option("allow_once", PermissionOptionKind::AllowOnce),
            option("allow_always", PermissionOptionKind::AllowAlways),
            option("reject_once", PermissionOptionKind::RejectOnce),
            option("reject", PermissionOptionKind::RejectAlways),
        ];

        if include_preferred {
            if let Some(preferred_allow) = allow_option_id {
                if !options
                    .iter()
                    .any(|opt| opt.option_id.0.as_ref() == preferred_allow)
                {
                    options.push(option(preferred_allow, PermissionOptionKind::AllowOnce));
                }
            }

            if let Some(preferred_reject) = reject_option_id {
                if !options
                    .iter()
                    .any(|opt| opt.option_id.0.as_ref() == preferred_reject)
                {
                    options.push(option(preferred_reject, PermissionOptionKind::RejectOnce));
                }
            }
        }

        let request = make_request(options);

        let mapping = PermissionMapping {
            allow_option_id: allow_option_id.map(|s| s.to_string()),
            reject_option_id: reject_option_id.map(|s| s.to_string()),
            rejected_tool_status: ToolCallStatus::Failed,
        };

        let response = map_permission_response(&mapping, &request, decision);
        match response.outcome {
            RequestPermissionOutcome::Selected(selected) => {
                assert_eq!(selected.option_id.0.as_ref(), expected_id);
            }
            _ => panic!("expected selected outcome"),
        }
    }

    #[test_case(PermissionDecision::Cancel; "cancelled")]
    fn test_permission_cancelled(decision: PermissionDecision) {
        let request = make_request(vec![option("allow_once", PermissionOptionKind::AllowOnce)]);
        let response = map_permission_response(&PermissionMapping::default(), &request, decision);
        assert!(matches!(
            response.outcome,
            RequestPermissionOutcome::Cancelled
        ));
    }
}
