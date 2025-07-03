use super::Session;
use anyhow::Result;
use goose::message::{Message, MessageContent};
use goose::session;
use mcp_core::handler::ToolError;
use mcp_core::role::Role;

use crate::session::output;

impl Session {
    /// Process a single message and get the response
    pub(crate) async fn process_message(&mut self, message: String) -> Result<()> {
        self.add_user_message_to_history(&message).await?;
        self.update_project_tracker(&message)?;
        self.process_agent_response(false).await?;
        Ok(())
    }

    /// Add a user message to the message history and persist it
    pub(crate) async fn add_user_message_to_history(&mut self, message: &str) -> Result<()> {
        self.messages.push(Message::user().with_text(message));

        // Get the provider from the agent for description generation
        let provider = self.agent.provider().await?;

        // Persist messages with provider for automatic description generation
        if let Some(session_file) = &self.session_file {
            session::persist_messages_with_schedule_id(
                session_file,
                &self.messages,
                Some(provider),
                self.scheduled_job_id.clone(),
            )
            .await?;
        }

        Ok(())
    }

    /// Handle interrupted messages during agent response processing
    pub(crate) async fn handle_interrupted_messages(&mut self, interrupt: bool) -> Result<()> {
        // First, get any tool requests from the last message if it exists
        let tool_requests = self
            .messages
            .last()
            .filter(|msg| msg.role == Role::Assistant)
            .map_or(Vec::new(), |msg| {
                msg.content
                    .iter()
                    .filter_map(|content| {
                        if let MessageContent::ToolRequest(req) = content {
                            Some((req.id.clone(), req.tool_call.clone()))
                        } else {
                            None
                        }
                    })
                    .collect()
            });

        if !tool_requests.is_empty() {
            // Interrupted during a tool request
            // Create tool responses for all interrupted tool requests
            let mut response_message = Message::user();
            let last_tool_name = tool_requests
                .last()
                .and_then(|(_, tool_call)| tool_call.as_ref().ok().map(|tool| tool.name.clone()))
                .unwrap_or_else(|| "tool".to_string());

            let notification = if interrupt {
                "Interrupted by the user to make a correction".to_string()
            } else {
                "An uncaught error happened during tool use".to_string()
            };
            for (req_id, _) in &tool_requests {
                response_message.content.push(MessageContent::tool_response(
                    req_id.clone(),
                    Err(ToolError::ExecutionError(notification.clone())),
                ));
            }
            self.messages.push(response_message);

            // No need for description update here
            if let Some(session_file) = &self.session_file {
                session::persist_messages_with_schedule_id(
                    session_file,
                    &self.messages,
                    None,
                    self.scheduled_job_id.clone(),
                )
                .await?;
            }

            let prompt = format!(
                "The existing call to {} was interrupted. How would you like to proceed?",
                last_tool_name
            );
            self.messages.push(Message::assistant().with_text(&prompt));

            // No need for description update here
            if let Some(session_file) = &self.session_file {
                session::persist_messages_with_schedule_id(
                    session_file,
                    &self.messages,
                    None,
                    self.scheduled_job_id.clone(),
                )
                .await?;
            }

            output::render_message(&Message::assistant().with_text(&prompt), self.debug);
        } else {
            // An interruption occurred outside of a tool request-response.
            if let Some(last_msg) = self.messages.last() {
                if last_msg.role == Role::User {
                    match last_msg.content.first() {
                        Some(MessageContent::ToolResponse(_)) => {
                            // Interruption occurred after a tool had completed but not assistant reply
                            let prompt = "The tool calling loop was interrupted. How would you like to proceed?";
                            self.messages.push(Message::assistant().with_text(prompt));

                            // No need for description update here
                            if let Some(session_file) = &self.session_file {
                                session::persist_messages_with_schedule_id(
                                    session_file,
                                    &self.messages,
                                    None,
                                    self.scheduled_job_id.clone(),
                                )
                                .await?;
                            }

                            output::render_message(
                                &Message::assistant().with_text(prompt),
                                self.debug,
                            );
                        }
                        Some(_) => {
                            // A real users message
                            self.messages.pop();
                            let prompt = "Interrupted before the model replied and removed the last message.";
                            output::render_message(
                                &Message::assistant().with_text(prompt),
                                self.debug,
                            );
                        }
                        None => panic!("No content in last message"),
                    }
                }
            }
        }
        Ok(())
    }

    /// Render all past messages from the session history
    pub fn render_message_history(&self) {
        if self.messages.is_empty() {
            return;
        }

        // Print session restored message
        println!(
            "\n{} {} messages loaded into context.",
            console::style("Session restored:").green().bold(),
            console::style(self.messages.len()).green()
        );

        // Render each message
        for message in &self.messages {
            output::render_message(message, self.debug);
        }

        // Add a visual separator after restored messages
        println!(
            "\n{}\n",
            console::style("──────── New Messages ────────").dim()
        );
    }
}
