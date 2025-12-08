use crate::agents::extension::PlatformExtensionContext;
use crate::agents::mcp_client::{Error, McpClientTrait};
use crate::conversation::Conversation;
use crate::session::extension_data::ExtensionState;
use crate::session::{extension_data, SessionManager};
use anyhow::Result;
use async_trait::async_trait;
use indoc::indoc;
use rmcp::model::{
    CallToolResult, Content, GetPromptResult, Implementation, InitializeResult, JsonObject,
    ListPromptsResult, ListResourcesResult, ListToolsResult, ProtocolVersion, ReadResourceResult,
    Role, ServerCapabilities, ServerNotification, Tool, ToolAnnotations, ToolsCapability,
};
use schemars::{schema_for, JsonSchema};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

pub static EXTENSION_NAME: &str = "todo";

const AUTONOMOUS_MODE_TRIGGERS: &[&str] = &["Act autonomously.", "Work independently."];

const AUTONOMOUS_MODE_TODO_TEMPLATE: &str = indoc! {r#"
    Operating principles (hard-won lessons that you need to consider when operating as an llm agent):
        Extract all requirements as checklist before coding. Missing one means failure.
        Read test files first - they define the spec. Filenames, paths, JSON keys, casing are immutable contracts.
        Follow existing codebase patterns. Examine similar code first. Consistency over innovation.
        Test with realistic inputs. Import success doesn't mean it works.
        Never run foreground services or use & operator - blocks execution. Use proper daemonization.
        Modify source config files directly, not runtime workarounds. Validate syntax before applying.
        Use pre-configured tools before manual setup. Existing over new.
        Preserve exact format, case, structure. Downstream tools need exact matches.
        Redirect large outputs to files. Summarize for context.
        Test simple commands manually before full automation.
        Repeated failures need root cause investigation - processes, scheduled tasks, interference.
        Multi-component systems need ALL pieces started explicitly. Any missing piece fails everything.
        Set explicit timeouts (30-300s) for all network operations. No timeout means indefinite hang.
        No interactive prompts anywhere in tool chain. Use non-interactive flags: git commit -m, docker run -d, etc.
        Implement completely after diagnosis. Identifying problems without fixing fails the task.
        Stop when tests pass. Don't optimize beyond requirements.
        chmod +x scripts with shebangs. Test write permissions with touch before actual writes.
        Match exact output format expectations - JSON/CSV/plain. Human-readable isn't machine-readable.
        Retry network operations 3x with 2-5s waits. Distinguish transient from permanent failures.
        Verify immediately after creating - files exist with correct content, APIs respond, services healthy.
        Sanitize inputs, check bounds, handle nulls, cover edge cases. Think like malicious QA.
        Python first. Bash only if system-specific. Other languages only when required.

    Start on this list right away before proceeding with the user instructions.
    Skipping any of these steps is an error and will result in failure:

    ## Phase 1: Capture & Scope
    - [ ] IMMEDIATELY write the task verbatim to .goose_task.md
    - [ ] Write all requirements (explicit, implicit, edge cases) to .goose_task_requirements.md
    - [ ] Write what is OUT OF SCOPE to .goose_task_requirements.md - resist scope creep
    - [ ] Write any relevant operating principles to the end of .goose_task_requirements.md

    ## Phase 2: Understand & Plan
    - [ ] If unfamiliar with the codebase/system, spawn Investigator to understand relevant areas
    - [ ] For complex tasks (3+ steps), spawn Planner to create an action plan
    - [ ] Update this TODO with the concrete steps from your plan

    ## Phase 3: Implement
    - [ ] Implement following the plan, verifying each step before proceeding
    - [ ] If tempted to add something, check: is it in scope? If not, don't do it.

    ## Phase 4: Verify
    - [ ] Test: run tests, lints, type checks - deterministic verification required
    - [ ] Anti-pattern checklist:
      - [ ] Did I handle errors appropriately?
      - [ ] Did I test both happy path AND failure paths?
      - [ ] Did I leave any debug code or temporary hacks?
      - [ ] Did I break any existing functionality?
    - [ ] Reread .goose_task.md and .goose_task_requirements.md
    - [ ] Confirm EVERY requirement is satisfied and nothing out-of-scope was added

    ## Subagent Guidance
    • Investigator: Use when you need to understand how something works before changing it
    • Planner: Use when the path forward is unclear or involves multiple steps

    IMPORTANT: Subagents have NO context from your conversation. When spawning them,
    instruct them to first read .goose_task.md and .goose_task_requirements.md so
    they understand the full task and constraints.

    You must include the "Reread .goose_task.md and .goose_task_requirements.md" step in your updated
    todo list.

    Your context degrades rapidly. Writing to disk is essential. DO IT NOW.
"#};

#[derive(Debug, Serialize, Deserialize, JsonSchema)]
struct TodoWriteParams {
    content: String,
}

pub struct TodoClient {
    info: InitializeResult,
    context: PlatformExtensionContext,
    fallback_content: tokio::sync::RwLock<String>,
}

impl TodoClient {
    pub fn new(context: PlatformExtensionContext) -> Result<Self> {
        let info = InitializeResult {
            protocol_version: ProtocolVersion::V_2025_03_26,
            capabilities: ServerCapabilities {
                tools: Some(ToolsCapability {
                    list_changed: Some(false),
                }),
                resources: None,
                prompts: None,
                completions: None,
                experimental: None,
                logging: None,
            },
            server_info: Implementation {
                name: EXTENSION_NAME.to_string(),
                title: Some("Todo".to_string()),
                version: "1.0.0".to_string(),
                icons: None,
                website_url: None,
            },
            instructions: Some(indoc! {r#"
                Task Management

                Use todo_write for tasks with 2+ steps, multiple files/components, or uncertain scope.
                Your TODO content is automatically available in your context.

                Workflow:
                - Start: write initial checklist
                - During: update progress
                - End: verify all complete

                Warning: todo_write overwrites entirely; always include ALL content you want to keep

                Keep items short, specific, action-oriented. Not using the todo tool for complex tasks is an error.

                For autonomous work, missing requirements means failure - document all requirements in TODO immediately.

                Template:
                - [ ] Implement feature X
                  - [ ] Update API
                  - [ ] Write tests
                  - [ ] Run tests
                  - [ ] Run lint
                - [ ] Blocked: waiting on credentials
            "#}.to_string()),
        };

        Ok(Self {
            info,
            context,
            fallback_content: tokio::sync::RwLock::new(String::new()),
        })
    }

    async fn handle_write_todo(
        &self,
        arguments: Option<JsonObject>,
    ) -> Result<Vec<Content>, String> {
        let content = arguments
            .as_ref()
            .ok_or("Missing arguments")?
            .get("content")
            .and_then(|v| v.as_str())
            .ok_or("Missing required parameter: content")?
            .to_string();

        let char_count = content.chars().count();
        let max_chars = std::env::var("GOOSE_TODO_MAX_CHARS")
            .ok()
            .and_then(|s| s.parse().ok())
            .unwrap_or(50_000);

        if max_chars > 0 && char_count > max_chars {
            return Err(format!(
                "Todo list too large: {} chars (max: {})",
                char_count, max_chars
            ));
        }

        if let Some(session_id) = &self.context.session_id {
            match SessionManager::get_session(session_id, false).await {
                Ok(mut session) => {
                    let todo_state = extension_data::TodoState::new(content);
                    if todo_state
                        .to_extension_data(&mut session.extension_data)
                        .is_ok()
                    {
                        match SessionManager::update_session(session_id)
                            .extension_data(session.extension_data)
                            .apply()
                            .await
                        {
                            Ok(_) => Ok(vec![Content::text(format!(
                                "Updated ({} chars)",
                                char_count
                            ))]),
                            Err(_) => Err("Failed to update session metadata".to_string()),
                        }
                    } else {
                        Err("Failed to serialize TODO state".to_string())
                    }
                }
                Err(_) => Err("Failed to read session metadata".to_string()),
            }
        } else {
            let mut fallback = self.fallback_content.write().await;
            *fallback = content;
            Ok(vec![Content::text(format!(
                "Updated ({} chars)",
                char_count
            ))])
        }
    }

    fn get_tools() -> Vec<Tool> {
        let schema = schema_for!(TodoWriteParams);
        let schema_value =
            serde_json::to_value(schema).expect("Failed to serialize TodoWriteParams schema");

        vec![Tool::new(
            "todo_write".to_string(),
            indoc! {r#"
                    Overwrite the entire TODO content.

                    The content persists across conversation turns and compaction. Use this for:
                    - Task tracking and progress updates
                    - Important notes and reminders

                    Update TODO after each step - it's the user's window into your progress.

                    WARNING: This operation completely replaces the existing content. Always include
                    all content you want to keep, not just the changes.
                "#}
            .to_string(),
            schema_value.as_object().unwrap().clone(),
        )
        .annotate(ToolAnnotations {
            title: Some("Write TODO".to_string()),
            read_only_hint: Some(false),
            destructive_hint: Some(true),
            idempotent_hint: Some(false),
            open_world_hint: Some(false),
        })]
    }

    fn should_initialize_autonomous_mode(
        conversation: &Conversation,
        system_prompt: Option<&str>,
    ) -> bool {
        // Check system prompt first (recipe instructions go here)
        if let Some(prompt) = system_prompt {
            if AUTONOMOUS_MODE_TRIGGERS
                .iter()
                .any(|trigger| prompt.contains(trigger))
            {
                return conversation.messages().len() == 1;
            }
        }

        // Fall back to checking the first user message (recipe prompt goes here)
        if conversation.messages().len() != 1 {
            return false;
        }

        let first_message = match conversation.messages().first() {
            Some(msg) if msg.role == Role::User => msg,
            _ => return false,
        };

        let text = first_message.as_concat_text();
        AUTONOMOUS_MODE_TRIGGERS
            .iter()
            .any(|trigger| text.contains(trigger))
    }

    async fn initialize_autonomous_todo(&self) {
        let session_id = match &self.context.session_id {
            Some(id) => id,
            None => return,
        };

        let session = match SessionManager::get_session(session_id, false).await {
            Ok(s) => s,
            Err(_) => return,
        };

        let existing = extension_data::TodoState::from_extension_data(&session.extension_data);
        if existing.is_some_and(|t| !t.content.trim().is_empty()) {
            return;
        }

        let todo_state = extension_data::TodoState::new(AUTONOMOUS_MODE_TODO_TEMPLATE.to_string());
        let mut extension_data = session.extension_data;

        if todo_state.to_extension_data(&mut extension_data).is_err() {
            return;
        }

        let _ = SessionManager::update_session(session_id)
            .extension_data(extension_data)
            .apply()
            .await;
    }
}

#[async_trait]
impl McpClientTrait for TodoClient {
    async fn list_resources(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListResourcesResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn read_resource(
        &self,
        _uri: &str,
        _cancellation_token: CancellationToken,
    ) -> Result<ReadResourceResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn list_tools(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListToolsResult, Error> {
        Ok(ListToolsResult {
            tools: Self::get_tools(),
            next_cursor: None,
        })
    }

    async fn call_tool(
        &self,
        name: &str,
        arguments: Option<JsonObject>,
        _cancellation_token: CancellationToken,
    ) -> Result<CallToolResult, Error> {
        let content = match name {
            "todo_write" => self.handle_write_todo(arguments).await,
            _ => Err(format!("Unknown tool: {}", name)),
        };

        match content {
            Ok(content) => Ok(CallToolResult::success(content)),
            Err(error) => Ok(CallToolResult::error(vec![Content::text(format!(
                "Error: {}",
                error
            ))])),
        }
    }

    async fn list_prompts(
        &self,
        _next_cursor: Option<String>,
        _cancellation_token: CancellationToken,
    ) -> Result<ListPromptsResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn get_prompt(
        &self,
        _name: &str,
        _arguments: Value,
        _cancellation_token: CancellationToken,
    ) -> Result<GetPromptResult, Error> {
        Err(Error::TransportClosed)
    }

    async fn subscribe(&self) -> mpsc::Receiver<ServerNotification> {
        mpsc::channel(1).1
    }

    fn get_info(&self) -> Option<&InitializeResult> {
        Some(&self.info)
    }

    async fn get_moim(
        &self,
        conversation: &Conversation,
        system_prompt: Option<&str>,
    ) -> Option<String> {
        if Self::should_initialize_autonomous_mode(conversation, system_prompt) {
            self.initialize_autonomous_todo().await;
        }

        let session_id = self.context.session_id.as_ref()?;
        let metadata = SessionManager::get_session(session_id, false).await.ok()?;
        let state = extension_data::TodoState::from_extension_data(&metadata.extension_data)?;

        if state.content.trim().is_empty() {
            return None;
        }

        Some(format!("Current tasks and notes:\n{}\n", state.content))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::Message;

    #[test]
    fn test_should_initialize_autonomous_mode_from_user_message() {
        // Single user message with trigger should return true
        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Act autonomously. Build me a web app.")
        ]);
        assert!(TodoClient::should_initialize_autonomous_mode(&conv, None));

        // Single user message with different trigger
        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Work independently. Fix the bug.")
        ]);
        assert!(TodoClient::should_initialize_autonomous_mode(&conv, None));

        // Single user message without trigger should return false
        let conv =
            Conversation::new_unvalidated(vec![Message::user().with_text("Build me a web app.")]);
        assert!(!TodoClient::should_initialize_autonomous_mode(&conv, None));
    }

    #[test]
    fn test_should_initialize_autonomous_mode_from_system_prompt() {
        // System prompt with trigger and single user message should return true
        let conv =
            Conversation::new_unvalidated(vec![Message::user().with_text("Build me a web app.")]);
        let system_prompt = "You are a helpful assistant. Act autonomously. Be thorough.";
        assert!(TodoClient::should_initialize_autonomous_mode(
            &conv,
            Some(system_prompt)
        ));

        // System prompt with different trigger
        let system_prompt = "Work independently. Complete the task without asking questions.";
        assert!(TodoClient::should_initialize_autonomous_mode(
            &conv,
            Some(system_prompt)
        ));

        // System prompt without trigger should return false
        let system_prompt = "You are a helpful assistant.";
        assert!(!TodoClient::should_initialize_autonomous_mode(
            &conv,
            Some(system_prompt)
        ));
    }

    #[test]
    fn test_should_initialize_autonomous_mode_multiple_messages() {
        // Multiple messages should return false even with trigger in system prompt
        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Hello"),
            Message::assistant().with_text("Hi there!"),
            Message::user().with_text("Build me a web app."),
        ]);
        let system_prompt = "Act autonomously.";
        assert!(!TodoClient::should_initialize_autonomous_mode(
            &conv,
            Some(system_prompt)
        ));

        // Multiple messages should return false even with trigger in user message
        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Act autonomously. Build me a web app."),
            Message::assistant().with_text("Sure!"),
        ]);
        assert!(!TodoClient::should_initialize_autonomous_mode(&conv, None));
    }

    #[test]
    fn test_should_initialize_autonomous_mode_empty_conversation() {
        let conv = Conversation::new_unvalidated(vec![]);
        assert!(!TodoClient::should_initialize_autonomous_mode(&conv, None));

        let system_prompt = "Act autonomously.";
        assert!(!TodoClient::should_initialize_autonomous_mode(
            &conv,
            Some(system_prompt)
        ));
    }

    #[test]
    fn test_should_initialize_autonomous_mode_system_prompt_takes_precedence() {
        // If system prompt has trigger, user message doesn't need it
        let conv = Conversation::new_unvalidated(vec![
            Message::user().with_text("Just a regular message without any trigger")
        ]);
        let system_prompt = "Act autonomously. Be helpful.";
        assert!(TodoClient::should_initialize_autonomous_mode(
            &conv,
            Some(system_prompt)
        ));
    }
}
