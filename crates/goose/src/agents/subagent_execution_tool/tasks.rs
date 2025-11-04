use serde_json::Value;
use std::process::Stdio;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio_util::sync::CancellationToken;

use crate::agents::subagent_execution_tool::task_execution_tracker::TaskExecutionTracker;
use crate::agents::subagent_execution_tool::task_types::{Task, TaskResult, TaskStatus, TaskType};
use crate::agents::subagent_execution_tool::utils::strip_ansi_codes;
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
    mut task_config: TaskConfig,
    cancellation_token: CancellationToken,
) -> Result<Value, String> {
    use crate::agents::subagent_handler::run_complete_subagent_task;
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

    // If extensions are explicitly provided in the recipe (even if empty),
    // override the task_config extensions. Empty array means no extensions.
    if let Some(exts) = recipe.extensions {
        task_config.extensions = exts.clone();
    }

    let instruction = recipe
        .instructions
        .or(recipe.prompt)
        .ok_or_else(|| "No instructions or prompt in recipe".to_string())?;

    let result = tokio::select! {
        result = run_complete_subagent_task(
            instruction,
            task_config,
            return_last_only,
        ) => result,
        _ = cancellation_token.cancelled() => {
            return Err("Task cancelled".to_string());
        }
    };

    match result {
        Ok(result_text) => Ok(serde_json::json!({
            "result": result_text
        })),
        Err(e) => {
            let error_msg = format!("Inline recipe execution failed: {}", e);
            Err(error_msg)
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

    let potential_json = line.get(start..=end)?;
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
