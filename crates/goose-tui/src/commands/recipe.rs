use anyhow::Result;
use console::style;
use goose::recipe::read_recipe_file_content::read_recipe_file;
use goose::recipe::validate_recipe::validate_recipe_template_from_file;
use std::collections::HashMap;

use goose::recipe_deeplink;

pub fn handle_validate(recipe_name: &str) -> Result<()> {
    let recipe_file = read_recipe_file(recipe_name)?;
    validate_recipe_template_from_file(&recipe_file).map_err(|err| {
        anyhow::anyhow!(
            "{} recipe file is invalid: {}",
            style("✗").red().bold(),
            err
        )
    })?;
    println!("{} recipe file is valid", style("✓").green().bold());
    Ok(())
}

pub fn handle_explain(recipe_name: &str, params: &[(String, String)]) -> Result<()> {
    let rf = read_recipe_file(recipe_name)?;
    let recipe = validate_recipe_template_from_file(&rf)?;
    println!("Title: {}", recipe.title);
    println!("Description: {}", recipe.description);
    if let Some(prompt) = &recipe.prompt {
        println!("Prompt: {}", prompt);
    }
    if !params.is_empty() {
        println!("\nParameters provided:");
        for (k, v) in params {
            println!("  {} = {}", k, v);
        }
    }
    Ok(())
}

pub fn handle_render(recipe_name: &str, params: &[(String, String)]) -> Result<()> {
    let rf = read_recipe_file(recipe_name)?;
    let recipe = validate_recipe_template_from_file(&rf)?;
    match serde_yaml::to_string(&recipe) {
        Ok(yaml) => {
            if !params.is_empty() {
                println!("# Parameters:");
                for (k, v) in params {
                    println!("#   {} = {}", k, v);
                }
                println!();
            }
            println!("{}", yaml);
            Ok(())
        }
        Err(e) => anyhow::bail!("Failed to serialize recipe: {}", e),
    }
}

pub fn handle_deeplink(recipe_name: &str, params: &[(String, String)]) -> Result<()> {
    let params_map: HashMap<String, String> = params.iter().cloned().collect();
    match generate_deeplink(recipe_name, params_map) {
        Ok((deeplink_url, recipe)) => {
            println!(
                "{} Generated deeplink for: {}",
                style("✓").green().bold(),
                recipe.title
            );
            println!("{}", deeplink_url);
            Ok(())
        }
        Err(err) => {
            println!(
                "{} Failed to encode recipe: {}",
                style("✗").red().bold(),
                err
            );
            Err(err)
        }
    }
}

pub fn handle_open(recipe_name: &str, params: &[(String, String)]) -> Result<()> {
    handle_open_with(
        recipe_name,
        params,
        |url| open::that(url),
        &mut std::io::stdout(),
    )
}

fn handle_open_with<F, W>(
    recipe_name: &str,
    params: &[(String, String)],
    opener: F,
    out: &mut W,
) -> Result<()>
where
    F: FnOnce(&str) -> std::io::Result<()>,
    W: std::io::Write,
{
    let params_map: HashMap<String, String> = params.iter().cloned().collect();
    match generate_deeplink(recipe_name, params_map) {
        Ok((deeplink_url, recipe)) => match opener(&deeplink_url) {
            Ok(_) => {
                writeln!(
                    out,
                    "{} Opened recipe '{}' in Goose Desktop",
                    style("✓").green().bold(),
                    recipe.title
                )?;
                Ok(())
            }
            Err(err) => {
                writeln!(
                    out,
                    "{} Failed to open recipe in Goose Desktop: {}",
                    style("✗").red().bold(),
                    err
                )?;
                writeln!(out, "Generated deeplink: {}", deeplink_url)?;
                writeln!(out, "You can manually copy and open the URL above, or ensure Goose Desktop is installed.")?;
                Err(anyhow::anyhow!("Failed to open recipe: {}", err))
            }
        },
        Err(err) => {
            writeln!(
                out,
                "{} Failed to encode recipe: {}",
                style("✗").red().bold(),
                err
            )?;
            Err(err)
        }
    }
}

pub fn handle_list(_format: &str, _verbose: bool) -> Result<()> {
    // TODO: Integrate recipe listing from goose-cli's recipes::search_recipe::list_available_recipes
    // and recipes::github_recipe::RecipeSource once available in goose-tui
    anyhow::bail!("recipe list not yet supported in goose-tui - TODO: integrate recipe search")
}

/// Load a recipe file by path.
fn load_recipe_file(recipe_name: &str) -> Result<String> {
    let path = std::path::Path::new(recipe_name);
    if path.exists() {
        Ok(recipe_name.to_string())
    } else {
        anyhow::bail!(
            "Recipe file not found: {}. Provide a direct file path.",
            recipe_name
        )
    }
}

fn generate_deeplink(
    recipe_name: &str,
    params: HashMap<String, String>,
) -> Result<(String, goose::recipe::Recipe)> {
    let _recipe_path = load_recipe_file(recipe_name)?;
    let rf = read_recipe_file(recipe_name)?;
    let recipe = validate_recipe_template_from_file(&rf)?;
    match recipe_deeplink::encode(&recipe) {
        Ok(encoded) => {
            let mut full_url = format!("goose://recipe?config={}", encoded);

            for (key, value) in params {
                let encoded_key = urlencoding::encode(&key);
                let encoded_value = urlencoding::encode(&value);
                full_url.push_str(&format!("&{}={}", encoded_key, encoded_value));
            }

            Ok((full_url, recipe))
        }
        Err(err) => Err(anyhow::anyhow!("Failed to encode recipe: {}", err)),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::TempDir;

    fn create_test_recipe_file(dir: &TempDir, filename: &str, content: &str) -> String {
        let file_path = dir.path().join(filename);
        fs::write(&file_path, content).expect("Failed to write test recipe file");
        file_path.to_string_lossy().into_owned()
    }

    const VALID_RECIPE_CONTENT: &str = r#"
title: "Test Recipe with Valid JSON Schema"
description: "A test recipe with valid JSON schema"
prompt: "Test prompt content"
instructions: "Test instructions"
response:
  json_schema:
    type: object
    properties:
      result:
        type: string
        description: "The result"
      count:
        type: number
        description: "A count value"
    required:
      - result
"#;

    const INVALID_RECIPE_CONTENT: &str = r#"
title: "Test Recipe"
description: "A test recipe for deeplink generation"
prompt: "Test prompt content {{ name }}"
instructions: "Test instructions"
"#;

    #[test]
    fn test_handle_deeplink_valid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let result = handle_deeplink(&recipe_path, &[]);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_deeplink_invalid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", INVALID_RECIPE_CONTENT);
        let result = handle_deeplink(&recipe_path, &[]);
        assert!(result.is_err());
    }

    fn run_handle_open(
        recipe_path: &str,
        params: &[(String, String)],
        opener_result: std::io::Result<()>,
    ) -> (Result<()>, String, String) {
        let captured_url = std::cell::RefCell::new(String::new());
        let mut out = Vec::new();
        let result = handle_open_with(
            recipe_path,
            params,
            |url| {
                *captured_url.borrow_mut() = url.to_string();
                opener_result
            },
            &mut out,
        );
        let output = String::from_utf8(out).unwrap();
        (result, captured_url.into_inner(), output)
    }

    #[test]
    fn test_handle_open_recipe() {
        let temp_dir = TempDir::new().unwrap();
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let (expected_url, _) = generate_deeplink(&recipe_path, HashMap::new()).unwrap();
        let (result, captured_url, _) = run_handle_open(&recipe_path, &[], Ok(()));

        assert!(result.is_ok());
        assert_eq!(captured_url, expected_url);
    }

    #[test]
    fn test_handle_open_with_parameters() {
        let temp_dir = TempDir::new().unwrap();
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let params = vec![
            ("name".to_string(), "Alice".to_string()),
            ("role".to_string(), "developer".to_string()),
        ];
        let (result, captured_url, _) = run_handle_open(&recipe_path, &params, Ok(()));

        assert!(result.is_ok());
        assert!(captured_url.contains("&name=Alice"));
        assert!(captured_url.contains("&role=developer"));
    }

    #[test]
    fn test_handle_open_opener_fails() {
        let temp_dir = TempDir::new().unwrap();
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let opener_err = std::io::Error::new(std::io::ErrorKind::NotFound, "desktop not found");
        let (result, _, output) = run_handle_open(&recipe_path, &[], Err(opener_err));

        assert!(result.is_err());
        assert!(output.contains("Failed to open recipe in Goose Desktop"));
        assert!(output.contains("desktop not found"));
    }

    #[test]
    fn test_handle_open_invalid_recipe() {
        let temp_dir = TempDir::new().unwrap();
        let recipe_path =
            create_test_recipe_file(&temp_dir, "invalid.yaml", INVALID_RECIPE_CONTENT);

        let (result, _, output) = run_handle_open(&recipe_path, &[], Ok(()));

        assert!(result.is_err());
        assert!(output.contains("Failed to encode recipe"));
    }

    #[test]
    fn test_handle_validation_valid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let result = handle_validate(&recipe_path);
        assert!(result.is_ok());
    }

    #[test]
    fn test_handle_validation_invalid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", INVALID_RECIPE_CONTENT);
        let result = handle_validate(&recipe_path);
        assert!(result.is_err());
    }

    #[test]
    fn test_generate_deeplink_valid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let result = generate_deeplink(&recipe_path, HashMap::new());
        assert!(result.is_ok());
        let (url, recipe) = result.unwrap();
        assert!(url.starts_with("goose://recipe?config="));
        assert_eq!(recipe.title, "Test Recipe with Valid JSON Schema");
    }

    #[test]
    fn test_generate_deeplink_with_parameters() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let mut params = HashMap::new();
        params.insert("name".to_string(), "Alice".to_string());

        let result = generate_deeplink(&recipe_path, params);
        assert!(result.is_ok());
        let (url, _) = result.unwrap();
        assert!(url.contains("&name=Alice"));
    }

    #[test]
    fn test_generate_deeplink_invalid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", INVALID_RECIPE_CONTENT);

        let result = generate_deeplink(&recipe_path, HashMap::new());
        assert!(result.is_err());
    }
}
