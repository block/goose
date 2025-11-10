use crate::agents::extension_manager_extension::MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE;
use crate::config::permission::PermissionLevel;
use crate::config::PermissionManager;
use crate::conversation::message::{Message, MessageContent, ToolRequest};
use crate::conversation::Conversation;
use crate::prompt_template::render_global_file;
use crate::providers::base::Provider;
use chrono::Utc;
use indoc::indoc;
use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

#[derive(Serialize)]
struct PermissionJudgeContext {
    // Empty struct for now since the current template doesn't need variables
}

/// Creates the tool definition for checking read-only permissions.
fn create_read_only_tool() -> Tool {
    Tool::new(
        "platform__tool_by_tool_permission".to_string(),
        indoc! {r#"
            Analyze the tool requests and determine which ones perform read-only operations.

            What constitutes a read-only operation:
            - A read-only operation retrieves information without modifying any data or state.
            - Examples include:
                - Reading a file without writing to it.
                - Querying a database without making updates.
                - Retrieving information from APIs without performing POST, PUT, or DELETE operations.

            Examples of read vs. write operations:
            - Read Operations:
                - `SELECT` query in SQL.
                - Reading file metadata or content.
                - Listing directory contents.
            - Write Operations:
                - `INSERT`, `UPDATE`, or `DELETE` in SQL.
                - Writing or appending to a file.
                - Modifying system configurations.
                - Sending messages to Slack channel.

            How to analyze tool requests:
            - Inspect each tool request to identify its purpose based on its name and arguments.
            - Categorize the operation as read-only if it does not involve any state or data modification.
            - Return a list of tool names that are strictly read-only. If you cannot make the decision, then it is not read-only.

            Use this analysis to generate the list of tools performing read-only operations from the provided tool requests.
        "#}
        .to_string(),
        object!({
            "type": "object",
            "properties": {
                "read_only_tools": {
                    "type": "array",
                    "items": {
                        "type": "string"
                    },
                    "description": "Optional list of tool names which has read-only operations."
                }
            },
            "required": []
        })
    ).annotate(ToolAnnotations {
        title: Some("Check tool operation".to_string()),
        read_only_hint: Some(true),
        destructive_hint: Some(false),
        idempotent_hint: Some(false),
        open_world_hint: Some(false),
    })
}

/// Builds the message to be sent to the LLM for detecting read-only operations.
fn create_check_messages(tool_requests: Vec<&ToolRequest>) -> Conversation {
    let tool_names: Vec<String> = tool_requests
        .iter()
        .filter_map(|req| {
            if let Ok(tool_call) = &req.tool_call {
                Some(tool_call.name.to_string().clone())
            } else {
                None // Skip requests with errors in tool_call
            }
        })
        .collect();
    let mut check_messages = vec![];
    check_messages.push(Message::new(
        rmcp::model::Role::User,
        Utc::now().timestamp(),
        vec![MessageContent::text(format!(
                "Here are the tool requests: {:?}\n\nAnalyze the tool requests and list the tools that perform read-only operations. \
                \n\nGuidelines for Read-Only Operations: \
                \n- Read-only operations do not modify any data or state. \
                \n- Examples include file reading, SELECT queries in SQL, and directory listing. \
                \n- Write operations include INSERT, UPDATE, DELETE, and file writing. \
                \n\nPlease provide a list of tool names that qualify as read-only:",
                tool_names.join(", "),
            ))],
    ));
    Conversation::new_unvalidated(check_messages)
}

/// Processes the response to extract the list of tools with read-only operations.
fn extract_read_only_tools(response: &Message) -> Option<Vec<String>> {
    for content in &response.content {
        if let MessageContent::ToolRequest(tool_request) = content {
            if let Ok(tool_call) = &tool_request.tool_call {
                if tool_call.name == "platform__tool_by_tool_permission" {
                    if let Some(arguments) = &tool_call.arguments {
                        if let Some(Value::Array(read_only_tools)) =
                            arguments.get("read_only_tools")
                        {
                            return Some(
                                read_only_tools
                                    .iter()
                                    .filter_map(|tool| tool.as_str().map(String::from))
                                    .collect(),
                            );
                        }
                    }
                }
            }
        }
    }
    None
}

/// Executes the read-only tools detection and returns the list of tools with read-only operations.
pub async fn detect_read_only_tools(
    provider: Arc<dyn Provider>,
    tool_requests: Vec<&ToolRequest>,
) -> Vec<String> {
    if tool_requests.is_empty() {
        return vec![];
    }
    let tool = create_read_only_tool();
    let check_messages = create_check_messages(tool_requests);

    let context = PermissionJudgeContext {};
    let system_prompt = render_global_file("permission_judge.md", &context)
        .unwrap_or_else(|_| "You are a good analyst and can detect operations whether they have read-only operations.".to_string());

    let res = provider
        .complete(
            &system_prompt,
            check_messages.messages(),
            std::slice::from_ref(&tool),
        )
        .await;

    // Process the response and return an empty vector if the response is invalid
    if let Ok((message, _usage)) = res {
        extract_read_only_tools(&message).unwrap_or_default()
    } else {
        vec![]
    }
}

/// Result of permission checking for tool requests
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PermissionCheckResult {
    pub approved: Vec<ToolRequest>,
    pub needs_approval: Vec<ToolRequest>,
    pub denied: Vec<ToolRequest>,
}

pub async fn check_tool_permissions(
    candidate_requests: &[ToolRequest],
    mode: &str,
    tools_with_readonly_annotation: HashSet<String>,
    tools_without_annotation: HashSet<String>,
    permission_manager: &mut PermissionManager,
    provider: Arc<dyn Provider>,
) -> (PermissionCheckResult, Vec<String>) {
    let mut approved = vec![];
    let mut needs_approval = vec![];
    let mut denied = vec![];
    let mut llm_detect_candidates = vec![];
    let mut extension_request_ids = vec![];

    for request in candidate_requests {
        if let Ok(tool_call) = request.tool_call.clone() {
            if mode == "chat" {
                continue;
            } else if mode == "auto" {
                approved.push(request.clone());
            } else {
                if tool_call.name == MANAGE_EXTENSIONS_TOOL_NAME_COMPLETE {
                    extension_request_ids.push(request.id.clone());
                }

                // 1. Check user-defined permission
                if let Some(level) = permission_manager.get_user_permission(&tool_call.name) {
                    match level {
                        PermissionLevel::AlwaysAllow => approved.push(request.clone()),
                        PermissionLevel::AskBefore => needs_approval.push(request.clone()),
                        PermissionLevel::NeverAllow => denied.push(request.clone()),
                    }
                    continue;
                }

                // 2. Fallback based on mode
                match mode {
                    "approve" => {
                        needs_approval.push(request.clone());
                    }
                    "smart_approve" => {
                        if let Some(level) =
                            permission_manager.get_smart_approve_permission(&tool_call.name)
                        {
                            match level {
                                PermissionLevel::AlwaysAllow => approved.push(request.clone()),
                                PermissionLevel::AskBefore => needs_approval.push(request.clone()),
                                PermissionLevel::NeverAllow => denied.push(request.clone()),
                            }
                            continue;
                        }

                        if tools_with_readonly_annotation.contains(&tool_call.name.to_string()) {
                            approved.push(request.clone());
                        } else if tools_without_annotation.contains(&tool_call.name.to_string()) {
                            llm_detect_candidates.push(request.clone());
                        } else {
                            needs_approval.push(request.clone());
                        }
                    }
                    _ => {
                        needs_approval.push(request.clone());
                    }
                }
            }
        }
    }

    // 3. LLM detect
    if !llm_detect_candidates.is_empty() && mode == "smart_approve" {
        let detected_readonly_tools =
            detect_read_only_tools(provider, llm_detect_candidates.iter().collect()).await;
        for request in llm_detect_candidates {
            if let Ok(tool_call) = request.tool_call.clone() {
                if detected_readonly_tools.contains(&tool_call.name.to_string()) {
                    approved.push(request.clone());
                    permission_manager.update_smart_approve_permission(
                        &tool_call.name,
                        PermissionLevel::AlwaysAllow,
                    );
                } else {
                    needs_approval.push(request.clone());
                    permission_manager.update_smart_approve_permission(
                        &tool_call.name,
                        PermissionLevel::AskBefore,
                    );
                }
            }
        }
    }

    (
        PermissionCheckResult {
            approved,
            needs_approval,
            denied,
        },
        extension_request_ids,
    )
}
