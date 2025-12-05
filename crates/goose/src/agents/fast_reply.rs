use anyhow::Result;
use indoc::indoc;
use std::env;
use std::sync::Arc;

use crate::conversation::message::{Message, MessageContent};
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use rmcp::model::{CallToolRequestParam, Role};
use uuid::Uuid;

pub struct FastReply;

impl FastReply {
    pub async fn fast_complete(
        provider: Arc<dyn Provider>,
        conversation: &Conversation,
    ) -> Result<Option<Message>> {
        let system_prompt = Self::build_system_prompt();
        let stripped_conversation = Self::strip_conversation(conversation);

        let (response, _usage) = provider
            .complete_fast(&system_prompt, stripped_conversation.messages(), &[])
            .await?;

        let reply_text = response.as_concat_text();
        let reply = reply_text.trim();

        if reply == "<<PASS>>" {
            return Ok(None);
        }

        if let Some(command) = reply.strip_prefix(">") {
            let mut arguments = serde_json::Map::new();
            arguments.insert("command".to_string(), serde_json::json!(command.trim()));

            let tool_call = CallToolRequestParam {
                name: "developer__execute".into(),
                arguments: Some(arguments),
            };

            let request_id = format!("f_{}", Uuid::new_v4());
            Ok(Some(
                Message::assistant().with_tool_request(request_id, Ok(tool_call)),
            ))
        } else {
            Ok(Some(Message::assistant().with_text(reply)))
        }
    }

    fn build_system_prompt() -> String {
        let shell = if cfg!(target_os = "windows") {
            "powershell"
        } else {
            &env::var("SHELL").unwrap_or_else(|_| "bash".to_string())
        };

        indoc! {"
            You can reply to the user's request in 3 ways:
             -  with a message: just return what you want to say to the user
             -  with a shell command, if the user wants you to execute something on their computer.
                in that case start with a >, like >ls -la
             -  reply with <<PASS>> to hand the request off to a more powerful agent
            You only have access to shell commands and you have a summarized history of the
            conversation. So if the request or the history is complicated or if it seems like
            other tools than shell commands are needed, just reply with <<PASS>>. There is no
            shame in that. Keep your answers short. Unless the task is simple and can be
            accomplished with a single shell command, or a one sentence answer, you MUST reply
            with <<PASS>>.
            your shell is {{SHELL}}
        "}
        .to_string()
        .replace("{{SHELL}}", shell)
    }

    fn strip_conversation(conv: &Conversation) -> Conversation {
        let mut stripped = Conversation::default();
        let messages = conv.messages();

        let user_indices: Vec<usize> = messages
            .iter()
            .enumerate()
            .filter(|(_, msg)| {
                msg.role == Role::User
                    && !msg
                        .content
                        .iter()
                        .all(|c| matches!(c, MessageContent::ToolResponse(_)))
            })
            .map(|(idx, _)| idx)
            .collect();

        if user_indices.len() < 2 {
            return conv.clone();
        }

        let keep_from_idx = user_indices[user_indices.len() - 2];

        for (idx, msg) in messages.iter().enumerate() {
            if idx >= keep_from_idx {
                stripped.push(msg.clone());
            } else {
                let content_mapped: Vec<MessageContent> = msg
                    .content
                    .iter()
                    .map(|c| match c {
                        MessageContent::ToolRequest(_) => {
                            MessageContent::text("<stripped tool call>")
                        }
                        MessageContent::ToolResponse(_) => {
                            MessageContent::text("<stripped tool result>")
                        }
                        other => other.clone(),
                    })
                    .collect();

                let new_msg = Message::new(msg.role.clone(), msg.created, content_mapped);
                stripped.push(new_msg);
            }
        }

        stripped
    }
}
