use crate::rate_limiter::acquire_global_permit;
use anyhow::Result;
use futures::stream::StreamExt;
use goose::agents::{Agent, AgentEvent};
use goose::message::{Message, MessageContent};

pub struct InteractionLimitedAgent {
    agent: Agent,
    messages: Vec<Message>,
    max_interactions: Option<usize>,
}

impl InteractionLimitedAgent {
    pub fn new(agent: Agent, max_interactions: Option<usize>) -> Self {
        Self {
            agent,
            messages: Vec::new(),
            max_interactions,
        }
    }

    pub async fn prompt_with_limit(
        &mut self,
        prompt: String,
        max_interactions: usize,
    ) -> Result<Vec<Message>> {
        self.process_prompt(prompt, Some(max_interactions)).await
    }

    async fn process_prompt(
        &mut self,
        prompt: String,
        max_interactions: Option<usize>,
    ) -> Result<Vec<Message>> {
        let limit = max_interactions
            .or(self.max_interactions)
            .unwrap_or(usize::MAX);

        if limit == 0 {
            return Ok(self.messages.clone());
        }

        self.messages.push(Message::user().with_text(&prompt));

        // Acquire rate limit permit BEFORE making the LLM request
        let _permit = acquire_global_permit().await;

        let mut reply_stream = self.agent.reply(&self.messages, None).await?;
        let mut interactions_count = 0;
        let mut collected_messages = Vec::new();

        while let Some(event) = reply_stream.next().await {
            if let AgentEvent::Message(msg) = event? {
                let has_tool_requests = msg
                    .content
                    .iter()
                    .any(|c| matches!(c, MessageContent::ToolRequest(_)));

                if has_tool_requests {
                    interactions_count += 1;

                    collected_messages.push(msg);

                    if interactions_count >= limit {
                        break;
                    }
                } else {
                    // Count non-tool assistant messages as interactions
                    let has_tool_responses = msg
                        .content
                        .iter()
                        .any(|c| matches!(c, MessageContent::ToolResponse(_)));

                    if !has_tool_responses && matches!(msg.role, mcp_core::role::Role::Assistant) {
                        interactions_count += 1;

                        collected_messages.push(msg);

                        if interactions_count >= limit {
                            break;
                        }
                    } else {
                        // Tool responses and other messages
                        collected_messages.push(msg);
                    }
                }
            }
        }

        self.messages.extend(collected_messages);

        Ok(self.messages.clone())
    }

    pub async fn prompt(&mut self, prompt: String) -> Result<Vec<Message>> {
        // Use unlimited interactions for regular prompt
        self.process_prompt(prompt, None).await
    }

    pub async fn prompt_multi_turn(&mut self, prompts: Vec<String>) -> Result<Vec<Message>> {
        if prompts.is_empty() {
            return Err(anyhow::anyhow!("At least one prompt is required"));
        }

        let mut prompt_iter = prompts.into_iter();
        let first_prompt = prompt_iter.next().unwrap();

        // Send the first prompt
        self.messages.push(Message::user().with_text(&first_prompt));

        let remaining_prompts: Vec<String> = prompt_iter.collect();
        let mut remaining_prompt_index = 0;

        loop {
            // Acquire rate limit permit BEFORE making the LLM request
            let _permit = acquire_global_permit().await;

            let mut reply_stream = self.agent.reply(&self.messages, None).await?;
            let mut collected_messages = Vec::new();
            let mut got_non_tool_response = false;

            while let Some(event) = reply_stream.next().await {
                if let AgentEvent::Message(msg) = event? {
                    let has_tool_requests = msg
                        .content
                        .iter()
                        .any(|c| matches!(c, MessageContent::ToolRequest(_)));

                    let has_tool_responses = msg
                        .content
                        .iter()
                        .any(|c| matches!(c, MessageContent::ToolResponse(_)));

                    collected_messages.push(msg.clone());

                    // Check if this is a non-tool assistant response
                    if !has_tool_requests
                        && !has_tool_responses
                        && matches!(msg.role, mcp_core::role::Role::Assistant)
                    {
                        got_non_tool_response = true;
                    }
                }
            }

            self.messages.extend(collected_messages);

            // If we got a non-tool response and have more prompts, send the next one
            if got_non_tool_response && remaining_prompt_index < remaining_prompts.len() {
                let next_prompt = &remaining_prompts[remaining_prompt_index];
                self.messages.push(Message::user().with_text(next_prompt));
                remaining_prompt_index += 1;
            } else if remaining_prompt_index >= remaining_prompts.len() {
                // No more prompts to send
                break;
            }
        }

        Ok(self.messages.clone())
    }

    pub fn message_history(&self) -> Vec<Message> {
        self.messages.clone()
    }

    pub async fn override_system_prompt(&self, override_prompt: String) {
        self.agent.override_system_prompt(override_prompt).await;
    }

    pub fn get_agent(&self) -> &Agent {
        &self.agent
    }

    /// Reset the agent state for reuse in the agent pool
    pub async fn reset(&mut self) -> Result<()> {
        // Clear message history
        self.messages.clear();

        // Reset the underlying agent state if possible
        // The agent should clear its conversation state but keep extensions

        Ok(())
    }

    /// Clean up extension processes
    pub async fn cleanup_extensions(&self) -> Result<()> {
        tracing::debug!("Cleaning up InteractionLimitedAgent extensions");
        
        // Get list of all extensions and remove them
        let extensions = self.agent.list_extensions().await;
        
        for extension_name in extensions {
            let remove_timeout = tokio::time::Duration::from_secs(10);
            match tokio::time::timeout(remove_timeout, self.agent.remove_extension(&extension_name)).await {
                Ok(Ok(_)) => {
                    tracing::debug!(extension = %extension_name, "Extension removed successfully");
                }
                Ok(Err(e)) => {
                    tracing::warn!(extension = %extension_name, error = %e, "Failed to remove extension");
                }
                Err(_timeout) => {
                    tracing::warn!(extension = %extension_name, "Timeout removing extension");
                }
            }
        }
        
        Ok(())
    }
}
