use crate::session::{build_session, SessionBuilderConfig};
use anyhow::{Context, Result};
use goose::session::session_manager::SessionType;
use goose::session::SessionManager;
use std::env;
use std::fs;
use std::path::PathBuf;

const TERMINAL_STATE_FILE: &str = "terminal_state.json";

#[derive(serde::Serialize, serde::Deserialize, Debug)]
struct TerminalState {
    session_id: String,
}

fn get_state_file_path() -> Result<PathBuf> {
    let config_dir = dirs::config_dir()
        .context("Could not determine config directory")?
        .join("goose");

    fs::create_dir_all(&config_dir)?;
    Ok(config_dir.join(TERMINAL_STATE_FILE))
}

fn load_terminal_state() -> Result<Option<TerminalState>> {
    let state_file = get_state_file_path()?;

    if !state_file.exists() {
        return Ok(None);
    }

    let contents = fs::read_to_string(&state_file)?;
    let state: TerminalState = serde_json::from_str(&contents)?;
    Ok(Some(state))
}

fn save_terminal_state(state: &TerminalState) -> Result<()> {
    let state_file = get_state_file_path()?;
    let contents = serde_json::to_string_pretty(state)?;
    fs::write(&state_file, contents)?;
    Ok(())
}

fn get_recent_shell_commands() -> Result<String> {
    let home = env::var("HOME").context("HOME environment variable not set")?;
    let history_path = PathBuf::from(home).join(".zsh_history");

    if !history_path.exists() {
        return Ok(String::new());
    }

    let contents = fs::read_to_string(&history_path)?;
    let lines: Vec<&str> = contents.lines().collect();

    // Find the last "goose" command
    let last_goose_idx = lines
        .iter()
        .rposition(|line| line.contains(";goose") || line.ends_with(";goose "));

    // Collect commands since the last goose command
    let commands: Vec<String> = if let Some(idx) = last_goose_idx {
        lines
            .iter()
            .skip(idx + 1)
            .filter_map(|line| {
                // Parse zsh history format: ": timestamp:duration;command"
                if let Some(cmd_start) = line.find(';') {
                    let cmd = &line[cmd_start + 1..];
                    if !cmd.trim().is_empty() && !cmd.starts_with("goose") {
                        return Some(cmd.to_string());
                    }
                }
                None
            })
            .collect()
    } else {
        Vec::new()
    };

    if commands.is_empty() {
        return Ok(String::new());
    }

    Ok(format!(
        "\n\nRecent shell commands run since last goose invocation:\n```\n{}\n```\n\
        Use this info if relevant to the user's message. Do not just bring it up by default,\
        but you should be ready to use info from knowing this command history when applicable",
        commands.join("\n")
    ))
}

pub async fn handle_terminal(message: Option<String>, force_new: bool) -> Result<()> {
    let state = load_terminal_state()?;
    let msg = message.ok_or(anyhow::anyhow!(
        "No message provided. Usage: goose \"your message\""
    ))?;

    let (session_id, is_new) = if force_new {
        let working_dir = std::env::current_dir()?;
        let session = SessionManager::create_session(
            working_dir,
            "Terminal Session".to_string(),
            SessionType::User,
        )
        .await?;
        (session.id, true)
    } else if let Some(state) = &state {
        if SessionManager::get_session(&state.session_id, false)
            .await
            .is_ok()
        {
            (state.session_id.clone(), false)
        } else {
            let working_dir = std::env::current_dir()?;
            let session = SessionManager::create_session(
                working_dir,
                "Terminal Session".to_string(),
                SessionType::User,
            )
            .await?;
            (session.id, true)
        }
    } else {
        let working_dir = std::env::current_dir()?;
        let session = SessionManager::create_session(
            working_dir,
            "Terminal Session".to_string(),
            SessionType::User,
        )
        .await?;
        (session.id, true)
    };

    let new_state = TerminalState {
        session_id: session_id.clone(),
    };

    save_terminal_state(&new_state)?;

    // Add recent shell commands to the message if resuming
    let enhanced_msg = if !is_new {
        let recent_commands = get_recent_shell_commands().unwrap_or_default();
        format!("{}{}", msg, recent_commands)
    } else {
        msg
    };

    let mut session = build_session(SessionBuilderConfig {
        session_id: Some(session_id),
        resume: !is_new,
        no_session: false,
        extensions: Vec::new(),
        remote_extensions: Vec::new(),
        streamable_http_extensions: Vec::new(),
        builtins: Vec::new(),
        extensions_override: None,
        additional_system_prompt: None,
        settings: None,
        provider: None,
        model: None,
        debug: false,
        max_tool_repetitions: None,
        max_turns: None,
        scheduled_job_id: None,
        interactive: false,
        quiet: true,
        sub_recipes: None,
        final_output_response: None,
        retry_config: None,
        output_format: "text".to_string(),
        skip_working_dir_check: true,
    })
    .await;

    session.headless(enhanced_msg).await?;

    Ok(())
}
