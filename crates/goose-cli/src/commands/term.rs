use anyhow::{anyhow, Result};
use goose::config::paths::Paths;
use goose::session::SessionManager;
use std::fs;
use std::io::Write;
use std::path::PathBuf;

use crate::session::{build_session, SessionBuilderConfig};

const TERMINAL_SESSION_PREFIX: &str = "term:";

async fn get_or_create_terminal_session(working_dir: PathBuf) -> Result<String> {
    let session_name = format!(
        "{}{}",
        TERMINAL_SESSION_PREFIX,
        working_dir.to_string_lossy()
    );

    // Find existing session by name
    let sessions = SessionManager::list_sessions().await?;
    if let Some(session) = sessions.iter().find(|s| s.name == session_name) {
        return Ok(session.id.clone());
    }

    // Create new session
    let session =
        SessionManager::create_session(working_dir, session_name.clone(), Default::default())
            .await?;

    SessionManager::update_session(&session.id)
        .user_provided_name(session_name)
        .apply()
        .await?;

    Ok(session.id)
}

/// Handle `goose term init <shell>` - print shell initialization script
pub async fn handle_term_init(shell: &str, with_command_not_found: bool) -> Result<()> {
    let working_dir = std::env::current_dir()?;
    let session_id = get_or_create_terminal_session(working_dir).await?;

    let goose_bin = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "goose".to_string());

    let command_not_found_handler = if with_command_not_found {
        match shell.to_lowercase().as_str() {
            "bash" => format!(
                r#"

# Command not found handler - sends unknown commands to goose
command_not_found_handle() {{
    echo "🪿 Command '$1' not found. Asking goose..."
    '{goose_bin}' term run "$@"
    return 0
}}"#
            ),
            "zsh" => format!(
                r#"

# Command not found handler - sends unknown commands to goose
command_not_found_handler() {{
    echo "🪿 Command '$1' not found. Asking goose..."
    '{goose_bin}' term run "$@"
    return 0
}}"#
            ),
            _ => String::new(),
        }
    } else {
        String::new()
    };

    let script = match shell.to_lowercase().as_str() {
        "bash" => {
            format!(
                r#"export GOOSE_SESSION_ID="{session_id}"
alias gt='{goose_bin} term run'

# Log commands to goose (runs silently in background)
goose_preexec() {{
    [[ "$1" =~ ^goose\ term ]] && return
    [[ "$1" =~ ^gt($|[[:space:]]) ]] && return
    ('{goose_bin}' term log "$1" &) 2>/dev/null
}}

# Install preexec hook for bash
if [[ -z "$goose_preexec_installed" ]]; then
    goose_preexec_installed=1
    trap 'goose_preexec "$BASH_COMMAND"' DEBUG
fi{command_not_found_handler}"#
            )
        }
        "zsh" => {
            format!(
                r#"export GOOSE_SESSION_ID="{session_id}"
alias gt='{goose_bin} term run'

# Log commands to goose (runs silently in background)
goose_preexec() {{
    [[ "$1" =~ ^goose\ term ]] && return
    [[ "$1" =~ ^gt($|[[:space:]]) ]] && return
    ('{goose_bin}' term log "$1" &) 2>/dev/null
}}

# Install preexec hook for zsh
autoload -Uz add-zsh-hook
add-zsh-hook preexec goose_preexec

# Add goose indicator to prompt
if [[ -z "$GOOSE_PROMPT_INSTALLED" ]]; then
    export GOOSE_PROMPT_INSTALLED=1
    PROMPT='%F{{cyan}}🪿%f '$PROMPT
fi{command_not_found_handler}"#
            )
        }
        "fish" => {
            format!(
                r#"set -gx GOOSE_SESSION_ID "{session_id}"
function gt; {goose_bin} term run $argv; end

# Log commands to goose
function goose_preexec --on-event fish_preexec
    string match -q -r '^goose term' -- $argv[1]; and return
    string match -q -r '^gt($|\s)' -- $argv[1]; and return
    {goose_bin} term log "$argv[1]" 2>/dev/null &
end"#
            )
        }
        "powershell" | "pwsh" => {
            format!(
                r#"$env:GOOSE_SESSION_ID = "{session_id}"
function gt {{ & '{goose_bin}' term run @args }}

# Log commands to goose
Set-PSReadLineKeyHandler -Chord Enter -ScriptBlock {{
    $line = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$null)
    if ($line -notmatch '^goose term' -and $line -notmatch '^gt($|\s)') {{
        Start-Job -ScriptBlock {{ & '{goose_bin}' term log $using:line }} | Out-Null
    }}
    [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
}}"#
            )
        }
        _ => {
            return Err(anyhow!(
                "Unsupported shell: {}. Supported shells: bash, zsh, fish, powershell",
                shell
            ));
        }
    };

    println!("{}", script);
    Ok(())
}

fn shell_history_path(session_id: &str) -> Result<PathBuf> {
    let history_dir = Paths::config_dir().join("shell-history");
    fs::create_dir_all(&history_dir)?;
    Ok(history_dir.join(format!("{}.txt", session_id)))
}

fn append_shell_command(session_id: &str, command: &str) -> Result<()> {
    let path = shell_history_path(session_id)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{}", command)?;
    Ok(())
}

fn read_and_clear_shell_history(session_id: &str) -> Result<Vec<String>> {
    let path = shell_history_path(session_id)?;

    if !path.exists() {
        return Ok(Vec::new());
    }

    let content = fs::read_to_string(&path)?;
    let commands: Vec<String> = content
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|s| s.to_string())
        .collect();

    fs::write(&path, "")?;

    Ok(commands)
}

pub async fn handle_term_log(command: String) -> Result<()> {
    let session_id = std::env::var("GOOSE_SESSION_ID").map_err(|_| {
        anyhow!("GOOSE_SESSION_ID not set. Run 'eval \"$(goose term init <shell>)\"' first.")
    })?;

    append_shell_command(&session_id, &command)?;

    Ok(())
}

pub async fn handle_term_run(prompt: Vec<String>) -> Result<()> {
    let prompt = prompt.join(" ");
    let session_id = std::env::var("GOOSE_SESSION_ID").map_err(|_| {
        anyhow!(
            "GOOSE_SESSION_ID not set.\n\n\
             Add to your shell config (~/.zshrc or ~/.bashrc):\n    \
             eval \"$(goose term init zsh)\"\n\n\
             Then restart your terminal or run: source ~/.zshrc"
        )
    })?;

    let working_dir = std::env::current_dir()?;

    SessionManager::update_session(&session_id)
        .working_dir(working_dir)
        .apply()
        .await?;

    let commands = read_and_clear_shell_history(&session_id)?;
    let prompt_with_context = if commands.is_empty() {
        prompt
    } else {
        format!(
            "<shell_history>\n{}\n</shell_history>\n\n{}",
            commands.join("\n"),
            prompt
        )
    };

    let config = SessionBuilderConfig {
        session_id: Some(session_id),
        resume: true,
        interactive: false,
        quiet: true,
        ..Default::default()
    };

    let mut session = build_session(config).await;
    session.headless(prompt_with_context).await?;

    Ok(())
}

/// Handle `goose term info` - print compact session info for prompt integration
pub async fn handle_term_info() -> Result<()> {
    let session_id = match std::env::var("GOOSE_SESSION_ID") {
        Ok(id) => id,
        Err(_) => return Ok(()),
    };

    let session = SessionManager::get_session(&session_id, false).await.ok();
    let total_tokens = session.as_ref().and_then(|s| s.total_tokens).unwrap_or(0) as usize;

    let model_name = session
        .as_ref()
        .and_then(|s| s.model_config.as_ref().map(|mc| mc.model_name.clone()))
        .map(|name| {
            let short = name.rsplit('/').next().unwrap_or(&name);
            if let Some(stripped) = short.strip_prefix("goose-") {
                stripped.to_string()
            } else {
                short.to_string()
            }
        })
        .unwrap_or_else(|| "?".to_string());

    let context_limit = session
        .as_ref()
        .and_then(|s| s.model_config.as_ref().map(|mc| mc.context_limit()))
        .unwrap_or(128_000);

    let percentage = if context_limit > 0 {
        ((total_tokens as f64 / context_limit as f64) * 100.0).round() as usize
    } else {
        0
    };

    let filled = (percentage / 20).min(5);
    let empty = 5 - filled;
    let dots = format!("{}{}", "●".repeat(filled), "○".repeat(empty));

    println!("{} {}", dots, model_name);

    Ok(())
}
