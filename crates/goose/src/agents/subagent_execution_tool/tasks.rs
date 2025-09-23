use serde_json::Value;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;
use tracing::debug;

use crate::agents::subagent_execution_tool::task_execution_tracker::TaskExecutionTracker;
use crate::agents::subagent_execution_tool::task_types::{Task, TaskResult, TaskStatus, TaskType};
use crate::agents::subagent_execution_tool::utils::strip_ansi_codes;
use crate::agents::subagent_task_config::TaskConfig;
use crate::agents::{Agent, AgentEvent, SessionConfig};
use crate::conversation::message::Message;
use crate::conversation::Conversation;
use crate::providers::base::Provider;
use crate::session;

pub async fn process_task(
    task: &Task,
    task_execution_tracker: Arc<TaskExecutionTracker>,
    task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> TaskResult {
    match get_task_result(
        task.clone(),
        task_execution_tracker,
        task_config,
        cancellation_token,
    )
    .await
    {
        Ok(data) => TaskResult {
            task_id: task.id.clone(),
            status: TaskStatus::Completed,
            data: Some(data),
            error: None,
        },
        Err(error) => TaskResult {
            task_id: task.id.clone(),
            status: TaskStatus::Failed,
            data: None,
            error: Some(error),
        },
    }
}

async fn get_task_result(
    task: Task,
    task_execution_tracker: Arc<TaskExecutionTracker>,
    task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> Result<Value, String> {
    match task.task_type {
        TaskType::InlineRecipe => {
            handle_inline_recipe_task(task, task_config, cancellation_token).await
        }
        TaskType::SubRecipe => {
            let (command, output_identifier) = build_command(&task)?;
            let (stdout_output, stderr_output, success) = run_command(
                command,
                &output_identifier,
                &task.id,
                task_execution_tracker,
                cancellation_token,
            )
            .await?;

            if success {
                process_output(stdout_output)
            } else {
                Err(format!("Command failed:\n{}", &stderr_output))
            }
        }
    }
}

async fn handle_inline_recipe_task(
    task: Task,
    task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> Result<Value, String> {
    use crate::recipe::Recipe;

    let recipe_value = task
        .payload
        .get("recipe")
        .ok_or_else(|| "Missing recipe in inline_recipe task payload".to_string())?;

    let recipe: Recipe = serde_json::from_value(recipe_value.clone())
        .map_err(|e| format!("Invalid recipe in payload: {}", e))?;

    let return_last_only = task
        .payload
        .get("return_last_only")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    let agent: Agent = Agent::new();
    let agent_provider: Arc<dyn Provider> = task_config
        .provider
        .ok_or_else(|| "No provider configured for subagent".to_string())?;
    if let Err(e) = agent.update_provider(agent_provider).await {
        return Err(format!("Failed to set provider on sub agent: {}", e));
    }

    if let Some(recipe_extensions) = recipe.extensions {
        for extension in recipe_extensions {
            if let Err(e) = agent.add_extension(extension.clone()).await {
                debug!(
                    "Failed to add extension '{}' to subagent: {}",
                    extension.name(),
                    e
                );
                // Continue with other extensions even if one fails
            }
        }
    }

    let session_id_for_return = session::generate_session_id();
    let session_file_path = match crate::session::storage::get_path(
        crate::session::storage::Identifier::Name(session_id_for_return.clone()),
    ) {
        Ok(path) => path,
        Err(e) => {
            return Err(format!("Failed to get sub agent session file path: {}", e));
        }
    };

    let instruction = recipe
        .instructions
        .or(recipe.prompt)
        .ok_or_else(|| "No instructions or prompt in recipe".to_string())?;

    let mut session_messages =
        Conversation::new_unvalidated(vec![Message::user().with_text(instruction.clone())]);

    let current_dir = match std::env::current_dir() {
        Ok(cd) => cd,
        Err(e) => {
            return Err(format!(
                "Failed to get current directory for sub agent: {}",
                e
            ));
        }
    };
    let session_config = SessionConfig {
        id: crate::session::storage::Identifier::Name(session_id_for_return.clone()),
        working_dir: current_dir.clone(),
        schedule_id: None,
        execution_mode: None,
        max_turns: task_config.max_turns.map(|v| v as u32),
        retry_config: None,
    };

    let reply_result = agent
        .reply(
            session_messages.clone(),
            Some(session_config.clone()),
            Some(cancellation_token.clone()),
        )
        .await;

    match reply_result {
        Ok(mut stream) => {
            use futures::StreamExt;

            while let Some(message_result) = stream.next().await {
                match message_result {
                    Ok(AgentEvent::Message(msg)) => {
                        session_messages.push(msg);
                    }
                    Ok(AgentEvent::McpNotification(_)) => {
                        // Handle notifications if needed
                    }
                    Ok(AgentEvent::ModelChange { .. }) => {
                        // Model change events are informational, just continue
                    }
                    Ok(AgentEvent::HistoryReplaced(_)) => {
                        // Handle history replacement events if needed
                    }
                    Err(e) => {
                        tracing::error!("Error receiving message from subagent: {}", e);
                        break;
                    }
                }
            }

            // Extract text content based on return_last_only flag
            let response_text = if return_last_only {
                // Get only the last message's text content
                session_messages
                    .messages()
                    .last()
                    .and_then(|message| {
                        message.content.iter().find_map(|content| match content {
                            crate::conversation::message::MessageContent::Text(text_content) => {
                                Some(text_content.text.clone())
                            }
                            _ => None,
                        })
                    })
                    .unwrap_or_else(|| String::from("No text content in last message"))
            } else {
                // Extract all text content from all messages (original behavior)
                let all_text_content: Vec<String> = session_messages
                    .iter()
                    .flat_map(|message| {
                        message.content.iter().filter_map(|content| match content {
                            crate::conversation::message::MessageContent::Text(text_content) => {
                                Some(text_content.text.clone())
                            }
                            crate::conversation::message::MessageContent::ToolResponse(
                                tool_response,
                            ) => tool_response
                                .tool_result
                                .as_ref()
                                .ok()
                                .map(|contents| {
                                    contents
                                        .iter()
                                        .filter_map(|content| match &content.raw {
                                            rmcp::model::RawContent::Text(raw_text_content) => {
                                                Some(raw_text_content.text.clone())
                                            }
                                            _ => None,
                                        })
                                        .collect::<Vec<String>>()
                                })
                                .filter(|texts: &Vec<String>| !texts.is_empty())
                                .map(|texts| format!("Tool result: {}", texts.join("\n"))),
                            _ => None,
                        })
                    })
                    .collect();

                all_text_content.join("\n")
            };

            match crate::session::storage::read_metadata(&session_file_path) {
                Ok(mut updated_metadata) => {
                    updated_metadata.message_count = session_messages.len();
                    if let Err(e) = crate::session::storage::save_messages_with_metadata(
                        &session_file_path,
                        &updated_metadata,
                        &session_messages,
                    ) {
                        tracing::error!("Failed to persist final messages: {}", e);
                        return Err(format!("Failed to save messages: {}", e));
                    }
                    Ok(serde_json::json!({"result": response_text}))
                }
                Err(e) => {
                    tracing::error!("Failed to read updated metadata before final save: {}", e);
                    Err(format!("Failed to read metadata: {}", e))
                }
            }
        }
        Err(e) => {
            return Err(format!("Agent failed to reply for subagent: {}", e));
        }
    }
}

fn build_command(task: &Task) -> Result<(Command, String), String> {
    let task_error = |field: &str| format!("Task {}: Missing {}", task.id, field);

    if !matches!(task.task_type, TaskType::SubRecipe) {
        return Err("Only sub-recipe tasks can be executed as commands".to_string());
    }

    let sub_recipe_name = task
        .get_sub_recipe_name()
        .ok_or_else(|| task_error("sub_recipe name"))?;
    let path = task
        .get_sub_recipe_path()
        .ok_or_else(|| task_error("sub_recipe path"))?;
    let command_parameters = task
        .get_command_parameters()
        .ok_or_else(|| task_error("command_parameters"))?;

    let mut command = Command::new("goose");
    command
        .arg("run")
        .arg("--recipe")
        .arg(path)
        .arg("--no-session");

    for (key, value) in command_parameters {
        let key_str = key.to_string();
        let value_str = value.as_str().unwrap_or(&value.to_string()).to_string();
        command
            .arg("--params")
            .arg(format!("{}={}", key_str, value_str));
    }

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());

    Ok((command, format!("sub-recipe {}", sub_recipe_name)))
}

async fn run_command(
    mut command: Command,
    output_identifier: &str,
    task_id: &str,
    task_execution_tracker: Arc<TaskExecutionTracker>,
    cancellation_token: CancellationToken,
) -> Result<(String, String, bool), String> {
    let mut child = command
        .spawn()
        .map_err(|e| format!("Failed to spawn goose: {}", e))?;

    let stdout = child.stdout.take().expect("Failed to capture stdout");
    let stderr = child.stderr.take().expect("Failed to capture stderr");

    let stdout_task = spawn_output_reader(
        stdout,
        output_identifier,
        false,
        task_id,
        task_execution_tracker.clone(),
    );
    let stderr_task = spawn_output_reader(
        stderr,
        output_identifier,
        true,
        task_id,
        task_execution_tracker.clone(),
    );

    let result = tokio::select! {
        _ = cancellation_token.cancelled() => {
            if let Err(e) = child.kill().await {
                tracing::warn!("Failed to kill child process: {}", e);
            }

            stdout_task.abort();
            stderr_task.abort();
            return Err("Command cancelled".to_string());
        }
        status_result = child.wait() => {
            status_result.map_err(|e| format!("Failed to wait for process: {}", e))?
        }
    };

    let stdout_output = stdout_task.await.unwrap();
    let stderr_output = stderr_task.await.unwrap();

    Ok((stdout_output, stderr_output, result.success()))
}

fn spawn_output_reader(
    reader: impl tokio::io::AsyncRead + Unpin + Send + 'static,
    output_identifier: &str,
    is_stderr: bool,
    task_id: &str,
    task_execution_tracker: Arc<TaskExecutionTracker>,
) -> tokio::task::JoinHandle<String> {
    let output_identifier = output_identifier.to_string();
    let task_id = task_id.to_string();
    tokio::spawn(async move {
        let mut buffer = String::new();
        let mut lines = BufReader::new(reader).lines();
        while let Ok(Some(line)) = lines.next_line().await {
            let line = strip_ansi_codes(&line);
            buffer.push_str(&line);
            buffer.push('\n');

            if !is_stderr {
                task_execution_tracker
                    .send_live_output(&task_id, &line)
                    .await;
            } else {
                tracing::warn!("Task stderr [{}]: {}", output_identifier, line);
            }
        }
        buffer
    })
}

fn extract_json_from_line(line: &str) -> Option<String> {
    let start = line.find('{')?;
    let end = line.rfind('}')?;

    if start >= end {
        return None;
    }

    let potential_json = &line[start..=end];
    if serde_json::from_str::<Value>(potential_json).is_ok() {
        Some(potential_json.to_string())
    } else {
        None
    }
}

fn process_output(stdout_output: String) -> Result<Value, String> {
    let last_line = stdout_output
        .lines()
        .filter(|line| !line.trim().is_empty())
        .next_back()
        .unwrap_or("");

    if let Some(json_string) = extract_json_from_line(last_line) {
        Ok(Value::String(json_string))
    } else {
        Ok(Value::String(stdout_output))
    }
}
