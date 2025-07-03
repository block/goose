use anyhow::Result;
use mcp_core::prompt::PromptMessage;
use serde_json::Value;
use std::collections::HashMap;

use crate::session::{input, output, Session};
use goose::message::Message;

impl Session {
    pub async fn list_prompts(
        &mut self,
        extension: Option<String>,
    ) -> Result<HashMap<String, Vec<String>>> {
        let prompts = self.agent.list_extension_prompts().await;

        // Early validation if filtering by extension
        if let Some(filter) = &extension {
            if !prompts.contains_key(filter) {
                return Err(anyhow::anyhow!("Extension '{}' not found", filter));
            }
        }

        // Convert prompts into filtered map of extension names to prompt names
        Ok(prompts
            .into_iter()
            .filter(|(ext, _)| extension.as_ref().is_none_or(|f| f == ext))
            .map(|(extension, prompt_list)| {
                let names = prompt_list.into_iter().map(|p| p.name).collect();
                (extension, names)
            })
            .collect())
    }

    pub async fn get_prompt_info(&mut self, name: &str) -> Result<Option<output::PromptInfo>> {
        let prompts = self.agent.list_extension_prompts().await;

        // Find which extension has this prompt
        for (extension, prompt_list) in prompts {
            if let Some(prompt) = prompt_list.iter().find(|p| p.name == name) {
                return Ok(Some(output::PromptInfo {
                    name: prompt.name.clone(),
                    description: prompt.description.clone(),
                    arguments: prompt.arguments.clone(),
                    extension: Some(extension),
                }));
            }
        }

        Ok(None)
    }

    pub async fn get_prompt(&mut self, name: &str, arguments: Value) -> Result<Vec<PromptMessage>> {
        let result = self.agent.get_prompt(name, arguments).await?;
        Ok(result.messages)
    }

    pub async fn handle_prompt_command(&mut self, opts: input::PromptCommandOptions) -> Result<()> {
        // name is required
        if opts.name.is_empty() {
            output::render_error("Prompt name argument is required");
            return Ok(());
        }

        if opts.info {
            match self.get_prompt_info(&opts.name).await? {
                Some(info) => output::render_prompt_info(&info),
                None => output::render_error(&format!("Prompt '{}' not found", opts.name)),
            }
        } else {
            // Convert the arguments HashMap to a Value
            let arguments = serde_json::to_value(opts.arguments)
                .map_err(|e| anyhow::anyhow!("Failed to serialize arguments: {}", e))?;

            match self.get_prompt(&opts.name, arguments).await {
                Ok(messages) => {
                    let start_len = self.messages.len();
                    let mut valid = true;
                    for (i, prompt_message) in messages.into_iter().enumerate() {
                        let msg = Message::from(prompt_message);
                        // ensure we get a User - Assistant - User type pattern
                        let expected_role = if i % 2 == 0 {
                            mcp_core::Role::User
                        } else {
                            mcp_core::Role::Assistant
                        };

                        if msg.role != expected_role {
                            output::render_error(&format!(
                                "Expected {:?} message at position {}, but found {:?}",
                                expected_role, i, msg.role
                            ));
                            valid = false;
                            // get rid of everything we added to messages
                            self.messages.truncate(start_len);
                            break;
                        }

                        if msg.role == mcp_core::Role::User {
                            output::render_message(&msg, self.debug);
                        }
                        self.messages.push(msg);
                    }

                    if valid {
                        output::show_thinking();
                        self.process_agent_response(true).await?;
                        output::hide_thinking();
                    }
                }
                Err(e) => output::render_error(&e.to_string()),
            }
        }

        Ok(())
    }
}
