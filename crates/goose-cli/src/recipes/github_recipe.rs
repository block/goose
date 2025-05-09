use anyhow::Result;
use std::env;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

pub const GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY: &str = "GOOSE_RECIPE_GITHUB_REPO";
pub fn retrieve_recipe_from_github(
    recipe_name: &str,
    recipe_repo_full_name: &str,
) -> Result<String> {
    println!(
        "retrieving recipe from github repo {}",
        recipe_repo_full_name
    );
    ensure_gh_authenticated()?;
    let local_repo_path = ensure_repo_cloned(recipe_repo_full_name)?;
    fetch_origin(&local_repo_path)?;
    let file_extensions = ["yaml", "json"];

    for ext in file_extensions {
        let file_path_in_repo = format!("{}/recipe.{}", recipe_name, ext);
        match get_file_content_from_github(&local_repo_path, &file_path_in_repo) {
            Ok(content) => {
                println!(
                    "retrieved recipe from github repo {}/{}",
                    recipe_repo_full_name, file_path_in_repo
                );
                return Ok(content);
            }
            Err(_) => continue,
        }
    }
    Err(anyhow::anyhow!(
        "Failed to retrieve recipe.yaml or recipe.json in path {} in github repo {} ",
        recipe_name,
        recipe_repo_full_name,
    ))
}

pub fn get_file_content_from_github(
    local_repo_path: &Path,
    file_path_in_repo: &str,
) -> Result<String> {
    let ref_and_path = format!("origin/main:{}", file_path_in_repo);
    let error_message: String = format!(
        "Failed to get content from {} in github repo",
        file_path_in_repo
    );
    let output = Command::new("git")
        .args(["show", &ref_and_path])
        .current_dir(local_repo_path)
        .output()
        .map_err(|_: std::io::Error| anyhow::anyhow!(error_message.clone()))?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        Err(anyhow::anyhow!(error_message.clone()))
    }
}

fn ensure_gh_authenticated() -> Result<()> {
    // Check authentication status
    let status = Command::new("gh")
        .args(["auth", "status"])
        .status()
        .map_err(|_| {
            anyhow::anyhow!("Failed to run `gh auth status`. Make sure you have `gh` installed.")
        })?;

    if status.success() {
        return Ok(());
    }
    println!("GitHub CLI is not authenticated. Launching `gh auth login`...");
    // Run `gh auth login` interactively
    let login_status = Command::new("gh")
        .args(["auth", "login"])
        .status()
        .map_err(|_| anyhow::anyhow!("Failed to run `gh auth login`"))?;

    if !login_status.success() {
        Err(anyhow::anyhow!("Failed to authenticate using GitHub CLI."))
    } else {
        Ok(())
    }
}

fn ensure_repo_cloned(recipe_repo_full_name: &str) -> Result<PathBuf> {
    let local_repo_parent_path = env::temp_dir();
    let (_, repo_name) = recipe_repo_full_name
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid repository name format"))?;

    let local_repo_path = local_repo_parent_path.clone().join(repo_name);
    if local_repo_path.join(".git").exists() {
        Ok(local_repo_path)
    } else {
        // Create the local repo parent directory if it doesn't exist
        if !local_repo_parent_path.exists() {
            std::fs::create_dir_all(local_repo_parent_path.clone())?;
        }
        let error_message: String = format!("Failed to clone repo: {}", recipe_repo_full_name);
        let status = Command::new("gh")
            .args(["repo", "clone", recipe_repo_full_name])
            .current_dir(local_repo_parent_path.clone())
            .status()
            .map_err(|_: std::io::Error| anyhow::anyhow!(error_message.clone()))?;

        if status.success() {
            Ok(local_repo_path)
        } else {
            Err(anyhow::anyhow!(error_message))
        }
    }
}

fn fetch_origin(local_repo_path: &Path) -> Result<()> {
    let error_message: String = format!("Failed to fetch at {}", local_repo_path.to_str().unwrap());
    let status = Command::new("git")
        .args(["fetch", "origin"])
        .current_dir(local_repo_path)
        .status()
        .map_err(|_| anyhow::anyhow!(error_message.clone()))?;

    if status.success() {
        Ok(())
    } else {
        Err(anyhow::anyhow!(error_message))
    }
}
