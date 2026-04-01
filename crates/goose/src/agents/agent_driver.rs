//! `LoopDriver` implementation for `Agent`.
//!
//! This bridges the generic agent loop with the concrete Agent struct.

use std::collections::HashMap;
use std::sync::Arc;

use anyhow::Result;
use async_trait::async_trait;
use futures::stream::BoxStream;
use rmcp::model::{CallToolRequestParams, ErrorData, Tool};
use tokio_util::sync::CancellationToken;

use super::agent::{Agent, ToolStream};
use super::agent_loop::LoopDriver;
use super::tool_execution::ToolCallResult;
use crate::agents::types::SessionConfig;
use crate::config::GooseMode;
use crate::conversation::message::{Message, ToolRequest};
use crate::conversation::Conversation;
use crate::permission::permission_judge::PermissionCheckResult;
use crate::providers::base::Provider;
use crate::providers::errors::ProviderError;
use crate::session::{Session, SessionManager};
use crate::tool_inspection::InspectionResult;

#[async_trait]
impl LoopDriver for Agent {
    fn provider(&self) -> Arc<dyn Provider> {
        // This panics if provider is not set — callers should ensure it's set.
        // We use try_lock + clone since the async provider() method can't be used
        // in a sync trait method. The provider is set once at startup and rarely changes.
        self.provider
            .try_lock()
            .expect("provider lock poisoned")
            .as_ref()
            .expect("provider not set")
            .clone()
    }

    fn session_manager(&self) -> Arc<SessionManager> {
        self.config.session_manager.clone()
    }

    async fn is_frontend_tool(&self, name: &str) -> bool {
        self.is_frontend_tool(name).await
    }

    async fn dispatch_tool_call(
        &self,
        tool_call: CallToolRequestParams,
        request_id: String,
        cancel_token: Option<CancellationToken>,
        session: &Session,
    ) -> (String, Result<ToolCallResult, ErrorData>) {
        self.dispatch_tool_call(tool_call, request_id, cancel_token, session)
            .await
    }

    async fn inspect_tools(
        &self,
        session_id: &str,
        requests: &[ToolRequest],
        messages: &[Message],
        goose_mode: GooseMode,
    ) -> Result<Vec<InspectionResult>> {
        self.tool_inspection_manager
            .inspect_tools(session_id, requests, messages, goose_mode)
            .await
    }

    fn process_permissions(
        &self,
        requests: &[ToolRequest],
        inspection_results: &[InspectionResult],
    ) -> Option<PermissionCheckResult> {
        self.tool_inspection_manager
            .process_inspection_results_with_permission_inspector(requests, inspection_results)
    }

    async fn handle_approved_and_denied_tools(
        &self,
        permission_check_result: &PermissionCheckResult,
        request_to_response_map: &mut HashMap<String, Message>,
        cancel_token: Option<CancellationToken>,
        session: &Session,
    ) -> Result<Vec<(String, ToolStream)>> {
        self.handle_approved_and_denied_tools(
            permission_check_result,
            request_to_response_map,
            cancel_token,
            session,
        )
        .await
    }

    fn handle_approval_tool_requests<'a>(
        &'a self,
        tool_requests: &'a [ToolRequest],
        tool_futures: &'a mut Vec<(String, ToolStream)>,
        request_to_response_map: &'a mut HashMap<String, Message>,
        cancel_token: Option<CancellationToken>,
        session: &'a Session,
        inspection_results: &'a [InspectionResult],
    ) -> BoxStream<'a, Result<Message>> {
        self.handle_approval_tool_requests(
            tool_requests,
            tool_futures,
            request_to_response_map,
            cancel_token,
            session,
            inspection_results,
        )
    }

    async fn drain_elicitation_messages(&self, session_id: &str) -> Vec<Message> {
        self.drain_elicitation_messages(session_id).await
    }

    async fn save_extension_state(&self, session_config: &SessionConfig) -> Result<()> {
        self.save_extension_state(session_config).await
    }

    async fn prepare_tools_and_prompt(
        &self,
        session_id: &str,
        working_dir: &std::path::Path,
    ) -> Result<(Vec<Tool>, Vec<Tool>, String)> {
        self.prepare_tools_and_prompt(session_id, working_dir).await
    }

    async fn inject_moim(
        &self,
        session_id: &str,
        conversation: Conversation,
        working_dir: &std::path::Path,
    ) -> Conversation {
        super::moim::inject_moim(session_id, conversation, &self.extension_manager, working_dir)
            .await
    }

    async fn stream_response(
        &self,
        session_id: &str,
        system_prompt: &str,
        messages: &[Message],
        tools: &[Tool],
        toolshim_tools: &[Tool],
    ) -> Result<crate::providers::base::MessageStream, ProviderError> {
        let provider = self
            .provider
            .lock()
            .await
            .as_ref()
            .ok_or_else(|| {
                ProviderError::RequestFailed("Provider not set".to_string())
            })?
            .clone();
        Self::stream_response_from_provider(
            provider,
            session_id,
            system_prompt,
            messages,
            tools,
            toolshim_tools,
        )
        .await
    }

    async fn load_subdirectory_hints(&self, working_dir: &std::path::Path) -> bool {
        self.prompt_manager
            .lock()
            .await
            .load_subdirectory_hints(working_dir)
    }
}
