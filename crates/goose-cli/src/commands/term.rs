use anyhow::{anyhow, Result};
use goose::session::session_manager::SessionType;
use goose::session::SessionManager;
use uuid::Uuid;

use crate::session::{build_session, SessionBuilderConfig};

const TERMINAL_SESSION_PREFIX: &str = "term:";

/// Ensure a terminal session exists, creating it if necessary
async fn ensure_terminal_session(
    session_name: String,
    working_dir: std::path::PathBuf,
) -> Result<()> {
    if SessionManager::get_session(&session_name, false)
        .await
        .is_err()
    {
        let session = SessionManager::create_session_with_id(
            session_name.clone(),
            working_dir,
            session_name.clone(),
            SessionType::Hidden,
        )
        .await?;

        SessionManager::update_session(&session.id)
            .user_provided_name(session_name)
            .apply()
            .await?;
    }
    Ok(())
}

/// Handle `goose term init <shell>` - print shell initialization script
pub fn handle_term_init(shell: &str, with_command_not_found: bool) -> Result<()> {
    let terminal_id = Uuid::new_v4().to_string();

    // Get the path to the current goose binary
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
                r#"export GOOSE_TERMINAL_ID="{terminal_id}"
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
                r#"export GOOSE_TERMINAL_ID="{terminal_id}"
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
                r#"set -gx GOOSE_TERMINAL_ID "{terminal_id}"
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
                r#"$env:GOOSE_TERMINAL_ID = "{terminal_id}"
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

/// Handle `goose term log <command>` - log a shell command to the database
pub async fn handle_term_log(command: String) -> Result<()> {
    let terminal_id = std::env::var("GOOSE_TERMINAL_ID")
        .map_err(|_| anyhow!("GOOSE_TERMINAL_ID not set. Run 'goose term init <shell>' first."))?;

    let session_name = format!("{}{}", TERMINAL_SESSION_PREFIX, terminal_id);
    let working_dir = std::env::current_dir()?;

    ensure_terminal_session(session_name.clone(), working_dir.clone()).await?;
    SessionManager::add_shell_command(&session_name, &command, &working_dir).await?;

    Ok(())
}

/// Handle `goose term run <prompt>` - run a prompt in the terminal session
pub async fn handle_term_run(prompt: Vec<String>) -> Result<()> {
    let prompt = prompt.join(" ");
    let terminal_id = std::env::var("GOOSE_TERMINAL_ID").map_err(|_| {
        anyhow!(
            "GOOSE_TERMINAL_ID not set.\n\n\
             Add to your shell config (~/.zshrc or ~/.bashrc):\n    \
             eval \"$(goose term init zsh)\"\n\n\
             Then restart your terminal or run: source ~/.zshrc"
        )
    })?;

    let session_name = format!("{}{}", TERMINAL_SESSION_PREFIX, terminal_id);
    let working_dir = std::env::current_dir()?;

    let session_id = match SessionManager::get_session(&session_name, false).await {
        Ok(_) => {
            SessionManager::update_session(&session_name)
                .working_dir(working_dir)
                .apply()
                .await?;
            session_name.clone()
        }
        Err(_) => {
            ensure_terminal_session(session_name.clone(), working_dir).await?;
            session_name.clone()
        }
    };

    let commands = SessionManager::get_shell_commands_since_last_message(&session_id).await?;
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
    use goose::config::Config;

    let terminal_id = match std::env::var("GOOSE_TERMINAL_ID") {
        Ok(id) => id,
        Err(_) => return Ok(()), // Silent exit if no terminal ID
    };

    let session_name = format!("{}{}", TERMINAL_SESSION_PREFIX, terminal_id);

    // Get tokens from session or 0 if none started yet in this terminal
    let session = SessionManager::get_session(&session_name, false).await.ok();
    let total_tokens = session.as_ref().and_then(|s| s.total_tokens).unwrap_or(0) as usize;

    let model_name = Config::global()
        .get_goose_model()
        .ok()
        .or_else(|| {
            session
                .as_ref()
                .and_then(|s| s.model_config.as_ref().map(|mc| mc.model_name.clone()))
        })
        .map(|name| {
            // Extract short name: after last / or after last - if it starts with "goose-"
            let short = name.rsplit('/').next().unwrap_or(&name);
            if let Some(stripped) = short.strip_prefix("goose-") {
                stripped.to_string()
            } else {
                short.to_string()
            }
        })
        .unwrap_or_else(|| "?".to_string());

    // Get context limit for the model
    let context_limit = session
        .as_ref()
        .and_then(|s| s.model_config.as_ref().map(|mc| mc.context_limit()))
        .unwrap_or(128_000);

    // Calculate percentage and create dot visualization
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
