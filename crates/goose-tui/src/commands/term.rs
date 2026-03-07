use anyhow::{anyhow, Result};
use chrono;
use goose::conversation::message::{Message, MessageContent, MessageMetadata};
use goose::session::{SessionManager, SessionType};
use rmcp::model::Role;

use clap::ValueEnum;

#[derive(ValueEnum, Clone, Debug)]
pub enum Shell {
    Bash,
    Zsh,
    Fish,
    #[value(alias = "pwsh")]
    Powershell,
}

struct ShellConfig {
    script_template: &'static str,
    command_not_found: Option<&'static str>,
}

impl Shell {
    fn config(&self) -> &'static ShellConfig {
        match self {
            Shell::Bash => &BASH_CONFIG,
            Shell::Zsh => &ZSH_CONFIG,
            Shell::Fish => &FISH_CONFIG,
            Shell::Powershell => &POWERSHELL_CONFIG,
        }
    }
}

static BASH_CONFIG: ShellConfig = ShellConfig {
    script_template: r#"export AGENT_SESSION_ID="{session_id}"
alias @goose='{goose_bin} term run'
alias @g='{goose_bin} term run'

goose_preexec() {
    [[ "$1" =~ ^goose\ term ]] && return
    [[ "$1" =~ ^(@goose|@g)($|[[:space:]]) ]] && return
    ('{goose_bin}' term log "$1" &) 2>/dev/null
}

if [[ -z "$goose_preexec_installed" ]]; then
    goose_preexec_installed=1
    trap 'goose_preexec "$BASH_COMMAND"' DEBUG
fi{command_not_found_handler}"#,
    command_not_found: Some(
        r#"

command_not_found_handle() {
    echo "🪿 Command '$1' not found. Asking goose..."
    '{goose_bin}' term run "$@"
    return 0
}"#,
    ),
};

static ZSH_CONFIG: ShellConfig = ShellConfig {
    script_template: r#"export AGENT_SESSION_ID="{session_id}"
alias @goose='{goose_bin} term run'
alias @g='{goose_bin} term run'

goose_preexec() {
    [[ "$1" =~ ^goose\ term ]] && return
    [[ "$1" =~ ^(@goose|@g)($|[[:space:]]) ]] && return
    ('{goose_bin}' term log "$1" &) 2>/dev/null
}

autoload -Uz add-zsh-hook
add-zsh-hook preexec goose_preexec{command_not_found_handler}"#,
    command_not_found: Some(
        r#"

command_not_found_handler() {
    echo "🪿 Command '$1' not found. Asking goose..."
    '{goose_bin}' term run "$@"
    return 0
}"#,
    ),
};

static FISH_CONFIG: ShellConfig = ShellConfig {
    script_template: r#"set -gx AGENT_SESSION_ID "{session_id}"
function @goose; {goose_bin} term run $argv; end
function @g; {goose_bin} term run $argv; end

function goose_preexec --on-event fish_preexec
    string match -q -r '^goose term' -- $argv[1]; and return
    string match -q -r '^(@goose|@g)($|\s)' -- $argv[1]; and return
    {goose_bin} term log "$argv[1]" 2>/dev/null &
end"#,
    command_not_found: None,
};

static POWERSHELL_CONFIG: ShellConfig = ShellConfig {
    script_template: r#"$env:AGENT_SESSION_ID = "{session_id}"
function @goose {{ & '{goose_bin}' term run @args }}
function @g {{ & '{goose_bin}' term run @args }}

Set-PSReadLineKeyHandler -Chord Enter -ScriptBlock {{
    $line = $null
    [Microsoft.PowerShell.PSConsoleReadLine]::GetBufferState([ref]$line, [ref]$null)
    if ($line -notmatch '^goose term' -and $line -notmatch '^(@goose|@g)($|\s)') {{
        Start-Job -ScriptBlock {{ & '{goose_bin}' term log $using:line }} | Out-Null
    }}
    [Microsoft.PowerShell.PSConsoleReadLine]::AcceptLine()
}}"#,
    command_not_found: None,
};

pub async fn handle_term_init(
    shell: Shell,
    name: Option<String>,
    with_command_not_found: bool,
) -> Result<()> {
    let config = shell.config();
    let session_manager = SessionManager::instance();

    let working_dir = std::env::current_dir()?;
    let named_session = if let Some(ref name) = name {
        let sessions = session_manager
            .list_sessions_by_types(&[SessionType::Terminal])
            .await?;
        sessions.into_iter().find(|s| s.name == *name)
    } else {
        None
    };

    let session = match named_session {
        Some(s) => s,
        None => {
            let session = session_manager
                .create_session(
                    working_dir,
                    "Goose Term Session".to_string(),
                    SessionType::Terminal,
                )
                .await?;

            if let Some(name) = name {
                session_manager
                    .update(&session.id)
                    .user_provided_name(name)
                    .apply()
                    .await?;
            }

            session
        }
    };

    let goose_bin = std::env::current_exe()
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| "goose".to_string());

    let command_not_found_handler = if with_command_not_found {
        config
            .command_not_found
            .map(|s| s.replace("{goose_bin}", &goose_bin))
            .unwrap_or_default()
    } else {
        String::new()
    };

    let script = config
        .script_template
        .replace("{session_id}", &session.id)
        .replace("{goose_bin}", &goose_bin)
        .replace("{command_not_found_handler}", &command_not_found_handler);

    println!("{}", script);
    Ok(())
}

pub async fn handle_term_log(command: String) -> Result<()> {
    let session_id = std::env::var("AGENT_SESSION_ID").map_err(|_| {
        anyhow!("AGENT_SESSION_ID not set. Run 'eval \"$(goose term init <shell>)\"' first.")
    })?;

    let message = Message::new(
        Role::User,
        chrono::Utc::now().timestamp_millis(),
        vec![MessageContent::text(command)],
    )
    .with_metadata(MessageMetadata::user_only())
    .with_generated_id();

    let session_manager = SessionManager::instance();
    session_manager.add_message(&session_id, &message).await?;

    Ok(())
}

pub async fn handle_term_run(_prompt: Vec<String>) -> Result<()> {
    anyhow::bail!(
        "term run not yet supported in unified binary - use goose-tui interactive mode instead"
    )
}

pub async fn handle_term_info() -> Result<()> {
    let session_id = match std::env::var("AGENT_SESSION_ID") {
        Ok(id) => id,
        Err(_) => return Ok(()),
    };

    let session_manager = SessionManager::instance();
    let session = session_manager.get_session(&session_id, false).await.ok();
    let total_tokens = session.as_ref().and_then(|s| s.total_tokens).unwrap_or(0) as usize;

    let config = goose::config::Config::global();
    let model_name = config
        .get_goose_model()
        .ok()
        .map(|name| {
            let short = name.rsplit('/').next().unwrap_or(&name);
            if let Some(stripped) = short.strip_prefix("goose-") {
                stripped.to_string()
            } else {
                short.to_string()
            }
        })
        .unwrap_or_else(|| "?".to_string());

    let context_limit = config
        .get_goose_model()
        .ok()
        .and_then(|model_name| {
            config.get_goose_provider().ok().and_then(|provider_name| {
                goose::model::ModelConfig::new(&model_name)
                    .ok()
                    .map(|c| c.with_canonical_limits(&provider_name))
            })
        })
        .map(|mc| mc.context_limit())
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
