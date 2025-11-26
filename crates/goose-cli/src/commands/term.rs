use anyhow::{anyhow, Result};
use goose::session::SessionManager;
use goose::session::SessionType;
use goose::conversation::message::{Message, MessageContent, MessageMetadata};
use rmcp::model::Role;
use chrono;

use crate::session::{build_session, SessionBuilderConfig};


/// Handle `goose term init <shell>` - print shell initialization script
pub async fn handle_term_init(shell: &str, with_command_not_found: bool) -> Result<()> {
    let working_dir = std::env::current_dir()?;
    let session = SessionManager::create_session(working_dir,
                                                    "Goose Term Session".to_string(),
                                                    SessionType::Terminal).await?;
    let session_id = session.id;

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
alias @goose='{goose_bin} term run'
alias @g='{goose_bin} term run'

# Log commands to goose (runs silently in background)
goose_preexec() {{
    [[ "$1" =~ ^goose\ term ]] && return
    [[ "$1" =~ ^(gt|@goose|@g)($|[[:space:]]) ]] && return
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
alias @goose='{goose_bin} term run'
alias @g='{goose_bin} term run'

# Log commands to goose (runs silently in background)
goose_preexec() {{
    [[ "$1" =~ ^goose\ term ]] && return
    [[ "$1" =~ ^(gt|@goose|@g)($|[[:space:]]) ]] && return
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
function @goose; {goose_bin} term run $argv; end
function @g; {goose_bin} term run $argv; end

# Log commands to goose
function goose_preexec --on-event fish_preexec
    string match -q -r '^goose term' -- $argv[1]; and return
    string match -q -r '^(gt|@goose|@g)($|\s)' -- $argv[1]; and return
    {goose_bin} term log "$argv[1]" 2>/dev/null &
end"#
            )
        }
        "powershell" | "pwsh" => {
            format!(
                r#"$env:GOOSE_SESSION_ID = "{session_id}"
function gt {{ & '{goose_bin}' term run @args }}
function @goose {{ & '{goose_bin}' term run @args }}
function @g {{ & '{goose_bin}' term run @args }}

# Log commands to goose
Set-PSReadLineKeyHandler -Chord Enter -ScriptBlock {{
    $line = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$null)
    if ($line -notmatch '^goose term' -and $line -notmatch '^(gt|@goose|@g)($|\s)') {{
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



pub async fn handle_term_log(command: String) -> Result<()> {
    let session_id = std::env::var("GOOSE_SESSION_ID").map_err(|_| {
        anyhow!("GOOSE_SESSION_ID not set. Run 'eval \"$(goose term init <shell>)\"' first.")
    })?;

    let message = Message::new(
        Role::User,
        chrono::Utc::now().timestamp_millis(),
        vec![MessageContent::text(command)],
    )
    .with_metadata(MessageMetadata::user_only());

    SessionManager::add_message(&session_id, &message).await?;

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

    let session = SessionManager::get_session(&session_id, true).await?;
    let user_messages_after_last_assistant: Vec<&Message> = if let Some(conv) = &session.conversation {
        conv.messages()
            .iter()
            .rev()
            .take_while(|m| m.role != Role::Assistant)
            .collect()
    } else {
        Vec::new()
    };

    if let Some(oldest_user) = user_messages_after_last_assistant.last() {
        SessionManager::truncate_conversation(&session_id, oldest_user.created).await?;
    }

    let prompt_with_context = if user_messages_after_last_assistant.is_empty() {
        prompt
    } else {
        let history = user_messages_after_last_assistant
            .iter()
            .rev() // back to chronological order
            .map(|m| m.as_concat_text())
            .collect::<Vec<_>>()
            .join("\n");

        format!(
            "<shell_history>\n{}\n</shell_history>\n\n{}",
            history,
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
