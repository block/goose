use anyhow::{anyhow, Result};
use base64::Engine as _;
use reqwest;
use std::fs;
use std::path::PathBuf;

pub async fn download_github_recipe(recipe_name: &str, target_dir: &PathBuf) -> Result<PathBuf> {
    // Try both yaml and json extensions
    for ext in &["yaml", "json"] {
        let filename = format!("{}.{}", recipe_name, ext);
        let target_path = target_dir.join(&filename);

        // GitHub API URL for the recipe file
        let api_url = format!(
            "https://api.github.com/repos/squareup/goose-recipes/contents/{}/recipe.{}?ref=douwe/joke-of-the-day",
            recipe_name, ext
        );

        println!("Downloading recipe from: {}", api_url);
        let token = std::env::var("GITHUB_TOKEN").unwrap_or_default();
        // Make request to GitHub API
        let client = reqwest::Client::new();
        let response = client
            .get(&api_url)
            .header("Accept", "application/vnd.github.v3+json")
            .header("Authorization", format!("Bearer {}", token))
            .header("X-GitHub-Api-Version", "2022-11-28")
            .header("User-Agent", "goose-cli")
            .send()
            .await?;

        if response.status().is_success() {
            // Parse the GitHub API response
            let github_content: serde_json::Value = response.json().await?;

            // Extract the content (base64 encoded)
            let content = github_content["content"]
                .as_str()
                .ok_or_else(|| anyhow!("Content field missing in GitHub response"))?;

            // Decode base64 content
            let decoded =
                base64::engine::general_purpose::STANDARD.decode(content.replace("\n", ""))?;
            let file_content = String::from_utf8(decoded)?;

            // Write content to local file
            fs::write(&target_path, file_content)?;
            let github_url = format!(
                "https://github.com/squareup/goose-recipes/{}/recipe.{}",
                recipe_name, ext
            );
            println!(
                "Downloaded recipe from GitHub {:?} to : {:?}",
                github_url, target_path
            );
            return Ok(target_path);
        }
    }

    Err(anyhow!("Recipe '{}' not found on GitHub", recipe_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_download_github_recipe() {
        // Get current directory for test
        let current_dir = std::env::current_dir().unwrap();
        let recipe_name = "joke-of-the-day";

        match download_github_recipe(recipe_name, &current_dir).await {
            Ok(path) => println!("Downloaded recipe to: {:?}", path),
            Err(e) => println!("Error downloading recipe: {}", e),
        }
    }
}
