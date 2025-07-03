use super::Session;
use anyhow::Result;
use console::Color;
use futures::StreamExt;
use goose::agents::{AgentEvent, SessionConfig};
use goose::config::Config;
use goose::message::{Message, MessageContent};
use goose::permission::permission_confirmation::PrincipalType;
use goose::permission::{Permission, PermissionConfirmation};
use goose::session;
use mcp_core::handler::ToolError;
use mcp_core::protocol::{JsonRpcMessage, JsonRpcNotification};
use serde_json::Value;

use crate::session::output;

impl Session {
    /// Process agent response stream and handle all events
    pub(crate) async fn process_agent_response(&mut self, interactive: bool) -> Result<()> {
        let session_config = self.session_file.as_ref().map(|s| {
            let session_id = session::Identifier::Path(s.clone());
            SessionConfig {
                id: session_id.clone(),
                working_dir: std::env::current_dir()
                    .expect("failed to get current session working directory"),
                schedule_id: self.scheduled_job_id.clone(),
                execution_mode: None,
                max_turns: self.max_turns,
            }
        });
        let mut stream = self
            .agent
            .reply(&self.messages, session_config.clone())
            .await?;

        let mut progress_bars = output::McpSpinners::new();

        loop {
            tokio::select! {
                result = stream.next() => {
                    match result {
                        Some(Ok(AgentEvent::Message(message))) => {
                            // If it's a confirmation request, get approval but otherwise do not render/persist
                            if let Some(MessageContent::ToolConfirmationRequest(confirmation)) = message.content.first() {
                                output::hide_thinking();

                                // Format the confirmation prompt
                                let prompt = "Goose would like to call the above tool, do you allow?".to_string();

                                // Get confirmation from user
                                let permission_result = cliclack::select(prompt)
                                    .item(Permission::AllowOnce, "Allow", "Allow the tool call once")
                                    .item(Permission::AlwaysAllow, "Always Allow", "Always allow the tool call")
                                    .item(Permission::DenyOnce, "Deny", "Deny the tool call")
                                    .item(Permission::Cancel, "Cancel", "Cancel the AI response and tool call")
                                    .interact();

                                let permission = match permission_result {
                                    Ok(p) => p, // If Ok, use the selected permission
                                    Err(e) => {
                                        // Check if the error is an interruption (Ctrl+C/Cmd+C, Escape)
                                        if e.kind() == std::io::ErrorKind::Interrupted {
                                            Permission::Cancel // If interrupted, set permission to Cancel
                                        } else {
                                            return Err(e.into()); // Otherwise, convert and propagate the original error
                                        }
                                    }
                                };

                                if permission == Permission::Cancel {
                                    output::render_text("Tool call cancelled. Returning to chat...", Some(Color::Yellow), true);

                                    let mut response_message = Message::user();
                                    response_message.content.push(MessageContent::tool_response(
                                        confirmation.id.clone(),
                                        Err(ToolError::ExecutionError("Tool call cancelled by user".to_string()))
                                    ));
                                    self.messages.push(response_message);
                                    if let Some(session_file) = &self.session_file {
                                        session::persist_messages_with_schedule_id(
                                            session_file,
                                            &self.messages,
                                            None,
                                            self.scheduled_job_id.clone(),
                                        )
                                        .await?;
                                    }

                                    drop(stream);
                                    break;
                                } else {
                                    self.agent.handle_confirmation(confirmation.id.clone(), PermissionConfirmation {
                                        principal_type: PrincipalType::Tool,
                                        permission,
                                    },).await;
                                }
                            } else if let Some(MessageContent::ContextLengthExceeded(_)) = message.content.first() {
                                output::hide_thinking();

                                // Check for user-configured default context strategy
                                let config = Config::global();
                                let context_strategy = config.get_param::<String>("GOOSE_CONTEXT_STRATEGY")
                                    .unwrap_or_else(|_| if interactive { "prompt".to_string() } else { "summarize".to_string() });

                                let selected = match context_strategy.as_str() {
                                    "clear" => "clear",
                                    "truncate" => "truncate",
                                    "summarize" => "summarize",
                                    _ => {
                                        if interactive {
                                            // In interactive mode with no default, ask the user what to do
                                            let prompt = "The model's context length is maxed out. You will need to reduce the # msgs. Do you want to?".to_string();
                                            cliclack::select(prompt)
                                                .item("clear", "Clear Session", "Removes all messages from Goose's memory")
                                                .item("truncate", "Truncate Messages", "Removes old messages till context is within limits")
                                                .item("summarize", "Summarize Session", "Summarize the session to reduce context length")
                                                .interact()?
                                        } else {
                                            // In headless mode, default to summarize
                                            "summarize"
                                        }
                                    }
                                };

                                match selected {
                                    "clear" => {
                                        self.messages.clear();
                                        let msg = if context_strategy == "clear" {
                                            format!("Context maxed out - automatically cleared session.\n{}", "-".repeat(50))
                                        } else {
                                            format!("Session cleared.\n{}", "-".repeat(50))
                                        };
                                        output::render_text(&msg, Some(Color::Yellow), true);
                                        break;  // exit the loop to hand back control to the user
                                    }
                                    "truncate" => {
                                        // Truncate messages to fit within context length
                                        let (truncated_messages, _) = self.agent.truncate_context(&self.messages).await?;
                                        let msg = if context_strategy == "truncate" {
                                            format!("Context maxed out - automatically truncated messages.\n{}\nGoose tried its best to truncate messages for you.", "-".repeat(50))
                                        } else {
                                            format!("Context maxed out\n{}\nGoose tried its best to truncate messages for you.", "-".repeat(50))
                                        };
                                        output::render_text("", Some(Color::Yellow), true);
                                        output::render_text(&msg, Some(Color::Yellow), true);
                                        self.messages = truncated_messages;
                                    }
                                    "summarize" => {
                                        // Use the helper function to summarize context
                                        let message_suffix = if context_strategy == "summarize" {
                                            "Goose automatically summarized messages for you."
                                        } else if interactive {
                                            "Goose summarized messages for you."
                                        } else {
                                            "Goose automatically summarized messages to continue processing."
                                        };
                                        crate::session::commands::context::summarize_context_messages(&mut self.messages, &self.agent, message_suffix).await?;
                                    }
                                    _ => {
                                        unreachable!()
                                    }
                                }

                                // Restart the stream after handling ContextLengthExceeded
                                stream = self
                                    .agent
                                    .reply(
                                        &self.messages,
                                        session_config.clone(),
                                    )
                                    .await?;
                            }
                            // otherwise we have a model/tool to render
                            else {
                                self.messages.push(message.clone());

                                // No need to update description on assistant messages
                                if let Some(session_file) = &self.session_file {
                                    session::persist_messages_with_schedule_id(
                                        session_file,
                                        &self.messages,
                                        None,
                                        self.scheduled_job_id.clone(),
                                    )
                                    .await?;
                                }

                                if interactive {output::hide_thinking()};
                                let _ = progress_bars.hide();
                                output::render_message(&message, self.debug);
                                if interactive {output::show_thinking()};
                            }
                        }
                        Some(Ok(AgentEvent::McpNotification((_id, message)))) => {
                                if let JsonRpcMessage::Notification(JsonRpcNotification{
                                    method,
                                    params: Some(Value::Object(o)),
                                    ..
                                }) = message {
                                match method.as_str() {
                                    "notifications/message" => {
                                        let data = o.get("data").unwrap_or(&Value::Null);
                                        let (formatted_message, subagent_id, _notification_type) = match data {
                                            Value::String(s) => (s.clone(), None, None),
                                            Value::Object(o) => {
                                                // Check for subagent notification structure first
                                                if let Some(Value::String(msg)) = o.get("message") {
                                                    // Extract subagent info for better display
                                                    let subagent_id = o.get("subagent_id")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("unknown");
                                                    let notification_type = o.get("type")
                                                        .and_then(|v| v.as_str())
                                                        .unwrap_or("");

                                                    let formatted = match notification_type {
                                                        "subagent_created" | "completed" | "terminated" => {
                                                            format!("🤖 {}", msg)
                                                        }
                                                        "tool_usage" | "tool_completed" | "tool_error" => {
                                                            format!("🔧 {}", msg)
                                                        }
                                                        "message_processing" | "turn_progress" => {
                                                            format!("💭 {}", msg)
                                                        }
                                                        "response_generated" => {
                                                            // Check verbosity setting for subagent response content
                                                            let config = Config::global();
                                                            let min_priority = config
                                                                .get_param::<f32>("GOOSE_CLI_MIN_PRIORITY")
                                                                .ok()
                                                                .unwrap_or(0.5);

                                                            if min_priority > 0.1 && !self.debug {
                                                                // High/Medium verbosity: show truncated response
                                                                if let Some(response_content) = msg.strip_prefix("Responded: ") {
                                                                    if response_content.len() > 100 {
                                                                        format!("🤖 Responded: {}...", &response_content[..100])
                                                                    } else {
                                                                        format!("🤖 {}", msg)
                                                                    }
                                                                } else {
                                                                    format!("🤖 {}", msg)
                                                                }
                                                            } else {
                                                                // All verbosity or debug: show full response
                                                                format!("🤖 {}", msg)
                                                            }
                                                        }
                                                        _ => {
                                                            msg.to_string()
                                                        }
                                                    };
                                                    (formatted, Some(subagent_id.to_string()), Some(notification_type.to_string()))
                                                } else if let Some(Value::String(output)) = o.get("output") {
                                                    // Fallback for other MCP notification types
                                                    (output.to_owned(), None, None)
                                                } else {
                                                    (data.to_string(), None, None)
                                                }
                                            },
                                            v => {
                                                (v.to_string(), None, None)
                                            },
                                        };

                                        // Handle subagent notifications - show immediately
                                        if let Some(_id) = subagent_id {
                                            // Show subagent notifications immediately (no buffering) with compact spacing
                                            if interactive {
                                                let _ = progress_bars.hide();
                                                println!("{}", console::style(&formatted_message).green().dim());
                                            } else {
                                                progress_bars.log(&formatted_message);
                                            }
                                        } else {
                                            // Non-subagent notification, display immediately with compact spacing
                                            if interactive {
                                                let _ = progress_bars.hide();
                                                println!("{}", console::style(&formatted_message).green().dim());
                                            } else {
                                                progress_bars.log(&formatted_message);
                                            }
                                        }
                                    },
                                    "notifications/progress" => {
                                        let progress = o.get("progress").and_then(|v| v.as_f64());
                                        let token = o.get("progressToken").map(|v| v.to_string());
                                        let message = o.get("message").and_then(|v| v.as_str());
                                        let total = o
                                            .get("total")
                                            .and_then(|v| v.as_f64());
                                        if let (Some(progress), Some(token)) = (progress, token) {
                                            progress_bars.update(
                                                token.as_str(),
                                                progress,
                                                total,
                                                message,
                                            );
                                        }
                                    },
                                    _ => (),
                                }
                            }
                        }
                        Some(Ok(AgentEvent::ModelChange { model, mode })) => {
                            // Log model change if in debug mode
                            if self.debug {
                                eprintln!("Model changed to {} in {} mode", model, mode);
                            }
                        }

                        Some(Err(e)) => {
                            eprintln!("Error: {}", e);
                            drop(stream);
                            if let Err(e) = self.handle_interrupted_messages(false).await {
                                eprintln!("Error handling interruption: {}", e);
                            }
                            output::render_error(
                                "The error above was an exception we were not able to handle.\n\
                                These errors are often related to connection or authentication\n\
                                We've removed the conversation up to the most recent user message\n\
                                - depending on the error you may be able to continue",
                            );
                            break;
                        }
                        None => break,
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    drop(stream);
                    if let Err(e) = self.handle_interrupted_messages(true).await {
                        eprintln!("Error handling interruption: {}", e);
                    }
                    break;
                }
            }
        }

        Ok(())
    }
}
