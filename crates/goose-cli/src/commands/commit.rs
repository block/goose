use anyhow::Result;
use indoc::indoc;

use crate::session::{build_session, SessionBuilderConfig};

/// Configuration for the commit command
pub struct CommitConfig {
    pub staged_only: bool,
    pub debug: bool,
    pub max_tool_repetitions: Option<u32>,
    pub max_turns: Option<u32>,
    pub extensions: Vec<String>,
    pub remote_extensions: Vec<String>,
    pub streamable_http_extensions: Vec<String>,
    pub builtins: Vec<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
}

/// Handle the commit command - review changes and create an AI-generated commit
pub async fn handle_commit(config: CommitConfig) -> Result<()> {
    let CommitConfig {
        staged_only,
        debug,
        max_tool_repetitions,
        max_turns,
        extensions,
        remote_extensions,
        streamable_http_extensions,
        builtins,
        provider,
        model,
    } = config;
    // Build the commit analysis prompt
    let commit_prompt = if staged_only {
        indoc! {r#"
            Review this git repository and create a commit for the staged changes.

            Please follow these steps:
            1. Use `git status` to see what files are staged for commit
            2. Use `git diff --staged` to review the staged changes in detail
            3. Use `git log --oneline -10` to review the last 10 commits for context
            4. If any of those recent commits seem relevant to the current changes, use `git show <commit-hash>` to review them in detail
            5. Based on your analysis, create a commit using `git commit` with a well-written conventional commit message that:
               - Follows the conventional commits format (e.g., "feat:", "fix:", "docs:", etc.)
               - Has a concise subject line (50 chars or less)
               - Includes a detailed body if the changes are complex
               - References any related issues if applicable

            After the commit is created, you can continue to interact with me for any follow-up actions.
        "#}
    } else {
        indoc! {r#"
            Review this git repository and create a commit for all uncommitted changes.

            Please follow these steps:
            1. Use `git status` to see what files have changed (both staged and unstaged)
            2. Use `git diff` to review unstaged changes
            3. Use `git diff --staged` to review staged changes (if any)
            4. Use `git log --oneline -10` to review the last 10 commits for context
            5. If any of those recent commits seem relevant to the current changes, use `git show <commit-hash>` to review them in detail
            6. Use `git add -A` to stage all changes
            7. Based on your analysis, create a commit using `git commit` with a well-written conventional commit message that:
               - Follows the conventional commits format (e.g., "feat:", "fix:", "docs:", etc.)
               - Has a concise subject line (50 chars or less)
               - Includes a detailed body if the changes are complex
               - References any related issues if applicable

            After the commit is created, you can continue to interact with me for any follow-up actions.
        "#}
    };

    tracing::info!(
        counter.goose.commit_sessions = 1,
        staged_only,
        "Commit session started"
    );

    // Build the session with interactive mode enabled
    let mut session = build_session(SessionBuilderConfig {
        session_id: None,
        resume: false,
        no_session: false,
        extensions,
        remote_extensions,
        streamable_http_extensions,
        builtins,
        extensions_override: None,
        additional_system_prompt: None,
        settings: None,
        provider,
        model,
        debug,
        max_tool_repetitions,
        max_turns,
        scheduled_job_id: None,
        interactive: true, // Always interactive for commit command
        quiet: false,
        sub_recipes: None,
        final_output_response: None,
        retry_config: None,
    })
    .await;

    // Start interactive session with the commit prompt as initial input
    session.interactive(Some(commit_prompt.to_string())).await?;

    Ok(())
}
