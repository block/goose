use crate::commands::goose_swarm::repo::{
    claim_issue, claim_pr, close_issue, close_planning_issue, close_pr, count_available_nodes,
    create_follow_up_issue, create_planning_issue, create_task_issue, get_all_issues,
    get_help_wanted_issues, get_issue_context, get_open_prs, get_pr_context, get_pr_status,
    is_issue_claimed, is_pr_claimed, register_node, remove_help_wanted_label, GitHubIssue,
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
const EVALUATE_PR_RECIPE: &str = include_str!("goose_swarm/evaluate.yaml");

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

#[derive(Debug, Clone)]
struct PullRequest {
    number: u64,
    title: String,
    body: Option<String>,
}

#[derive(Debug)]
enum WorkType {
    Planning(GitHubIssue),   // Planning issue that needs breakdown
    Task(GitHubIssue),       // Task ready to execute
    Evaluation(PullRequest), // PR ready for evaluation
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

/// Detect available work - prioritizes evaluating PRs, then tasks, then planning work
fn detect_work_type(repo: &str) -> Result<WorkType> {
    // First check for open PRs that need evaluation
    let open_prs = get_open_prs(repo)?;

    // Find non-draft PRs that haven't been evaluated yet
    for pr in open_prs {
        // Skip draft PRs
        if pr.get("isDraft").and_then(|v| v.as_bool()).unwrap_or(false) {
            continue;
        }

        // Extract PR details
        if let (Some(number), Some(title)) = (
            pr.get("number").and_then(|v| v.as_u64()),
            pr.get("title").and_then(|v| v.as_str()),
        ) {
            // Skip if already claimed by another evaluator
            if is_pr_claimed(repo, number)? {
                continue;
            }

            let body = pr.get("body").and_then(|v| v.as_str()).map(String::from);
            return Ok(WorkType::Evaluation(PullRequest {
                number,
                title: title.to_string(),
                body,
            }));
        }
    }

    // Check for tasks and planning work
    let help_wanted = get_help_wanted_issues(repo)?;
    if help_wanted.is_empty() {
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

    // Sort tasks alphabetically by title and pick the first one
    if !available_tasks.is_empty() {
        let mut sorted_tasks = available_tasks;
        sorted_tasks.sort_by(|a, b| a.title.cmp(&b.title));
        return Ok(WorkType::Task(sorted_tasks.into_iter().next().unwrap()));
    }

    // Sort planning issues alphabetically by title and pick the first one
    if !available_planning.is_empty() {
        let mut sorted_planning = available_planning;
        sorted_planning.sort_by(|a, b| a.title.cmp(&b.title));
        return Ok(WorkType::Planning(
            sorted_planning.into_iter().next().unwrap(),
        ));
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

/// Process task files in tasks/ directory to create [task] issues
/// Returns the number of task issues created
fn process_task_files(
    repo: &str,
    parent_issue: u32,
    tasks_dir: &str,
    original_issue_body: &str,
) -> Result<usize> {
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

            create_task_issue(repo, parent_issue, title, &content, original_issue_body)?;
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

    // Build recipe parameters for planning
    let params = vec![
        ("repo".to_string(), repo.to_string()),
        ("issue_number".to_string(), issue.number.to_string()),
        ("context".to_string(), context.clone()),
        ("available_nodes".to_string(), available_nodes.to_string()),
        ("work_dir".to_string(), work_dir.clone()),
        ("repo_dir".to_string(), repo_dir.clone()), // Pass the cloned repo directory
    ];

    // Run the standard planning recipe to create tasks
    println!("🎯 Running standard planning recipe to break down the issue...");
    run_recipe(PLAN_WORK_RECIPE, params).await?;

    // Process task files in tasks/ directory and get the count
    let original_body = issue.body.as_deref().unwrap_or("");
    let task_count = process_task_files(repo, issue.number, &tasks_dir, original_body)?;

    // Process issue files in issues/ directory
    let issue_count = process_issue_files(repo, issue.number, &issues_dir)?;

    // Check if any task issues or follow-on issues were created
    if task_count == 0 && issue_count == 0 {
        println!("ℹ️ No task issues or follow-on issues were created.");
        println!("   This may be intentional if the issue requires no further action.");

        // Still close the planning issue even if no tasks were created
        println!("🔒 Closing planning issue #{}...", issue.number);
        close_planning_issue(repo, issue.number)?;

        println!("✅ Planning complete (no tasks needed).");
        return Ok(());
    }

    println!(
        "📊 Created {} task issues and {} follow-on issues.",
        task_count, issue_count
    );

    // Close the original planning issue now that tasks have been created
    println!("🔒 Closing original planning issue #{}...", issue.number);
    close_planning_issue(repo, issue.number)?;

    // Note: No need to remove claims since the issue is now closed
    // The task issues will be picked up by drone workers independently

    println!(
        "✅ Planning complete. Original issue #{} closed. Task issues created and ready for work.",
        issue.number
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

/// Execute evaluation work for a PR
async fn execute_pr_evaluation(repo: &str, pr: &PullRequest, worker_id: &str) -> Result<()> {
    println!("🔍 Evaluating PR #{}: {}", pr.number, pr.title);

    // Random pause to avoid race conditions (1-30 seconds)
    let wait_seconds = rand::thread_rng().gen_range(1..=30);
    println!(
        "⏸️  Waiting {} seconds before claiming to avoid collisions...",
        wait_seconds
    );
    sleep(Duration::from_secs(wait_seconds)).await;

    // Check if someone else claimed it while we were waiting
    if is_pr_claimed(repo, pr.number)? {
        println!(
            "⚠️ PR #{} was claimed by another evaluator while waiting. Skipping.",
            pr.number
        );
        return Ok(()); // Exit gracefully, will look for more work
    }

    // Claim the PR as evaluator
    claim_pr(repo, pr.number, worker_id)?;

    // Get PR context (diff, comments, etc.)
    let pr_context = get_pr_context(repo, pr.number)?;

    // Create working directory for evaluation
    let home_dir = std::env::var("HOME")
        .or_else(|_| std::env::var("USERPROFILE"))
        .context("Failed to get home directory")?;
    let eval_work_dir = format!(
        "{}/.local/share/goose-swarm/pr-eval-{}",
        home_dir, pr.number
    );
    std::fs::create_dir_all(&eval_work_dir).context("Failed to create evaluation directory")?;

    // Prepare workspace for checking out the PR
    let workspace = prepare_workspace(repo, worker_id)?;

    // Checkout the PR branch
    println!("📥 Checking out PR #{}...", pr.number);
    let output = Command::new("gh")
        .args(["pr", "checkout", &pr.number.to_string()])
        .current_dir(&workspace)
        .output()
        .context("Failed to checkout PR")?;

    if !output.status.success() {
        let error = String::from_utf8_lossy(&output.stderr);
        return Err(anyhow::anyhow!("Failed to checkout PR: {}", error));
    }

    // Build parameters for evaluation recipe
    let eval_params = vec![
        ("repo".to_string(), repo.to_string()),
        ("pr_number".to_string(), pr.number.to_string()),
        ("pr_title".to_string(), pr.title.clone()),
        ("pr_body".to_string(), pr.body.clone().unwrap_or_default()),
        ("pr_context".to_string(), pr_context),
        ("workspace".to_string(), workspace.clone()),
        ("worker_id".to_string(), worker_id.to_string()),
    ];

    // Run the evaluation recipe
    println!("🧪 Running PR evaluation recipe...");
    run_recipe(EVALUATE_PR_RECIPE, eval_params).await?;

    // The recipe should have decided whether to merge or close the PR
    // Check if PR is still open
    let pr_status = get_pr_status(repo, pr.number)?;

    match pr_status.as_str() {
        "MERGED" => println!("✅ PR #{} was merged", pr.number),
        "CLOSED" => println!("❌ PR #{} was closed", pr.number),
        "OPEN" => {
            // If still open, close it with a comment
            println!(
                "⚠️ PR #{} still open after evaluation, closing...",
                pr.number
            );
            close_pr(
                repo,
                pr.number,
                "Evaluation complete. PR not approved for merge.",
            )?;

            // Create a follow-up issue for the failed PR
            println!("📋 Creating follow-up issue for unevaluated PR...");

            // Clean up the PR title - remove [task:N] prefix if present
            let clean_title = pr
                .title
                .split("[task:")
                .next()
                .unwrap_or(&pr.title)
                .trim()
                .to_string();

            let follow_up_body = format!(
                "This is from a failed pull request, consider if still relevant, as it may be useful information or not needed.\n\n{}",
                pr.body.as_ref().unwrap_or(&String::from("(No PR description provided)"))
            );

            create_follow_up_issue(repo, &clean_title, &follow_up_body)?;
            println!("✅ Created follow-up issue for unevaluated PR");
        }
        _ => println!("❓ Unknown PR status: {}", pr_status),
    }

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

    // Get full issue context and body for potential follow-up
    let context = get_issue_context(repo, issue.number)?;
    let task_body = issue.body.clone().unwrap_or_default();

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

    let pr_created;
    if output.status.success() {
        let branch = String::from_utf8_lossy(&output.stdout).trim().to_string();
        if branch == "main" || branch == "master" {
            println!("⚠️ Drone failed to create PR (still on {} branch)", branch);
            pr_created = false;
        } else {
            println!("✅ Successfully created PR on branch: {}", branch);
            pr_created = true;
        }
    } else {
        println!("⚠️ Failed to determine branch status");
        pr_created = false;
    }

    // Always close the task issue
    println!("📝 Closing task issue #{}...", issue.number);
    close_issue(repo, issue.number, "Task processing complete.")?;

    // If PR creation failed, create a follow-up issue
    if !pr_created {
        println!("📋 Creating follow-up issue for failed task...");

        // Remove [task] prefix and [issue:N] suffix, then add "Retrying: " prefix
        let title_without_task = issue.title.replace("[task]", "");
        let clean_title = title_without_task
            .split("[issue:")
            .next()
            .unwrap_or(&title_without_task)
            .trim();
        let follow_up_title = format!("Retrying: {}", clean_title);
        let follow_up_body = format!(
            "We failed to implement this task, consider if it is still needed.\n\n{}",
            task_body
        );
        create_follow_up_issue(repo, &follow_up_title, &follow_up_body)?;
        println!("✅ Created follow-up issue for failed task");
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

        Some(format!(
            "swarm-drone-{}-task-{}",
            worker_id, task_number
        ))
    } else {
        None
    };

    // Build and run the session
    let mut session = build_session(SessionBuilderConfig {
        session_id: session_identifier.clone(),
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
                    Ok(WorkType::Evaluation(pr)) => {
                        if let Err(e) = execute_pr_evaluation(&args.repo, &pr, &worker_id).await {
                            eprintln!("❌ Error during PR evaluation: {}", e);
                        } else {
                            println!("✅ Completed PR evaluation");
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
