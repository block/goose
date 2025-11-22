use anyhow::{anyhow, Result};
use goose::session::session_manager::SessionType;
use goose::session::SessionManager;
use uuid::Uuid;

use crate::session::{build_session, SessionBuilderConfig};

const TERMINAL_SESSION_PREFIX: &str = "term:";

/// Handle `goose term init <shell>` - print shell initialization script
pub fn handle_term_init(shell: &str) -> Result<()> {
    let terminal_id = Uuid::new_v4().to_string();

    // Get the path to the current goose binary
    let goose_bin = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "goose".to_string());

    let script = match shell.to_lowercase().as_str() {
        "bash" => {
            format!(
                r#"export GOOSE_TERMINAL_ID="{terminal_id}"
alias gt='{goose_bin} term run'

# Log commands to goose (runs silently in background)
goose_preexec() {{
    [[ "$1" =~ ^goose\ term ]] && return
    [[ "$1" =~ ^gt\  ]] && return
    ("{goose_bin}" term log "$1" &) 2>/dev/null
}}

# Install preexec hook for bash
if [[ -z "$goose_preexec_installed" ]]; then
    goose_preexec_installed=1
    trap 'goose_preexec "$BASH_COMMAND"' DEBUG
fi"#
            )
        }
        "zsh" => {
            format!(
                r#"export GOOSE_TERMINAL_ID="{terminal_id}"
alias gt='{goose_bin} term run'

# Log commands to goose (runs silently in background)
goose_preexec() {{
    [[ "$1" =~ ^goose\ term ]] && return
    [[ "$1" =~ ^gt\  ]] && return
    ("{goose_bin}" term log "$1" &) 2>/dev/null
}}

# Install preexec hook for zsh
autoload -Uz add-zsh-hook
add-zsh-hook preexec goose_preexec"#
            )
        }
        "fish" => {
            format!(
                r#"set -gx GOOSE_TERMINAL_ID "{terminal_id}"
alias gt='{goose_bin} term run'

# Log commands to goose
function goose_preexec --on-event fish_preexec
    string match -q -r '^goose term' -- $argv[1]; and return
    string match -q -r '^gt ' -- $argv[1]; and return
    {goose_bin} term log $argv[1] 2>/dev/null &
end"#
            )
        }
        "powershell" | "pwsh" => {
            format!(
                r#"$env:GOOSE_TERMINAL_ID = "{terminal_id}"
Set-Alias -Name gt -Value {{ {goose_bin} term run $args }}

# Log commands to goose
Set-PSReadLineKeyHandler -Chord Enter -ScriptBlock {{
    $line = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$null)
    if ($line -notmatch '^goose term' -and $line -notmatch '^gt ') {{
        Start-Job -ScriptBlock {{ {goose_bin} term log $using:line }} | Out-Null
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

    // Create session if it doesn't exist (so we can log commands before first run)
    if SessionManager::get_session(&session_name, false)
        .await
        .is_err()
    {
        let session = SessionManager::create_session_with_id(
            session_name.clone(),
            working_dir.clone(),
            session_name.clone(),
            SessionType::User,
        )
        .await?;

        SessionManager::update_session(&session.id)
            .user_provided_name(session_name.clone())
            .apply()
            .await?;
    }

    SessionManager::add_shell_command(&session_name, &command, &working_dir).await?;

    Ok(())
}

/// Handle `goose term run <prompt>` - run a prompt in the terminal session
pub async fn handle_term_run(prompt: String) -> Result<()> {
    let terminal_id = std::env::var("GOOSE_TERMINAL_ID").map_err(|_| {
        anyhow!(
            "GOOSE_TERMINAL_ID not set.\n\n\
             Add to your shell config (~/.zshrc or ~/.bashrc):\n    \
             eval \"$(goose term init zsh)\"\n\n\
             Then restart your terminal or run: source ~/.zshrc"
        )
    })?;

    let session_name = format!("{}{}", TERMINAL_SESSION_PREFIX, terminal_id);

    let session_id = match SessionManager::get_session(&session_name, false).await {
        Ok(_) => {
            SessionManager::update_session(&session_name)
                .working_dir(std::env::current_dir()?)
                .apply()
                .await?;
            session_name.clone()
        }
        Err(_) => {
            let session = SessionManager::create_session_with_id(
                session_name.clone(),
                std::env::current_dir()?,
                session_name.clone(),
                SessionType::User,
            )
            .await?;

            // Mark with user-provided name so session persists across restarts
            SessionManager::update_session(&session.id)
                .user_provided_name(session_name)
                .apply()
                .await?;

            session.id
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
