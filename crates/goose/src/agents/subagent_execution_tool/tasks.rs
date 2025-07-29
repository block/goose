use serde_json::Value;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::agents::subagent_execution_tool::task_execution_tracker::TaskExecutionTracker;
use crate::agents::subagent_execution_tool::task_types::{Task, TaskResult, TaskStatus};
use crate::agents::subagent_execution_tool::utils::strip_ansi_codes;
use crate::agents::subagent_handler::run_complete_subagent_task;
use crate::agents::subagent_task_config::TaskConfig;

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
    if task.task_type == "text_instruction" {
        // Handle text_instruction tasks using subagent system
        handle_text_instruction_task(
            task,
            task_execution_tracker,
            task_config,
            cancellation_token,
        )
        .await
    } else {
        // Handle sub_recipe tasks using command execution
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

async fn handle_text_instruction_task(
    task: Task,
    task_execution_tracker: Arc<TaskExecutionTracker>,
    task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> Result<Value, String> {
    let text_instruction = task
        .get_text_instruction()
        .ok_or_else(|| format!("Task {}: Missing text_instruction", task.id))?;

    // Start tracking the task
    task_execution_tracker.start_task(&task.id).await;

    let result = tokio::select! {
        result = run_complete_subagent_task(text_instruction.to_string(), task_config) => result,
        _ = cancellation_token.cancelled() => {
            return Err("Task cancelled".to_string());
        }
    };
    match result {
        Ok(result_text) => Ok(serde_json::json!({
            "result": result_text
        })),
        Err(e) => {
            let error_msg = format!("Subagent execution failed: {}", e);
            Err(error_msg)
        }
    }
}

fn build_command(task: &Task) -> Result<(Command, String), String> {
    let task_error = |field: &str| format!("Task {}: Missing {}", task.id, field);

    let (mut command, output_identifier) = if task.task_type == "sub_recipe" {
        let sub_recipe_name = task
            .get_sub_recipe_name()
            .ok_or_else(|| task_error("sub_recipe name"))?;
        let path = task
            .get_sub_recipe_path()
            .ok_or_else(|| task_error("sub_recipe path"))?;
        let command_parameters = task
            .get_command_parameters()
            .ok_or_else(|| task_error("command_parameters"))?;

        let mut cmd = Command::new("goose");
        cmd.arg("run").arg("--recipe").arg(path).arg("--no-session");

        for (key, value) in command_parameters {
            let key_str = key.to_string();
            let value_str = value.as_str().unwrap_or(&value.to_string()).to_string();
            cmd.arg("--params")
                .arg(format!("{}={}", key_str, value_str));
        }
        (cmd, format!("sub-recipe {}", sub_recipe_name))
    } else {
        // This branch should not be reached for text_instruction tasks anymore
        // as they are handled in handle_text_instruction_task
        return Err("Text instruction tasks are handled separately".to_string());
    };

    command.stdout(Stdio::piped());
    command.stderr(Stdio::piped());
    Ok((command, output_identifier))
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
            // Abort the output reading tasks
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
