use anyhow::Result;
use console::style;
use std::env;
use std::fs;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;
use std::process::Stdio;
use tar::Archive;

use crate::recipes::recipe::RECIPE_FILE_EXTENSIONS;
use crate::recipes::search_recipe::RecipeFile;

pub const GOOSE_RECIPE_GITHUB_REPO_CONFIG_KEY: &str = "GOOSE_RECIPE_GITHUB_REPO";
pub fn retrieve_recipe_from_github(
    recipe_name: &str,
    recipe_repo_full_name: &str,
) -> Result<RecipeFile> {
    println!(
        "📦 Looking for recipe \"{}\" in github repo: {}",
        recipe_name, recipe_repo_full_name
    );
    ensure_gh_authenticated()?;
    let max_attempts = 2;
    let mut last_err = None;

    for attempt in 1..=max_attempts {
        match clone_and_download_recipe(recipe_name, recipe_repo_full_name) {
            Ok(download_dir) => match read_recipe_file(&download_dir) {
                Ok((content, recipe_file_local_path)) => {
                    return Ok(RecipeFile {
                        content,
                        parent_dir: download_dir.clone(),
                        file_path: recipe_file_local_path,
                    })
                }
                Err(err) => return Err(err),
            },
            Err(err) => {
                last_err = Some(err);
            }
        }
        if attempt < max_attempts {
            clean_cloned_dirs(recipe_repo_full_name)?;
        }
    }
    Err(last_err.unwrap_or_else(|| anyhow::anyhow!("Unknown error occurred")))
}

fn clean_cloned_dirs(recipe_repo_full_name: &str) -> anyhow::Result<()> {
    let local_repo_path = get_local_repo_path(&env::temp_dir(), recipe_repo_full_name)?;
    if local_repo_path.exists() {
        fs::remove_dir_all(&local_repo_path)?;
    }
    Ok(())
}
fn read_recipe_file(download_dir: &Path) -> Result<(String, PathBuf)> {
    for ext in RECIPE_FILE_EXTENSIONS {
        let candidate_file_path = download_dir.join(format!("recipe.{}", ext));
        if candidate_file_path.exists() {
            let content = fs::read_to_string(&candidate_file_path)?;
            println!(
                "⬇️  Retrieved recipe file: {}",
                candidate_file_path
                    .strip_prefix(download_dir)
                    .unwrap()
                    .display()
            );
            return Ok((content, candidate_file_path));
        }
    }

    Err(anyhow::anyhow!(
        "No recipe file found in {} (looked for extensions: {:?})",
        download_dir.display(),
        RECIPE_FILE_EXTENSIONS
    ))
}

fn clone_and_download_recipe(recipe_name: &str, recipe_repo_full_name: &str) -> Result<PathBuf> {
    let local_repo_path = ensure_repo_cloned(recipe_repo_full_name)?;
    fetch_origin(&local_repo_path)?;
    get_folder_from_github(&local_repo_path, recipe_name)
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
        .args(["auth", "login", "--git-protocol", "https"])
        .status()
        .map_err(|_| anyhow::anyhow!("Failed to run `gh auth login`"))?;

    if !login_status.success() {
        Err(anyhow::anyhow!("Failed to authenticate using GitHub CLI."))
    } else {
        Ok(())
    }
}

fn get_local_repo_path(
    local_repo_parent_path: &Path,
    recipe_repo_full_name: &str,
) -> Result<PathBuf> {
    let (_, repo_name) = recipe_repo_full_name
        .split_once('/')
        .ok_or_else(|| anyhow::anyhow!("Invalid repository name format"))?;
    let local_repo_path = local_repo_parent_path.to_path_buf().join(repo_name);
    Ok(local_repo_path)
}

fn ensure_repo_cloned(recipe_repo_full_name: &str) -> Result<PathBuf> {
    let local_repo_parent_path = env::temp_dir();
    if !local_repo_parent_path.exists() {
        std::fs::create_dir_all(local_repo_parent_path.clone())?;
    }
    let local_repo_path = get_local_repo_path(&local_repo_parent_path, recipe_repo_full_name)?;

    if local_repo_path.join(".git").exists() {
        Ok(local_repo_path)
    } else {
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

fn get_folder_from_github(local_repo_path: &Path, recipe_name: &str) -> Result<PathBuf> {
    let ref_and_path = format!("origin/main:{}", recipe_name);
    let output_dir = env::temp_dir().join(recipe_name);

    if output_dir.exists() {
        fs::remove_dir_all(&output_dir)?;
    }
    fs::create_dir_all(&output_dir)?;

    let archive_output = Command::new("git")
        .args(["archive", &ref_and_path])
        .current_dir(local_repo_path)
        .stdout(Stdio::piped())
        .spawn()?;

    let stdout = archive_output
        .stdout
        .ok_or_else(|| anyhow::anyhow!("Failed to capture stdout from git archive"))?;

    let mut archive = Archive::new(stdout);
    archive.unpack(&output_dir)?;
    list_files(&output_dir)?;

    Ok(output_dir)
}

fn list_files(dir: &Path) -> Result<()> {
    println!("{}", style("Files downloaded from github:").bold());
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() {
            println!("  - {}", path.display());
        }
    }
    Ok(())
}
