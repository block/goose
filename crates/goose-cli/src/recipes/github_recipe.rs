use anyhow::Result;
use std::path::Path;
use std::path::PathBuf;
use std::process::Command;

const GOOSE_RECIPE_REPO_NAME: &str = "goose-recipes";
const GOOSE_RECIPE_GITHUB_CLONE_URL: &str = "org-49461806@github.com:squareup/goose-recipes.git";
pub const GOOSE_RECIPE_GITHUB_HTTP_URL: &str = "https://github.com/squareup/goose-recipes";
const LOCAL_REPO_PARENT_PATH: &str = "/tmp";

pub fn download_github_recipe(recipe_name: &str, target_dir: &Path) -> Result<PathBuf> {
    println!(
        "downloading recipe from github repo {}/{} to {:?}",
        GOOSE_RECIPE_GITHUB_HTTP_URL, recipe_name, target_dir
    );
    let local_repo_parent_path = Path::new(LOCAL_REPO_PARENT_PATH);
    let local_repo_path =
        ensure_repo_cloned(GOOSE_RECIPE_GITHUB_CLONE_URL, local_repo_parent_path)?;
    fetch_origin(&local_repo_path)?;
    let file_extensions = ["yaml", "json"];

    for ext in file_extensions {
        let file_path_in_repo = format!("{}/recipe.{}", recipe_name, ext);
        match get_file_content_from_github(&local_repo_path, &file_path_in_repo) {
            Ok(content) => {
                let downloaded_file_path = target_dir.join(format!("{}.{}", recipe_name, ext));
                std::fs::write(downloaded_file_path.clone(), content)?;
                println!(
                    "downloaded recipe from github repo {}/{}/recipe.{} to {:?}",
                    GOOSE_RECIPE_GITHUB_HTTP_URL, recipe_name, ext, downloaded_file_path
                );
                return Ok(downloaded_file_path);
            }
            Err(_) => continue,
        }
    }
    Err(anyhow::anyhow!(
        "Failed to retrieve recipe.yaml or recipe.json in {} directory in {}",
        GOOSE_RECIPE_REPO_NAME,
        GOOSE_RECIPE_GITHUB_CLONE_URL
    ))
}

pub fn get_file_content_from_github(
    local_repo_path: &Path,
    file_path_in_repo: &str,
) -> Result<String> {
    let ref_and_path = format!("origin/main:{}", file_path_in_repo);
    let error_message: String = format!("Failed to get content from {}", file_path_in_repo);
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

fn ensure_repo_cloned(github_clone_url: &str, local_repo_parent_path: &Path) -> Result<PathBuf> {
    let local_repo_path = local_repo_parent_path.join(GOOSE_RECIPE_REPO_NAME);
    if local_repo_path.join(".git").exists() {
        Ok(local_repo_path)
    } else {
        // Create the local repo parent directory if it doesn't exist
        if !local_repo_parent_path.exists() {
            std::fs::create_dir_all(local_repo_parent_path)?;
        }
        let error_message: String = format!("Failed to clone repo: {}", github_clone_url);
        let status = Command::new("git")
            .args(["clone", github_clone_url, local_repo_path.to_str().unwrap()])
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
