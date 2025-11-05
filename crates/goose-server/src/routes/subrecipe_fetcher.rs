use anyhow::{anyhow, Result};
use goose::recipe::SubRecipe;
use std::fs;
use std::path::Path;
use std::time::Duration;

const GOOSE_RECIPES_REPO: &str = "block/goose";
const GOOSE_RECIPES_BRANCH: &str = "main";
const RECIPES_BASE_PATH: &str = "documentation/src/pages/recipes/data/recipes";
const REQUEST_TIMEOUT_SECS: u64 = 10;

async fn fetch_subrecipe_from_github(sub_recipe: &SubRecipe) -> Result<String> {
    let path = Path::new(&sub_recipe.path);

    let filename = if sub_recipe.path.starts_with("./") || sub_recipe.path.starts_with("../") {
        path.file_name()
            .and_then(|f| f.to_str())
            .ok_or_else(|| anyhow!("Invalid subrecipe path: {}", sub_recipe.path))?
    } else {
        sub_recipe.path.as_str()
    };

    let url = format!(
        "https://raw.githubusercontent.com/{}/{}/{}/subrecipes/{}",
        GOOSE_RECIPES_REPO, GOOSE_RECIPES_BRANCH, RECIPES_BASE_PATH, filename
    );

    tracing::info!("Fetching subrecipe from: {}", url);

    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()?;
    let response = client.get(&url).send().await.map_err(|e| {
        if e.is_timeout() {
            anyhow!(
                "Request timed out after {}s fetching subrecipe from GitHub: {}",
                REQUEST_TIMEOUT_SECS,
                e
            )
        } else {
            anyhow!("Failed to fetch subrecipe from GitHub: {}", e)
        }
    })?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "Failed to fetch subrecipe '{}': HTTP {}",
            filename,
            response.status()
        ));
    }

    let content = response
        .text()
        .await
        .map_err(|e| anyhow!("Failed to read subrecipe content: {}", e))?;

    Ok(content)
}

pub async fn fetch_and_store_subrecipes(
    sub_recipes: Vec<SubRecipe>,
    recipe_id: &str,
) -> Result<Vec<SubRecipe>> {
    // Sanitize recipe_id to prevent directory traversal
    let recipe_id = recipe_id.replace(['/', '\\', '.'], "_");
    let mut updated_sub_recipes = Vec::new();

    for sub_recipe in sub_recipes {
        match fetch_subrecipe_from_github(&sub_recipe).await {
            Ok(content) => {
                let temp_dir = std::env::temp_dir()
                    .join("goose_subrecipes")
                    .join(&recipe_id);
                fs::create_dir_all(&temp_dir)?;

                let filename = Path::new(&sub_recipe.path)
                    .file_name()
                    .and_then(|f| f.to_str())
                    .ok_or_else(|| anyhow!("Invalid subrecipe path: {}", sub_recipe.path))?;

                let local_path = temp_dir.join(filename);
                fs::write(&local_path, content)?;

                tracing::info!(
                    "Stored subrecipe '{}' at: {}",
                    sub_recipe.name,
                    local_path.display()
                );

                let updated_sub_recipe = SubRecipe {
                    name: sub_recipe.name.clone(),
                    path: local_path.to_string_lossy().to_string(),
                    values: sub_recipe.values.clone(),
                    sequential_when_repeated: sub_recipe.sequential_when_repeated,
                    description: sub_recipe.description.clone(),
                };

                updated_sub_recipes.push(updated_sub_recipe);
            }
            Err(e) => {
                tracing::warn!(
                    "Failed to fetch subrecipe '{}': {}. Skipping.",
                    sub_recipe.name,
                    e
                );
                // Continue with other subrecipes even if one fails
            }
        }
    }

    Ok(updated_sub_recipes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_fetch_subrecipe_from_github() {
        let sub_recipe = SubRecipe {
            name: "experiment_tracker".to_string(),
            path: "./subrecipes/experiment-tracker.yaml".to_string(),
            values: None,
            sequential_when_repeated: false,
            description: None,
        };

        let result = fetch_subrecipe_from_github(&sub_recipe).await;

        assert!(
            result.is_ok(),
            "Failed to fetch subrecipe: {:?}",
            result.err()
        );

        let content = result.unwrap();
        assert!(!content.is_empty(), "Subrecipe content should not be empty");
        assert!(
            content.contains("title:") || content.contains("description:"),
            "Subrecipe content should contain expected YAML fields"
        );
    }
}
