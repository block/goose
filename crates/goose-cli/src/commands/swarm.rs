use crate::commands::goose_swarm::repo::{
    add_help_wanted_label, add_in_progress_label, check_open_prs_for_issue, claim_issue,
    close_planning_issue, close_pr, close_task_issues, count_available_nodes,
    create_evaluation_issue, create_planning_issue, create_task_issue, get_all_issues,
    get_help_wanted_issues, get_in_progress_issues, get_issue_context, get_ready_prs_for_issue,
    get_task_issue_numbers, is_issue_claimed, register_node, remove_all_goose_claims,
    remove_help_wanted_label, remove_in_progress_label, unclaim_issue, GitHubIssue,
};
use crate::session::{build_session, SessionBuilderConfig};
use anyhow::{Context, Result};
use clap::Args;
use rand::Rng;
use std::fs;
use std::path::Path;
use std::process::Command;
use tokio::time::{sleep, Duration};

// Embed recipe YAML files at compile time
const PLAN_WORK_RECIPE: &str = include_str!("goose_swarm/plan_work.yaml");
const SWARM_DRONE_RECIPE: &str = include_str!("goose_swarm/swarm_drone.yaml");
const EVALUATE_RECIPE: &str = include_str!("goose_swarm/evaluate.yaml");
const INIT_WORK_RECIPE: &str = include_str!("goose_swarm/init_work.yaml");

/// Orchestrate parallel work on GitHub issues using swarm intelligence
#[derive(Args, Debug)]
pub struct SwarmArgs {
    /// GitHub repository in format owner/repo
    #[arg(help = "GitHub repository (e.g., owner/repo)")]
    pub repo: String,

    /// Optional worker ID for this swarm agent
    #[arg(
        long,
        help = "Worker ID for this swarm agent (auto-generated if not provided)"
    )]
    pub worker_id: Option<String>,

    /// Poll interval in seconds (default: 30)
    #[arg(long, default_value = "30", help = "Polling interval in seconds")]
    pub poll_interval: u64,
}

#[derive(Debug)]
enum WorkType {
    Planning(GitHubIssue),   // Planning issue that needs breakdown
    Task(GitHubIssue),       // Task ready to execute
    Evaluation(GitHubIssue), // Issue ready for evaluation (has all PRs)
    None,                    // No work available
}

/// Generate a unique worker ID with fun names
fn generate_worker_id() -> String {
    // Fun adjectives for the first word
    let adjectives = [
        "happy", "brave", "clever", "swift", "mighty", "gentle", "bright", "eager", "noble",
        "bold", "wise", "keen", "quick", "sharp", "calm", "cool", "smart", "lucky", "proud",
        "strong", "wild", "free", "grand", "true", "fair", "kind", "warm", "neat", "glad", "busy",
        "spry", "jolly",
    ];

    // Fun nouns for the last word
    let nouns = [
        "fox", "owl", "bee", "cat", "dog", "bat", "ant", "elk", "emu", "jay", "hawk", "wolf",
        "bear", "lion", "deer", "duck", "crow", "dove", "fish", "frog", "goat", "hare", "moth",
        "newt", "orca", "puma", "quail", "raven", "seal", "swan", "toad", "vole", "wasp", "yak",
        "lynx", "mole", "otter",
    ];

    let adj_idx = rand::random::<usize>() % adjectives.len();
    let noun_idx = rand::random::<usize>() % nouns.len();
    let timestamp = chrono::Utc::now().format("%m%d");

    format!("{}-{}-{}", adjectives[adj_idx], timestamp, nouns[noun_idx])
}

/// Load or generate worker ID, persisting it to .node-id file
fn get_or_create_worker_id(provided_id: Option<String>) -> Result<String> {
    // If explicitly provided, use that
    if let Some(id) = provided_id {
        return Ok(id);
    }

    // Check for .node-id file in current directory
    let node_id_path = Path::new(".node-id");

    if node_id_path.exists() {
        // Try to read existing ID
        match fs::read_to_string(node_id_path) {
            Ok(contents) => {
                let id = contents.trim().to_string();
                if !id.is_empty() {
                    println!("📎 Loaded worker ID from .node-id: {}", id);
                    return Ok(id);
                }
            }
            Err(e) => {
                println!("⚠️ Failed to read .node-id: {}", e);
            }
        }
    }

    // Generate new ID
    let new_id = generate_worker_id();

    // Try to save it
    if let Err(e) = fs::write(node_id_path, &new_id) {
        println!("⚠️ Failed to save .node-id: {}", e);
        println!("   Worker ID will be regenerated on next run");
    } else {
        println!("💾 Saved worker ID to .node-id for future runs");
    }

    Ok(new_id)
}

/// Detect available work - prioritizes tasks, then planning work, then evaluation work
fn detect_work_type(repo: &str) -> Result<WorkType> {
    let in_progress = get_in_progress_issues(repo)?;
    let help_wanted = get_help_wanted_issues(repo)?;

    if help_wanted.is_empty() && in_progress.is_empty() {
        return Ok(WorkType::None);
    }

    // Get full details for all issues to classify them
    let all_issues = get_all_issues(repo)?;

    // Collect available work, separating tasks and planning issues
    let mut available_tasks = Vec::new();
    let mut available_planning = Vec::new();

    for issue_num in help_wanted {
        // Skip if already claimed
        if is_issue_claimed(repo, issue_num)? {
            continue;
        }

        // Find the full issue details
        if let Some(issue) = all_issues.iter().find(|i| i.number == issue_num) {
            // Check if it has help wanted label
            let has_help_wanted = issue.labels.iter().any(|l| l.name == "help wanted");

            if has_help_wanted {
                // Determine role based on title prefix
                if issue.title.starts_with("[task]") {
                    // Task issue = drone/worker role
                    available_tasks.push(issue.clone());
                } else {
                    // Non-task issue with help wanted = planner role
                    available_planning.push(issue.clone());
                }
            }
        }
    }

    // Prioritize tasks over planning work
    if !available_tasks.is_empty() {
        // Return the first available task
        return Ok(WorkType::Task(available_tasks.into_iter().next().unwrap()));
    }

    if !available_planning.is_empty() {
        // Return the first available planning issue
        return Ok(WorkType::Planning(
            available_planning.into_iter().next().unwrap(),
        ));
    }

    // Next, check for evaluation work in progress issues
    if !in_progress.is_empty() {
        for issue_num in in_progress {
            // Skip if already claimed
            if is_issue_claimed(repo, issue_num)? {
                continue;
            }

            // Get the task issues for this parent issue
            let task_issues = get_task_issue_numbers(repo, issue_num)?;

            if !task_issues.is_empty() {
                // Check if all tasks have ready PRs
                let ready_prs = get_ready_prs_for_issue(repo, issue_num)?;

                if ready_prs.len() >= task_issues.len() {
                    // All tasks have PRs - ready for evaluation
                    if let Some(issue) = all_issues.iter().find(|i| i.number == issue_num) {
                        return Ok(WorkType::Evaluation(issue.clone()));
                    }
                }
            }
        }
    }

    Ok(WorkType::None)
}

/// Clone or refresh repository to workspace
fn prepare_workspace(repo: &str, worker_id: &str) -> Result<String> {
    let repo_name = repo.split('/').next_back().unwrap_or(repo);

    // Use ~/.local/share/goose-swarm as per design spec
    // Include worker_id to make directory unique per node
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Failed to get home directory")?;
    let workspace_dir = format!(
        "{}/.local/share/goose-swarm/{}-{}",
        home_dir, repo_name, worker_id
    );

    // Create workspace directory
    std::fs::create_dir_all(&workspace_dir).context("Failed to create workspace")?;

    // Check if repo already cloned
    if !std::path::Path::new(&format!("{}/.git", workspace_dir)).exists() {
        println!("📦 Cloning repository...");
        let output = Command::new("gh")
            .args(["repo", "clone", repo, &workspace_dir])
            .output()
            .context("Failed to clone repository")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to clone: {}", error));
        }
    } else {
        // Pull latest changes
        println!("🔄 Updating repository...");

        // First, fetch the latest refs to know what branches are available
        let output = Command::new("git")
            .args(["fetch", "origin"])
            .current_dir(&workspace_dir)
            .output()
            .context("Failed to fetch from origin")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!("Failed to fetch: {}", error));
        }

        // Determine the default branch (try main first, then master)
        let default_branch = {
            let output = Command::new("git")
                .args(["rev-parse", "--verify", "origin/main"])
                .current_dir(&workspace_dir)
                .output();

            if output.is_ok() && output.unwrap().status.success() {
                "main"
            } else {
                "master"
            }
        };

        // Switch to the default branch
        println!("🔀 Switching to {} branch...", default_branch);
        let output = Command::new("git")
            .args(["checkout", default_branch])
            .current_dir(&workspace_dir)
            .output()
            .context("Failed to checkout branch")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to checkout {}: {}",
                default_branch,
                error
            ));
        }

        // Pull latest changes from the default branch
        println!("⬇️ Pulling latest changes from {}...", default_branch);
        let output = Command::new("git")
            .args(["pull", "origin", default_branch])
            .current_dir(&workspace_dir)
            .output()
            .context("Failed to pull from origin")?;

        if !output.status.success() {
            let error = String::from_utf8_lossy(&output.stderr);
            return Err(anyhow::anyhow!(
                "Failed to pull from {}: {}",
                default_branch,
                error
            ));
        }
    }

    Ok(workspace_dir)
}

/// Check if the repository has a .workflow directory
fn has_workflow_directory(repo_dir: &str) -> bool {
    let workflow_path = format!("{}/.workflow", repo_dir);
    std::path::Path::new(&workflow_path).exists()
}

/// Process task files in tasks/ directory to create [task] issues
/// Returns the number of task issues created
fn process_task_files(repo: &str, parent_issue: u32, tasks_dir: &str) -> Result<usize> {
    let entries = std::fs::read_dir(tasks_dir).context("Failed to read tasks directory")?;
    let mut task_count = 0;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let content = std::fs::read_to_string(&path).context("Failed to read task file")?;

            // Extract title from first line (should be a markdown header)
            let title = content
                .lines()
                .next()
                .unwrap_or("Task")
                .trim_start_matches('#')
                .trim();

            create_task_issue(repo, parent_issue, title, &content)?;
            task_count += 1;
        }
    }

    Ok(task_count)
}

/// Process issue files in issues/ directory to create non-task issues
/// Returns the number of issues created
fn process_issue_files(repo: &str, parent_issue: u32, issues_dir: &str) -> Result<usize> {
    let entries = std::fs::read_dir(issues_dir).context("Failed to read issues directory")?;
    let mut issue_count = 0;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let content = std::fs::read_to_string(&path).context("Failed to read issue file")?;

            // Extract title from first line (should be a markdown header)
            let title = content
                .lines()
                .next()
                .unwrap_or("Issue")
                .trim_start_matches('#')
                .trim();

            create_planning_issue(repo, parent_issue, title, &content)?;
            issue_count += 1;
        }
    }

    Ok(issue_count)
}

/// Process new issues created by the evaluation recipe
fn process_evaluation_issues(repo: &str, parent_issue: u32, issues_dir: &str) -> Result<()> {
    let entries = match std::fs::read_dir(issues_dir) {
        Ok(entries) => entries,
        Err(_) => return Ok(()), // No issues directory or empty
    };

    for entry in entries {
        let entry = entry?;
        let path = entry.path();

        if path.extension().and_then(|s| s.to_str()) == Some("md") {
            let content =
                std::fs::read_to_string(&path).context("Failed to read evaluation issue file")?;

            // Extract title from first line (should be a markdown header)
            let title = content
                .lines()
                .next()
                .unwrap_or("Follow-up Issue")
                .trim_start_matches('#')
                .trim();

            create_evaluation_issue(repo, parent_issue, title, &content)?;
        }
    }

    Ok(())
}

/// Execute planning work
#[allow(clippy::too_many_lines)]
async fn execute_planning_work(repo: &str, issue: &GitHubIssue, worker_id: &str) -> Result<()> {
    println!(
        "📋 Working on planning issue #{}: {}",
        issue.number, issue.title
    );

    // Remove help wanted label first
    remove_help_wanted_label(repo, issue.number)?;

    // Random pause to avoid race conditions (1-30 seconds)
    let wait_seconds = rand::thread_rng().gen_range(1..=30);
    println!(
        "⏸️  Waiting {} seconds before claiming to avoid collisions...",
        wait_seconds
    );
    sleep(Duration::from_secs(wait_seconds)).await;

    // Check if someone else claimed it while we were waiting
    if is_issue_claimed(repo, issue.number)? {
        println!(
            "⚠️ Issue #{} was claimed by another worker while waiting. Skipping.",
            issue.number
        );
        return Ok(()); // Exit gracefully, will look for more work
    }

    // Claim the issue as planner
    claim_issue(repo, issue.number, worker_id, "planner")?;

    // Get full issue context
    let context = get_issue_context(repo, issue.number)?;

    // Count available drone nodes
    let available_nodes = count_available_nodes(repo)?;

    // Prepare repository workspace (shared location for read-only access)
    let repo_dir = prepare_workspace(repo, worker_id)?;
    println!("📁 Repository available at: {}", repo_dir);

    // Create working directory with tasks/ and issues/ subdirectories
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Failed to get home directory")?;
    let work_dir = format!(
        "{}/.local/share/goose-swarm/planner-work-{}",
        home_dir, issue.number
    );
    let tasks_dir = format!("{}/tasks", work_dir);
    let issues_dir = format!("{}/issues", work_dir);

    prepare_task_directories(&tasks_dir, &issues_dir)?;

    // Check if .workflow directory exists and run initialization recipe if needed
    if !has_workflow_directory(&repo_dir) {
        println!("🚀 No .workflow directory found, running initialization recipe first...");
    } else {
        println!("📂 Found .workflow directory, skipping initialization.");
    }

    // Build recipe parameters for planning
    let params = vec![
        ("repo".to_string(), repo.to_string()),
        ("issue_number".to_string(), issue.number.to_string()),
        ("context".to_string(), context.clone()),
        ("available_nodes".to_string(), available_nodes.to_string()),
        ("work_dir".to_string(), work_dir.clone()),
        ("repo_dir".to_string(), repo_dir.clone()), // Pass the cloned repo directory
    ];

    // Run initialization recipe if needed
    if !has_workflow_directory(&repo_dir) {
        run_recipe(INIT_WORK_RECIPE, params.clone()).await?;
        println!("✅ Initialization recipe complete.");
    }

    // Run the standard planning recipe to create tasks
    println!("🎯 Running standard planning recipe to break down the issue...");
    run_recipe(PLAN_WORK_RECIPE, params).await?;

    // Process task files in tasks/ directory and get the count
    let task_count = process_task_files(repo, issue.number, &tasks_dir)?;

    // Process issue files in issues/ directory
    let issue_count = process_issue_files(repo, issue.number, &issues_dir)?;

    // Check if any task issues or follow-on issues were created
    if task_count == 0 && issue_count == 0 {
        println!("⚠️ No task issues or follow-on issues were created. Planning may have failed.");
        println!("🔓 Removing claim and restoring help wanted label...");

        // Remove the claim from the issue
        unclaim_issue(repo, issue.number, worker_id)?;

        // Re-add help wanted label so others can try
        add_help_wanted_label(repo, issue.number)?;

        println!(
            "↩️ Issue #{} is available for other workers to attempt.",
            issue.number
        );
        return Ok(());
    }

    println!(
        "📊 Created {} task issues and {} follow-on issues.",
        task_count, issue_count
    );

    // Add "in progress" label to mark that tasks are being worked on
    add_in_progress_label(repo, issue.number)?;

    // Remove the planner claim so evaluator can pick it up later
    remove_all_goose_claims(repo, issue.number)?;

    println!(
        "✅ Planning complete. Issue marked as 'in progress' for evaluation when PRs are ready."
    );

    Ok(())
}

fn prepare_task_directories(tasks_dir: &String, issues_dir: &String) -> Result<(), anyhow::Error> {
    std::fs::create_dir_all(tasks_dir).context("Failed to create tasks directory")?;
    std::fs::create_dir_all(issues_dir).context("Failed to create issues directory")?;
    for entry in fs::read_dir(tasks_dir)? {
        let entry = entry?;
        fs::remove_file(entry.path())?;
    }
    for entry in fs::read_dir(issues_dir)? {
        let entry = entry?;
        fs::remove_file(entry.path())?;
    }
    Ok(())
}

/// Execute evaluation work for issues with all PRs ready
async fn execute_evaluation_work(repo: &str, issue: &GitHubIssue, worker_id: &str) -> Result<()> {
    println!(
        "🔮 Ready to evaluate issue #{}: {}",
        issue.number, issue.title
    );

    // Remove "in progress" label first
    remove_in_progress_label(repo, issue.number)?;

    // Random pause to avoid race conditions (1-30 seconds)
    let wait_seconds = rand::thread_rng().gen_range(1..=30);
    println!(
        "⏸️  Waiting {} seconds before claiming as evaluator...",
        wait_seconds
    );
    sleep(Duration::from_secs(wait_seconds)).await;

    // Check if someone else claimed it while we were waiting
    if is_issue_claimed(repo, issue.number)? {
        println!(
            "⚠️ Issue #{} was claimed by another evaluator while waiting. Skipping.",
            issue.number
        );
        return Ok(()); // Exit gracefully, will look for more work
    }

    // Claim the issue as evaluator
    claim_issue(repo, issue.number, worker_id, "evaluator")?;

    // Get full issue context
    let context = get_issue_context(repo, issue.number)?;

    // Get the task issues and count
    let task_issues = get_task_issue_numbers(repo, issue.number)?;
    let task_count = task_issues.len();

    // Get ready PRs
    let ready_prs = get_ready_prs_for_issue(repo, issue.number)?;

    println!(
        "📊 Evaluating {} PRs for {} tasks",
        ready_prs.len(),
        task_count
    );

    // Format PR list as simple comma-separated number/status pairs
    let pr_list = if ready_prs.is_empty() {
        "none".to_string()
    } else {
        let pr_entries: Vec<String> = ready_prs
            .iter()
            .filter_map(|pr| {
                let pr_num = pr.get("number").and_then(|v| v.as_u64())?;
                let state = pr
                    .get("state")
                    .and_then(|v| v.as_str())
                    .unwrap_or("unknown");
                Some(format!("{}/{}", pr_num, state))
            })
            .collect();
        pr_entries.join(", ")
    };

    // Create evaluation working directory with issues/ subdirectory
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Failed to get home directory")?;
    let eval_work_dir = format!(
        "{}/.local/share/goose-swarm/eval-work-{}",
        home_dir, issue.number
    );
    let eval_issues_dir = format!("{}/issues", eval_work_dir);
    std::fs::create_dir_all(&eval_issues_dir)
        .context("Failed to create evaluation issues directory")?;

    // Build parameters for evaluate recipe
    let eval_params = vec![
        ("repo".to_string(), repo.to_string()),
        ("issue_number".to_string(), issue.number.to_string()),
        ("context".to_string(), context),
        ("pr_list".to_string(), pr_list), // Simple PR list like "1234/open, 1235/closed"
    ];

    // Run the evaluate recipe
    println!("🔮 Running evaluation recipe...");
    run_recipe(EVALUATE_RECIPE, eval_params.clone()).await?;

    // Process any new issues created by evaluate recipe
    process_evaluation_issues(repo, issue.number, &eval_issues_dir)?;

    // Check if all PRs are closed/merged
    let remaining_open_prs = check_open_prs_for_issue(repo, issue.number)?;

    if !remaining_open_prs.is_empty() {
        println!(
            "⚠️ {} PR(s) still open after evaluation. Running evaluation once more...",
            remaining_open_prs.len()
        );

        // Run evaluation one more time to give it a chance to address open PRs
        run_recipe(EVALUATE_RECIPE, eval_params).await?;

        // Process any new issues from second evaluation
        process_evaluation_issues(repo, issue.number, &eval_issues_dir)?;

        // Check PRs again
        let final_open_prs = check_open_prs_for_issue(repo, issue.number)?;

        if !final_open_prs.is_empty() {
            println!(
                "📝 {} PR(s) still open. Closing them now...",
                final_open_prs.len()
            );

            // Close all remaining open PRs
            for pr in final_open_prs {
                if let Some(pr_num) = pr.get("number").and_then(|n| n.as_u64()) {
                    close_pr(
                        repo,
                        pr_num,
                        "Automatically closed by Goose Swarm after evaluation phase.",
                    )?;
                }
            }
        }
    }

    // Close all task issues that were created
    close_task_issues(repo, issue.number)?;

    // Close the original planning issue
    close_planning_issue(repo, issue.number)?;

    println!("✅ Evaluation complete. Issue #{} closed.", issue.number);

    Ok(())
}

/// Extract issue number from title containing [issue:N] pattern
fn extract_issue_number_from_title(title: &str) -> Option<u32> {
    // Look for pattern [issue:N] in the title
    if let Some(start) = title.find("[issue:") {
        let rest = &title[start + 7..];
        if let Some(end) = rest.find(']') {
            return rest[..end].parse().ok();
        }
    }
    None
}

/// Execute task work
async fn execute_task_work(repo: &str, issue: &GitHubIssue, worker_id: &str) -> Result<()> {
    println!("🔧 Working on task #{}: {}", issue.number, issue.title);

    // Remove help wanted label
    remove_help_wanted_label(repo, issue.number)?;

    // Random pause to avoid race conditions (1-30 seconds)
    let wait_seconds = rand::thread_rng().gen_range(1..=30);
    println!(
        "⏸️  Waiting {} seconds before claiming to avoid collisions...",
        wait_seconds
    );
    sleep(Duration::from_secs(wait_seconds)).await;

    // Check if someone else claimed it while we were waiting
    if is_issue_claimed(repo, issue.number)? {
        println!(
            "⚠️ Task #{} was claimed by another worker while waiting. Skipping.",
            issue.number
        );
        return Ok(()); // Exit gracefully, will look for more work
    }

    // Claim the issue as drone
    claim_issue(repo, issue.number, worker_id, "drone")?;

    // Get full issue context
    let context = get_issue_context(repo, issue.number)?;

    // Prepare workspace
    let workspace = prepare_workspace(repo, worker_id)?;
    println!("📁 Workspace: {}", workspace);

    // Build recipe parameters
    let mut params = vec![
        ("repo".to_string(), repo.to_string()),
        ("task_number".to_string(), issue.number.to_string()),
        ("worker_id".to_string(), worker_id.to_string()),
        ("context".to_string(), context),
        ("workspace".to_string(), workspace.clone()),
    ];

    // Extract original issue number from title (format: [issue:N])
    // This is REQUIRED for task issues - fail if not present
    let original_id = extract_issue_number_from_title(&issue.title).ok_or_else(|| {
        anyhow::anyhow!(
            "Task issue #{} is missing [issue:N] pattern in title: '{}'",
            issue.number,
            issue.title
        )
    })?;

    params.push(("original_issue".to_string(), original_id.to_string()));

    // Run the worker recipe
    run_recipe(SWARM_DRONE_RECIPE, params).await?;

    // Check if still on main branch (indicates failure)
    let output = Command::new("git")
        .args(["branch", "--show-current"])
        .current_dir(&workspace)
        .output()
        .context("Failed to check current branch")?;

    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch == "main" || branch == "master" {
            println!("⚠️ Drone failed to create PR (still on {} branch)", branch);

            // Remove claim from issue body
            unclaim_issue(repo, issue.number, worker_id)?;

            // Re-add help wanted label
            add_help_wanted_label(repo, issue.number)?;

            return Err(anyhow::anyhow!("Failed to complete task - no PR created"));
        } else {
            println!("✅ Successfully created PR on branch: {}", branch);
        }
    }

    Ok(())
}

/// Run a recipe with given parameters
/// For swarm_drone, use a session identifier to allow for persistence
async fn run_recipe(recipe_content: &str, params: Vec<(String, String)>) -> Result<()> {
    // Parse the YAML content directly
    let recipe: goose::recipe::Recipe =
        serde_yaml::from_str(recipe_content).context("Failed to parse recipe YAML")?;

    // Build the prompt from recipe template and parameters
    let prompt = if let Some(mut template) = recipe.prompt.clone() {
        for (key, value) in &params {
            let placeholder = format!("{{{{ {} }}}}", key);
            template = template.replace(&placeholder, value);
        }
        template
    } else {
        return Err(anyhow::anyhow!("Recipe does not have a prompt template"));
    };

    // Convert recipe Settings to SessionSettings
    let session_settings = recipe
        .settings
        .clone()
        .map(|s| crate::session::SessionSettings {
            goose_provider: s.goose_provider,
            goose_model: s.goose_model,
            temperature: s.temperature,
        });

    // Create input config from the recipe
    let input_config = crate::cli::InputConfig {
        contents: Some(prompt),
        extensions_override: recipe.extensions.clone(),
        additional_system_prompt: None,
    };

    // Create recipe info from the recipe
    let recipe_info = crate::cli::RecipeInfo {
        session_settings,
        sub_recipes: recipe.sub_recipes.clone(),
        final_output_response: recipe.response.clone(),
        retry_config: recipe.retry.clone(),
    };

    // Determine if this is a swarm_drone task that needs a session
    // We check if task_number is in the params to identify drone work
    let needs_session = params.iter().any(|(k, _)| k == "task_number");

    // Create a session identifier for drone tasks
    let session_identifier = if needs_session {
        // Extract worker_id and task_number for unique session name
        let worker_id = params
            .iter()
            .find(|(k, _)| k == "worker_id")
            .map(|(_, v)| v.as_str())
            .unwrap_or("unknown");
        let task_number = params
            .iter()
            .find(|(k, _)| k == "task_number")
            .map(|(_, v)| v.as_str())
            .unwrap_or("0");

        Some(goose::session::Identifier::Name(format!(
            "swarm-drone-{}-task-{}",
            worker_id, task_number
        )))
    } else {
        None
    };

    // Build and run the session
    let mut session = build_session(SessionBuilderConfig {
        identifier: session_identifier.clone(),
        resume: false,
        no_session: !needs_session, // Use session for drone tasks
        extensions: Vec::new(),
        remote_extensions: Vec::new(),
        streamable_http_extensions: Vec::new(),
        builtins: vec!["developer".to_string()],
        extensions_override: input_config.extensions_override,
        additional_system_prompt: input_config.additional_system_prompt,
        settings: recipe_info.session_settings.clone(),
        provider: None,
        model: None,
        debug: false,
        max_tool_repetitions: None,
        max_turns: Some(100),
        scheduled_job_id: None,
        interactive: false,
        quiet: false,
        sub_recipes: recipe_info.sub_recipes.clone(),
        final_output_response: recipe_info.final_output_response.clone(),
        retry_config: recipe_info.retry_config.clone(),
    })
    .await;

    // Run the session
    if let Some(contents) = input_config.contents {
        session.headless(contents).await?;
    } else {
        return Err(anyhow::anyhow!("Recipe did not provide a prompt"));
    }

    Ok(())
}

pub async fn run(args: SwarmArgs) -> Result<()> {
    let worker_id = get_or_create_worker_id(args.worker_id)?;

    println!("🐝 Starting Goose Swarm");
    println!("📍 Repository: {}", args.repo);
    println!("🤖 Worker ID: {}", worker_id);
    println!("⏱️ Poll interval: {} seconds\n", args.poll_interval);

    // Register this node in the swarm issue
    register_node(&args.repo, &worker_id)?;

    // Set up Ctrl+C handler
    let shutdown = tokio::signal::ctrl_c();
    tokio::pin!(shutdown);

    // Main polling loop with graceful shutdown
    loop {
        tokio::select! {
            _ = &mut shutdown => {
                println!("\n\n🛑 Received shutdown signal, exiting gracefully...");
                println!("👋 Goose Swarm worker {} shutting down", worker_id);
                break;
            }
            result = async {
                println!("🔍 Polling for available work...");

                match detect_work_type(&args.repo) {
                    Ok(WorkType::Planning(issue)) => {
                        if let Err(e) = execute_planning_work(&args.repo, &issue, &worker_id).await {
                            eprintln!("❌ Error during planning work: {}", e);
                        } else {
                            println!("✅ Completed planning work");
                        }
                    }
                    Ok(WorkType::Task(issue)) => {
                        if let Err(e) = execute_task_work(&args.repo, &issue, &worker_id).await {
                            eprintln!("❌ Error during task work: {}", e);
                        } else {
                            println!("✅ Completed task work");
                        }
                    }
                    Ok(WorkType::Evaluation(issue)) => {
                        if let Err(e) = execute_evaluation_work(&args.repo, &issue, &worker_id).await {
                            eprintln!("❌ Error during evaluation work: {}", e);
                        } else {
                            println!("✅ Completed evaluation work");
                        }
                    }
                    Ok(WorkType::None) => {
                        println!(
                            "💤 No work available, waiting {} seconds...",
                            args.poll_interval
                        );
                        sleep(Duration::from_secs(args.poll_interval)).await;
                    }
                    Err(e) => {
                        eprintln!("❌ Error detecting work type: {}", e);
                        // Wait before retrying to avoid hammering the API
                        sleep(Duration::from_secs(5)).await;
                    }
                }
                Ok::<(), anyhow::Error>(())
            } => {
                // Continue looping
                let _ = result;
            }
        }
    }

    Ok(())
}
