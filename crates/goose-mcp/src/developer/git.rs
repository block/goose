use crate::developer::DeveloperRouter;
use anyhow::Result;
use indoc::formatdoc;
use mcp_core::{handler::ToolError, Content};
use serde_json::Value;
use std::path::Path;
use std::process::Command;

// ===============================
// Git Core Functionality
// ===============================

/// Checks if the current directory is a git repository
pub fn is_git_repo(path: &Path) -> bool {
    Command::new("git")
        .args(["rev-parse", "--is-inside-work-tree"])
        .current_dir(path)
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

/// Checks if there are unstaged changes in the repository
pub fn has_unstaged_changes(path: &Path) -> bool {
    Command::new("git")
        .args(["diff", "--quiet"])
        .current_dir(path)
        .status()
        .map(|status| !status.success())
        .unwrap_or(false)
}

/// Gets the current branch name
pub fn get_current_branch(path: &Path) -> Result<String> {
    let output = Command::new("git")
        .args(["rev-parse", "--abbrev-ref", "HEAD"])
        .current_dir(path)
        .output()?;

    if output.status.success() {
        let branch = String::from_utf8(output.stdout)?;
        Ok(branch.trim().to_string())
    } else {
        anyhow::bail!("Failed to get current branch")
    }
}

/// Creates a new git repository
pub fn init_repo(path: &Path) -> Result<()> {
    let status = Command::new("git")
        .args(["init"])
        .current_dir(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to initialize git repository")
    }
}

/// Creates a new branch and switches to it
pub fn create_branch(path: &Path, branch_name: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["checkout", "-b", branch_name])
        .current_dir(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to create branch {}", branch_name)
    }
}

/// Switches to an existing branch
pub fn switch_branch(path: &Path, branch_name: &str) -> Result<()> {
    let status = Command::new("git")
        .args(["checkout", branch_name])
        .current_dir(path)
        .status()?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to switch to branch {}", branch_name)
    }
}

/// Commits all changes with a message
pub fn commit_changes(path: &Path, message: &str) -> Result<()> {
    // Add all changes
    let add_status = Command::new("git")
        .args(["add", "."])
        .current_dir(path)
        .status()?;

    if !add_status.success() {
        anyhow::bail!("Failed to stage changes");
    }

    // Commit with message
    let commit_status = Command::new("git")
        .args(["commit", "-m", message])
        .current_dir(path)
        .status()?;

    if commit_status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to commit changes")
    }
}

/// Lists all branches in the repository
pub fn list_branches(path: &Path) -> Result<Vec<String>> {
    let output = Command::new("git")
        .args(["branch"])
        .current_dir(path)
        .output()?;

    if output.status.success() {
        let branches_output = String::from_utf8(output.stdout)?;
        let branches = branches_output
            .lines()
            .map(|line| line.trim_start_matches('*').trim().to_string())
            .collect();
        Ok(branches)
    } else {
        anyhow::bail!("Failed to list branches")
    }
}

/// Resets to the last commit
pub fn reset_to_last_commit(path: &Path, hard: bool) -> Result<()> {
    let args = if hard {
        vec!["reset", "--hard", "HEAD"]
    } else {
        vec!["reset", "HEAD"]
    };

    let status = Command::new("git").args(args).current_dir(path).status()?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to reset to last commit")
    }
}

/// Gets the list of commits on the current branch
pub fn get_commits(path: &Path, count: usize) -> Result<Vec<(String, String)>> {
    let format = "%h %s";
    let count_arg = format!("-{}", count);

    let output = Command::new("git")
        .args(["log", &count_arg, &format!("--pretty=format:{}", format)])
        .current_dir(path)
        .output()?;

    if output.status.success() {
        let commits_output = String::from_utf8(output.stdout)?;
        let commits = commits_output
            .lines()
            .map(|line| {
                let parts: Vec<&str> = line.splitn(2, ' ').collect();
                if parts.len() == 2 {
                    (parts[0].to_string(), parts[1].to_string())
                } else {
                    (parts[0].to_string(), String::new())
                }
            })
            .collect();
        Ok(commits)
    } else {
        anyhow::bail!("Failed to get commit history")
    }
}

/// Resets to a specific commit
pub fn reset_to_commit(path: &Path, commit_hash: &str, hard: bool) -> Result<()> {
    let args = if hard {
        vec!["reset", "--hard", commit_hash]
    } else {
        vec!["reset", commit_hash]
    };

    let status = Command::new("git").args(args).current_dir(path).status()?;

    if status.success() {
        Ok(())
    } else {
        anyhow::bail!("Failed to reset to commit {}", commit_hash)
    }
}

// ===============================
// Git Tool Implementations
// ===============================

pub async fn git_status(
    _router: &DeveloperRouter,
    _params: Value,
) -> Result<Vec<Content>, ToolError> {
    let cwd = std::env::current_dir().expect("should have a current working dir");

    if !is_git_repo(&cwd) {
        return Ok(vec![Content::text(formatdoc! {r#"
            The current directory is not a git repository.
            
            Would you like to initialize a git repository here? If so, use the `git_branch` tool with the action "create" to create a new branch.
        "#})]);
    }

    let has_unstaged = has_unstaged_changes(&cwd);
    let current_branch = get_current_branch(&cwd)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to get current branch: {}", e)))?;

    let status_message = if has_unstaged {
        formatdoc! {r#"
            Git Status:
            - Current branch: {}
            - There are unstaged changes in the repository
            
            It's recommended to create a checkpoint using the `git_checkpoint` tool before making further changes.
        "#, current_branch}
    } else {
        formatdoc! {r#"
            Git Status:
            - Current branch: {}
            - Working directory is clean (no unstaged changes)
            
            You can safely create a new branch for your task using the `git_branch` tool.
        "#, current_branch}
    };

    Ok(vec![Content::text(status_message)])
}

pub async fn git_branch(
    _router: &DeveloperRouter,
    params: Value,
) -> Result<Vec<Content>, ToolError> {
    let cwd = std::env::current_dir().expect("should have a current working dir");

    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidParameters("Missing 'action' parameter".into()))?;

    match action {
        "list" => {
            if !is_git_repo(&cwd) {
                return Ok(vec![Content::text("The current directory is not a git repository. Use the 'create' action to initialize a repository and create a branch.")]);
            }

            let branches = list_branches(&cwd).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to list branches: {}", e))
            })?;

            let current_branch = get_current_branch(&cwd).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to get current branch: {}", e))
            })?;

            let branches_list = branches
                .iter()
                .map(|branch| {
                    if branch == &current_branch {
                        format!("* {} (current)", branch)
                    } else {
                        format!("  {}", branch)
                    }
                })
                .collect::<Vec<String>>()
                .join("\n");

            Ok(vec![Content::text(formatdoc! {r#"
                Available branches:
                {}
            "#, branches_list})])
        }
        "create" => {
            let branch_name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters("Missing 'name' parameter".into()))?;

            if !is_git_repo(&cwd) {
                // Initialize a new repository
                init_repo(&cwd).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to initialize git repository: {}", e))
                })?;

                // Create an initial commit
                std::fs::write(cwd.join(".gitignore"), "").map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to create .gitignore: {}", e))
                })?;

                commit_changes(&cwd, "Initial commit").map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to create initial commit: {}", e))
                })?;
            }

            // Check for unstaged changes
            if has_unstaged_changes(&cwd) {
                return Ok(vec![Content::text(formatdoc! {r#"
                    There are unstaged changes in the repository. 
                    
                    Please commit these changes using the `git_checkpoint` tool before creating a new branch.
                "#})]);
            }

            // Create the new branch
            create_branch(&cwd, branch_name).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to create branch: {}", e))
            })?;

            Ok(vec![Content::text(formatdoc! {r#"
                Successfully created and switched to branch '{}'.
                
                You can now make changes and create checkpoints using the `git_checkpoint` tool.
            "#, branch_name})])
        }
        "switch" => {
            let branch_name = params
                .get("name")
                .and_then(|v| v.as_str())
                .ok_or_else(|| ToolError::InvalidParameters("Missing 'name' parameter".into()))?;

            if !is_git_repo(&cwd) {
                return Ok(vec![Content::text("The current directory is not a git repository. Use the 'create' action to initialize a repository and create a branch.")]);
            }

            // Check for unstaged changes
            if has_unstaged_changes(&cwd) {
                return Ok(vec![Content::text(formatdoc! {r#"
                    There are unstaged changes in the repository. 
                    
                    Please commit these changes using the `git_checkpoint` tool before switching branches.
                "#})]);
            }

            // Switch to the branch
            switch_branch(&cwd, branch_name).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to switch to branch: {}", e))
            })?;

            Ok(vec![Content::text(formatdoc! {r#"
                Successfully switched to branch '{}'.
            "#, branch_name})])
        }
        _ => Err(ToolError::InvalidParameters(format!(
            "Unknown action: {}",
            action
        ))),
    }
}

pub async fn git_checkpoint(
    _router: &DeveloperRouter,
    params: Value,
) -> Result<Vec<Content>, ToolError> {
    let cwd = std::env::current_dir().expect("should have a current working dir");

    if !is_git_repo(&cwd) {
        return Ok(vec![Content::text("The current directory is not a git repository. Use the `git_branch` tool with the 'create' action to initialize a repository and create a branch.")]);
    }

    let message = params
        .get("message")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidParameters("Missing 'message' parameter".into()))?;

    // Check if there are any changes to commit
    if !has_unstaged_changes(&cwd) {
        return Ok(vec![Content::text(
            "There are no changes to commit. The working directory is clean.",
        )]);
    }

    // Commit the changes
    commit_changes(&cwd, message)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to commit changes: {}", e)))?;

    let current_branch = get_current_branch(&cwd)
        .map_err(|e| ToolError::ExecutionError(format!("Failed to get current branch: {}", e)))?;

    Ok(vec![Content::text(formatdoc! {r#"
        Successfully created checkpoint on branch '{}' with message:
        "{}"
        
        The working directory is now clean and ready for more changes.
    "#, current_branch, message})])
}

pub async fn git_rollback(
    _router: &DeveloperRouter,
    params: Value,
) -> Result<Vec<Content>, ToolError> {
    let cwd = std::env::current_dir().expect("should have a current working dir");

    if !is_git_repo(&cwd) {
        return Ok(vec![Content::text("The current directory is not a git repository. Use the `git_branch` tool with the 'create' action to initialize a repository and create a branch.")]);
    }

    let action = params
        .get("action")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ToolError::InvalidParameters("Missing 'action' parameter".into()))?;

    match action {
        "show_commits" => {
            // Show the last 10 commits
            let commits = get_commits(&cwd, 10).map_err(|e| {
                ToolError::ExecutionError(format!("Failed to get commit history: {}", e))
            })?;

            if commits.is_empty() {
                return Ok(vec![Content::text("No commits found in the repository.")]);
            }

            let commits_list = commits
                .iter()
                .map(|(hash, message)| format!("{}: {}", hash, message))
                .collect::<Vec<String>>()
                .join("\n");

            Ok(vec![Content::text(formatdoc! {r#"
                Recent commits (newest first):
                {}
                
                You can use these commit hashes with the `reset_soft` or `reset_hard` actions to roll back to a specific commit.
            "#, commits_list})])
        }
        "reset_soft" => {
            let commit = params
                .get("commit")
                .and_then(|v| v.as_str())
                .unwrap_or("HEAD");

            // Soft reset (keep changes)
            if commit == "HEAD" {
                reset_to_last_commit(&cwd, false).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to reset to last commit: {}", e))
                })?;
            } else {
                reset_to_commit(&cwd, commit, false).map_err(|e| {
                    ToolError::ExecutionError(format!(
                        "Failed to reset to commit {}: {}",
                        commit, e
                    ))
                })?;
            }

            Ok(vec![Content::text(formatdoc! {r#"
                Successfully performed a soft reset to {}.
                
                Your changes have been unstaged but are still present in the working directory.
                You can make further modifications and then create a new checkpoint.
            "#, if commit == "HEAD" { "the last commit".to_string() } else { format!("commit {}", commit) }})])
        }
        "reset_hard" => {
            let commit = params
                .get("commit")
                .and_then(|v| v.as_str())
                .unwrap_or("HEAD");

            // Hard reset (discard changes)
            if commit == "HEAD" {
                reset_to_last_commit(&cwd, true).map_err(|e| {
                    ToolError::ExecutionError(format!("Failed to reset to last commit: {}", e))
                })?;
            } else {
                reset_to_commit(&cwd, commit, true).map_err(|e| {
                    ToolError::ExecutionError(format!(
                        "Failed to reset to commit {}: {}",
                        commit, e
                    ))
                })?;
            }

            Ok(vec![Content::text(formatdoc! {r#"
                Successfully performed a hard reset to {}.
                
                All changes since that commit have been discarded.
                The working directory is now clean and matches the state at that commit.
            "#, if commit == "HEAD" { "the last commit".to_string() } else { format!("commit {}", commit) }})])
        }
        _ => Err(ToolError::InvalidParameters(format!(
            "Unknown action: {}",
            action
        ))),
    }
}
