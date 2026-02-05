//! Session notification - real-time updates to clients
//!
//! The notifier is responsible for sending streaming updates to the client
//! as the agent processes a prompt. This includes text chunks, tool calls,
//! and tool results.

use agent_client_protocol_schema::{
    Content, ContentBlock, ContentChunk, SessionId, SessionNotification, SessionUpdate,
    TextContent, ToolCall as AcpToolCall, ToolCallContent, ToolCallId, ToolCallStatus,
};
use rig::message::ToolCall;
use sacp::role::HasPeer;
use sacp::{Client, ConnectionTo, Role};

use crate::{Error, Result};

/// Trait for sending session notifications back to the client
///
/// This is used by the agent loop to stream updates in real-time.
/// Implementations handle the actual transport (ACP, test mocks, etc.)
pub trait Notifier: Send + Sync {
    /// Send a text chunk (streaming text output)
    fn send_text_chunk(
        &self,
        session_id: &SessionId,
        text: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Send notification that a tool is being called
    fn send_tool_use(
        &self,
        session_id: &SessionId,
        tool_call: &ToolCall,
    ) -> impl std::future::Future<Output = Result<()>> + Send;

    /// Send the result of a tool call
    fn send_tool_result(
        &self,
        session_id: &SessionId,
        tool_call: &ToolCall,
        result: &str,
    ) -> impl std::future::Future<Output = Result<()>> + Send;
}

/// ACP-based notifier implementation
///
/// Sends notifications over the ACP connection to the client.
pub struct AcpNotifier<Link: Role> {
    connection: ConnectionTo<Link>,
}

impl<Link: Role> AcpNotifier<Link> {
    pub fn new(connection: ConnectionTo<Link>) -> Self {
        Self { connection }
    }
}

impl<Link> Notifier for AcpNotifier<Link>
where
    Link: Role + HasPeer<Client> + Send + Sync,
{
    async fn send_text_chunk(&self, session_id: &SessionId, text: &str) -> Result<()> {
        let notification = SessionNotification::new(
            session_id.clone(),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(ContentBlock::Text(
                TextContent::new(text),
            ))),
        );

        self.connection
            .send_notification_to(Client, notification)
            .map_err(|e| Error::Internal(format!("Failed to send text chunk: {}", e)))?;

        Ok(())
    }

    async fn send_tool_use(&self, session_id: &SessionId, tool_call: &ToolCall) -> Result<()> {
        let acp_tool_call = AcpToolCall::new(
            ToolCallId::new(tool_call.id.clone()),
            format!("Calling {}", tool_call.function.name),
        )
        .status(ToolCallStatus::InProgress)
        .raw_input(Some(tool_call.function.arguments.clone()));

        let notification = SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ToolCall(acp_tool_call),
        );

        self.connection
            .send_notification_to(Client, notification)
            .map_err(|e| Error::Internal(format!("Failed to send tool use: {}", e)))?;

        Ok(())
    }

    async fn send_tool_result(
        &self,
        session_id: &SessionId,
        tool_call: &ToolCall,
        result: &str,
    ) -> Result<()> {
        let acp_tool_call = AcpToolCall::new(
            ToolCallId::new(tool_call.id.clone()),
            format!("Completed {}", tool_call.function.name),
        )
        .status(ToolCallStatus::Completed)
        .content(vec![ToolCallContent::Content(Content::new(
            ContentBlock::Text(TextContent::new(result)),
        ))]);

        let notification = SessionNotification::new(
            session_id.clone(),
            SessionUpdate::ToolCall(acp_tool_call),
        );

        self.connection
            .send_notification_to(Client, notification)
            .map_err(|e| Error::Internal(format!("Failed to send tool result: {}", e)))?;

        Ok(())
    }
}

/// No-op notifier for testing or batch processing
#[derive(Default)]
pub struct NullNotifier;

impl Notifier for NullNotifier {
    async fn send_text_chunk(&self, _session_id: &SessionId, _text: &str) -> Result<()> {
        Ok(())
    }

    async fn send_tool_use(&self, _session_id: &SessionId, _tool_call: &ToolCall) -> Result<()> {
        Ok(())
    }

    async fn send_tool_result(
        &self,
        _session_id: &SessionId,
        _tool_call: &ToolCall,
        _result: &str,
    ) -> Result<()> {
        Ok(())
    }
}
