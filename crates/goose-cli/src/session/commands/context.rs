use anyhow::Result;
use console::Color;
use goose::message::Message;

use crate::session::{output, Session};
use goose::session;

impl Session {
    pub fn handle_clear_command(&mut self) {
        self.messages.clear();
        tracing::info!("Chat context cleared by user.");
        output::render_message(
            &Message::assistant().with_text("Chat context cleared."),
            self.debug,
        );
    }

    pub async fn handle_summarize_command(&mut self) -> Result<()> {
        let prompt = "Are you sure you want to summarize this conversation? This will condense the message history.";
        let should_summarize = match cliclack::confirm(prompt).initial_value(true).interact() {
            Ok(choice) => choice,
            Err(e) => {
                if e.kind() == std::io::ErrorKind::Interrupted {
                    false // If interrupted, set should_summarize to false
                } else {
                    return Err(e.into());
                }
            }
        };

        if should_summarize {
            println!("{}", console::style("Summarizing conversation...").yellow());
            output::show_thinking();

            let provider = self.agent.provider().await?;

            let (summarized_messages, _) = self.agent.summarize_context(&self.messages).await?;

            self.messages = summarized_messages;

            if let Some(session_file) = &self.session_file {
                session::persist_messages_with_schedule_id(
                    session_file,
                    &self.messages,
                    Some(provider),
                    self.scheduled_job_id.clone(),
                )
                .await?;
            }

            output::hide_thinking();
            println!(
                "{}",
                console::style("Conversation has been summarized.").green()
            );
            println!(
                "{}",
                console::style("Key information has been preserved while reducing context length.")
                    .green()
            );
        } else {
            println!("{}", console::style("Summarization cancelled.").yellow());
        }

        Ok(())
    }

    pub async fn display_context_usage(&self) -> Result<()> {
        let provider = self.agent.provider().await?;
        let model_config = provider.get_model_config();
        let context_limit = model_config.context_limit.unwrap_or(32000);

        match self.get_metadata() {
            Ok(metadata) => {
                let total_tokens = metadata.total_tokens.unwrap_or(0) as usize;

                output::display_context_usage(total_tokens, context_limit);
            }
            Err(_) => {
                output::display_context_usage(0, context_limit);
            }
        }

        Ok(())
    }
}

pub async fn summarize_context_messages(
    messages: &mut Vec<Message>,
    agent: &goose::agents::Agent,
    message_suffix: &str,
) -> Result<()> {
    let (summarized_messages, _) = agent.summarize_context(messages).await?;
    let msg = format!("Context maxed out\n{}\n{}", "-".repeat(50), message_suffix);
    output::render_text(&msg, Some(Color::Yellow), true);
    *messages = summarized_messages;

    Ok(())
}
