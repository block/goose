use crate::agents::platform_tools::PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME;
use crate::config::permission::PermissionLevel;
use crate::config::PermissionManager;
use crate::conversation::message::{Message, MessageContent, ToolRequest};
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use chrono::Utc;
use indoc::indoc;
use rmcp::model::{Tool, ToolAnnotations};
use rmcp::object;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashSet;
use std::sync::Arc;

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
                Some(tool_call.name.clone())
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
                    if let Value::Object(arguments) = &tool_call.arguments {
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

    let res = provider
        .complete(
            "You are a good analyst and can detect operations whether they have read-only operations.",
            check_messages.messages(),
            &[tool.clone()],
        )
        .await;

    // Process the response and return an empty vector if the response is invalid
    if let Ok((message, _usage)) = res {
        extract_read_only_tools(&message).unwrap_or_default()
    } else {
        vec![]
    }
}

// Define return structure
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
                if tool_call.name == PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME {
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

                        if tools_with_readonly_annotation.contains(&tool_call.name) {
                            approved.push(request.clone());
                        } else if tools_without_annotation.contains(&tool_call.name) {
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
                if detected_readonly_tools.contains(&tool_call.name) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{Message, MessageContent, ToolRequest};
    use crate::model::ModelConfig;
    use crate::providers::base::{Provider, ProviderMetadata, ProviderUsage, Usage};
    use crate::providers::errors::ProviderError;
    use chrono::Utc;
    use mcp_core::{ToolCall, ToolResult};
    use rmcp::model::{Role, Tool};
    use serde_json::json;
    use tempfile::NamedTempFile;

    #[derive(Clone)]
    struct MockProvider {
        model_config: ModelConfig,
    }

    #[async_trait::async_trait]
    impl Provider for MockProvider {
        fn metadata() -> ProviderMetadata {
            ProviderMetadata::empty()
        }

        fn get_model_config(&self) -> ModelConfig {
            self.model_config.clone()
        }

        async fn complete(
            &self,
            _system: &str,
            _messages: &[Message],
            _tools: &[Tool],
        ) -> anyhow::Result<(Message, ProviderUsage), ProviderError> {
            Ok((
                Message::new(
                    Role::Assistant,
                    Utc::now().timestamp(),
                    vec![MessageContent::ToolRequest(ToolRequest {
                        id: "mock_tool_request".to_string(),
                        tool_call: ToolResult::Ok(ToolCall {
                            name: "platform__tool_by_tool_permission".to_string(),
                            arguments: json!({
                                "read_only_tools": ["file_reader", "data_fetcher"]
                            }),
                        }),
                    })],
                ),
                ProviderUsage::new("mock".to_string(), Usage::default()),
            ))
        }
    }

    fn create_mock_provider() -> Arc<dyn Provider> {
        let config = ModelConfig::new_or_fail("test-model");
        let mock_model_config = config.with_context_limit(200_000.into());
        Arc::new(MockProvider {
            model_config: mock_model_config,
        })
    }

    #[tokio::test]
    async fn test_create_read_only_tool() {
        let tool = create_read_only_tool();
        assert_eq!(tool.name, "platform__tool_by_tool_permission");
        assert!(tool
            .description
            .as_ref()
            .map_or(false, |desc| desc.contains("read-only operation")));
    }

    #[test]
    fn test_create_check_messages() {
        let tool_request = ToolRequest {
            id: "tool_1".to_string(),
            tool_call: ToolResult::Ok(ToolCall {
                name: "file_reader".to_string(),
                arguments: json!({"path": "/path/to/file"}),
            }),
        };

        let messages = create_check_messages(vec![&tool_request]);
        assert_eq!(messages.len(), 1);
        let content = &messages.first().unwrap().content[0];
        if let MessageContent::Text(text_content) = content {
            assert!(text_content
                .text
                .contains("Analyze the tool requests and list the tools"));
            assert!(text_content.text.contains("file_reader"));
        } else {
            panic!("Expected text content");
        }
    }

    #[test]
    fn test_extract_read_only_tools() {
        let message = Message::new(
            Role::Assistant,
            Utc::now().timestamp(),
            vec![MessageContent::ToolRequest(ToolRequest {
                id: "tool_2".to_string(),
                tool_call: ToolResult::Ok(ToolCall {
                    name: "platform__tool_by_tool_permission".to_string(),
                    arguments: json!({
                        "read_only_tools": ["file_reader", "data_fetcher"]
                    }),
                }),
            })],
        );

        let result = extract_read_only_tools(&message);
        assert!(result.is_some());
        let tools = result.unwrap();
        assert_eq!(tools, vec!["file_reader", "data_fetcher"]);
    }

    #[tokio::test]
    async fn test_detect_read_only_tools() {
        let provider = create_mock_provider();
        let tool_request = ToolRequest {
            id: "tool_1".to_string(),
            tool_call: ToolResult::Ok(ToolCall {
                name: "file_reader".to_string(),
                arguments: json!({"path": "/path/to/file"}),
            }),
        };

        let result = detect_read_only_tools(provider, vec![&tool_request]).await;
        assert_eq!(result, vec!["file_reader", "data_fetcher"]);
    }

    #[tokio::test]
    async fn test_detect_read_only_tools_empty_requests() {
        let provider = create_mock_provider();
        let result = detect_read_only_tools(provider, vec![]).await;
        assert!(result.is_empty());
    }

    #[tokio::test]
    async fn test_check_tool_permissions_smart_approve() {
        // Setup mocks
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();
        let mut permission_manager = PermissionManager::new(temp_path);
        let provider = create_mock_provider();

        let tools_with_readonly_annotation: HashSet<String> =
            vec!["file_reader".to_string()].into_iter().collect();
        let tools_without_annotation: HashSet<String> =
            vec!["data_fetcher".to_string()].into_iter().collect();

        permission_manager.update_user_permission("file_reader", PermissionLevel::AlwaysAllow);
        permission_manager
            .update_smart_approve_permission("data_fetcher", PermissionLevel::AskBefore);

        let tool_request_1 = ToolRequest {
            id: "tool_1".to_string(),
            tool_call: ToolResult::Ok(ToolCall {
                name: "file_reader".to_string(),
                arguments: serde_json::json!({"path": "/path/to/file"}),
            }),
        };

        let tool_request_2 = ToolRequest {
            id: "tool_2".to_string(),
            tool_call: ToolResult::Ok(ToolCall {
                name: "data_fetcher".to_string(),
                arguments: serde_json::json!({"url": "http://example.com"}),
            }),
        };

        let enable_extension = ToolRequest {
            id: "tool_3".to_string(),
            tool_call: ToolResult::Ok(ToolCall {
                name: PLATFORM_MANAGE_EXTENSIONS_TOOL_NAME.to_string(),
                arguments: serde_json::json!({"action": "enable", "extension_name": "data_fetcher"}),
            }),
        };

        let candidate_requests: Vec<ToolRequest> =
            vec![tool_request_1, tool_request_2, enable_extension];

        // Call the function under test
        let (result, enable_extension_request_ids) = check_tool_permissions(
            &candidate_requests,
            "smart_approve",
            tools_with_readonly_annotation,
            tools_without_annotation,
            &mut permission_manager,
            provider,
        )
        .await;

        // Validate the result
        assert_eq!(result.approved.len(), 1); // file_reader should be approved
        assert_eq!(result.needs_approval.len(), 2); // data_fetcher should need approval
        assert_eq!(result.denied.len(), 0); // No tool should be denied in this test
        assert_eq!(enable_extension_request_ids.len(), 1);

        // Ensure the right tools are in the approved and needs_approval lists
        assert!(result.approved.iter().any(|req| req.id == "tool_1"));
        assert!(result.needs_approval.iter().any(|req| req.id == "tool_2"));
        assert!(result.needs_approval.iter().any(|req| req.id == "tool_3"));
        assert!(enable_extension_request_ids.iter().any(|id| id == "tool_3"));
    }

    #[tokio::test]
    async fn test_check_tool_permissions_auto() {
        // Setup mocks
        let temp_file = NamedTempFile::new().unwrap();
        let temp_path = temp_file.path();
        let mut permission_manager = PermissionManager::new(temp_path);
        let provider = create_mock_provider();

        let tools_with_readonly_annotation: HashSet<String> =
            vec!["file_reader".to_string()].into_iter().collect();
        let tools_without_annotation: HashSet<String> =
            vec!["data_fetcher".to_string()].into_iter().collect();

        permission_manager.update_user_permission("file_reader", PermissionLevel::AlwaysAllow);
        permission_manager
            .update_smart_approve_permission("data_fetcher", PermissionLevel::AskBefore);

        let tool_request_1 = ToolRequest {
            id: "tool_1".to_string(),
            tool_call: ToolResult::Ok(ToolCall {
                name: "file_reader".to_string(),
                arguments: serde_json::json!({"path": "/path/to/file"}),
            }),
        };

        let tool_request_2 = ToolRequest {
            id: "tool_2".to_string(),
            tool_call: ToolResult::Ok(ToolCall {
                name: "data_fetcher".to_string(),
                arguments: serde_json::json!({"url": "http://example.com"}),
            }),
        };

        let candidate_requests: Vec<ToolRequest> = vec![tool_request_1, tool_request_2];

        // Call the function under test
        let (result, _) = check_tool_permissions(
            &candidate_requests,
            "auto",
            tools_with_readonly_annotation,
            tools_without_annotation,
            &mut permission_manager,
            provider,
        )
        .await;

        // Validate the result
        assert_eq!(result.approved.len(), 2); // file_reader should be approved
        assert_eq!(result.needs_approval.len(), 0); // data_fetcher should need approval
        assert_eq!(result.denied.len(), 0); // No tool should be denied in this test
    }
}
