use anyhow::Result;
use comfy_table::{presets::UTF8_FULL_CONDENSED, ContentArrangement, Table};
use console::style;
use goose::recipe::validate_recipe::validate_recipe_template_from_file;
use std::collections::HashMap;

use crate::recipes::github_recipe::{RecipeInfo, RecipeSource};
use crate::recipes::search_recipe::{list_available_recipes, load_recipe_file};
use goose::recipe_deeplink;

pub fn handle_validate(recipe_name: &str) -> Result<()> {
    // Load and validate the recipe file
    let recipe_file = load_recipe_file(recipe_name)?;
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

pub fn handle_deeplink(recipe_name: &str, params: &[String]) -> Result<String> {
    let params_map = parse_params(params)?;
    match generate_deeplink(recipe_name, params_map) {
        Ok((deeplink_url, recipe)) => {
            println!(
                "{} Generated deeplink for: {}",
                style("✓").green().bold(),
                recipe.title
            );
            println!("{}", deeplink_url);
            Ok(deeplink_url)
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

pub fn handle_open(recipe_name: &str, params: &[String]) -> Result<()> {
    handle_open_with(
        recipe_name,
        params,
        |url| open::that(url),
        &mut std::io::stdout(),
    )
}

fn handle_open_with<F, W>(
    recipe_name: &str,
    params: &[String],
    opener: F,
    out: &mut W,
) -> Result<()>
where
    F: FnOnce(&str) -> std::io::Result<()>,
    W: std::io::Write,
{
    let params_map = parse_params(params)?;
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

pub fn handle_list(format: &str, verbose: bool) -> Result<()> {
    let mut recipes = match list_available_recipes() {
        Ok(recipes) => recipes,
        Err(e) => {
            return Err(anyhow::anyhow!("Failed to list recipes: {}", e));
        }
    };

    match format {
        "json" => {
            // Unchanged: flat array, so existing automation/scripts don't break.
            println!("{}", serde_json::to_string(&recipes)?);
        }
        _ => {
            if recipes.is_empty() {
                println!("No recipes found");
                return Ok(());
            }

            recipes.sort_by(|a, b| a.name.cmp(&b.name));

            // Group by name. A recipe found via more than one search path
            // (GOOSE_RECIPE_PATH + cwd, most commonly) currently prints as
            // flat, indistinguishable duplicate rows with no indication
            // that one file silently shadows the other. Group instead of
            // hiding the collision.
            let mut groups: Vec<(String, Vec<&RecipeInfo>)> = Vec::new();
            for recipe in &recipes {
                match groups.last_mut() {
                    Some((name, entries)) if *name == recipe.name => entries.push(recipe),
                    _ => groups.push((recipe.name.clone(), vec![recipe])),
                }
            }

            if verbose {
                for (name, entries) in &groups {
                    println!("{}", style(name).bold());
                    println!("  {}", describe(entries[0]));
                    if entries.len() > 1 {
                        println!(
                            "  {} found in {} locations (later paths override earlier ones):",
                            style("⚠").yellow(),
                            entries.len()
                        );
                        for (i, entry) in entries.iter().enumerate() {
                            println!("    {}. {}", i + 1, source_path(entry));
                        }
                    } else {
                        println!("  {}", source_path(entries[0]));
                    }
                    println!();
                }
                return Ok(());
            }

            let mut table = Table::new();
            table
                .load_preset(UTF8_FULL_CONDENSED)
                .set_content_arrangement(ContentArrangement::Dynamic)
                .set_header(vec!["Name", "Description", "Location"]);

            for (name, entries) in &groups {
                let location = if entries.len() > 1 {
                    format!("⚠ {} locations", entries.len())
                } else {
                    source_path(entries[0])
                };
                table.add_row(vec![name.clone(), describe(entries[0]), location]);
            }

            println!("{table}");

            let dup_count = groups.iter().filter(|(_, entries)| entries.len() > 1).count();
            if dup_count > 0 {
                println!(
                    "\n{} {} recipe{} found in multiple locations. Run with --verbose for full paths.",
                    style("⚠").yellow(),
                    dup_count,
                    if dup_count == 1 { "" } else { "s" }
                );
            }
        }
    }
    Ok(())
}

fn describe(recipe: &RecipeInfo) -> String {
    match &recipe.description {
        Some(desc) if !desc.is_empty() => desc.clone(),
        _ => "(none)".to_string(),
    }
}

fn source_path(recipe: &RecipeInfo) -> String {
    match recipe.source {
        RecipeSource::Local => format!("local: {}", recipe.path),
        RecipeSource::GitHub => format!("github: {}", recipe.path),
    }
}

fn parse_params(params: &[String]) -> Result<HashMap<String, String>> {
    let mut params_map = HashMap::new();
    for param in params {
        let parts: Vec<&str> = param.splitn(2, '=').collect();
        if parts.len() != 2 {
            return Err(anyhow::anyhow!(
                "Invalid parameter format: '{}'. Expected format: key=value",
                param
            ));
        }
        params_map.insert(parts[0].to_string(), parts[1].to_string());
    }
    Ok(params_map)
}

fn generate_deeplink(
    recipe_name: &str,
    params: HashMap<String, String>,
) -> Result<(String, goose::recipe::Recipe)> {
    let recipe_file = load_recipe_file(recipe_name)?;
    // Load the recipe file first to validate it
    let recipe = validate_recipe_template_from_file(&recipe_file)?;
    match recipe_deeplink::encode(&recipe) {
        Ok(encoded) => {
            let mut full_url = format!("goose://recipe?config={}", encoded);

            // Append parameters as additional query parameters
            for (key, value) in params {
                // URL-encode the parameter keys and values
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
        let url = result.unwrap();
        assert!(url.starts_with("goose://recipe?config="));
        let encoded_part = url.strip_prefix("goose://recipe?config=").unwrap();
        assert!(!encoded_part.is_empty());
    }

    #[test]
    fn test_handle_deeplink_with_parameters() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let params = vec!["name=John".to_string(), "age=30".to_string()];
        let result = handle_deeplink(&recipe_path, &params);
        assert!(result.is_ok());
        let url = result.unwrap();
        assert!(url.starts_with("goose://recipe?config="));
        assert!(url.contains("&name=John"));
        assert!(url.contains("&age=30"));
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
        params: &[String],
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

        let (base_url, _) = generate_deeplink(&recipe_path, HashMap::new()).unwrap();

        let params = vec!["name=Alice".to_string(), "role=developer".to_string()];
        let (result, captured_url, _) = run_handle_open(&recipe_path, &params, Ok(()));

        assert!(result.is_ok());
        assert!(captured_url.starts_with(&base_url));
        assert!(captured_url.contains("&name=Alice"));
        assert!(captured_url.contains("&role=developer"));
    }

    #[test]
    fn test_handle_open_opener_fails() {
        let temp_dir = TempDir::new().unwrap();
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let (expected_url, _) = generate_deeplink(&recipe_path, HashMap::new()).unwrap();
        let opener_err = std::io::Error::new(std::io::ErrorKind::NotFound, "desktop not found");
        let (result, _, output) = run_handle_open(&recipe_path, &[], Err(opener_err));

        assert!(result.is_err());
        assert!(output.contains("Failed to open recipe in Goose Desktop"));
        assert!(output.contains("desktop not found"));
        assert!(output.contains(&expected_url));
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
        assert_eq!(recipe.description, "A test recipe with valid JSON schema");
        let encoded_part = url.strip_prefix("goose://recipe?config=").unwrap();
        assert!(!encoded_part.is_empty());
    }

    #[test]
    fn test_generate_deeplink_with_parameters() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", VALID_RECIPE_CONTENT);

        let mut params = HashMap::new();
        params.insert("name".to_string(), "Alice".to_string());
        params.insert("role".to_string(), "developer".to_string());

        let result = generate_deeplink(&recipe_path, params);
        assert!(result.is_ok());
        let (url, recipe) = result.unwrap();
        assert!(url.starts_with("goose://recipe?config="));
        assert!(url.contains("&name=Alice"));
        assert!(url.contains("&role=developer"));
        assert_eq!(recipe.title, "Test Recipe with Valid JSON Schema");
    }

    #[test]
    fn test_generate_deeplink_invalid_recipe() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let recipe_path =
            create_test_recipe_file(&temp_dir, "test_recipe.yaml", INVALID_RECIPE_CONTENT);

        let result = generate_deeplink(&recipe_path, HashMap::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_params_basic() {
        let params = vec!["name=John".to_string(), "age=30".to_string()];
        let result = parse_params(&params);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.get("name"), Some(&"John".to_string()));
        assert_eq!(map.get("age"), Some(&"30".to_string()));
    }

    #[test]
    fn test_parse_params_with_equals_in_value() {
        let params = vec!["key=value=with=equals".to_string()];
        let result = parse_params(&params);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.get("key"), Some(&"value=with=equals".to_string()));
    }

    #[test]
    fn test_parse_params_empty_value() {
        let params = vec!["key=".to_string()];
        let result = parse_params(&params);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(map.get("key"), Some(&"".to_string()));
    }

    #[test]
    fn test_parse_params_no_equals() {
        let params = vec!["invalid".to_string()];
        let result = parse_params(&params);
        assert!(result.is_err());
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Invalid parameter format"));
    }

    #[test]
    fn test_parse_params_empty_key() {
        let params = vec!["=value".to_string()];
        let result = parse_params(&params);
        assert!(result.is_ok());
        let map = result.unwrap();
        // Empty key is technically valid according to current implementation
        assert_eq!(map.get(""), Some(&"value".to_string()));
    }

    #[test]
    fn test_parse_params_special_characters() {
        let params = vec![
            "url=https://example.com/path?query=test".to_string(),
            "message=Hello World!".to_string(),
            "email=user@example.com".to_string(),
        ];
        let result = parse_params(&params);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert_eq!(
            map.get("url"),
            Some(&"https://example.com/path?query=test".to_string())
        );
        assert_eq!(map.get("message"), Some(&"Hello World!".to_string()));
        assert_eq!(map.get("email"), Some(&"user@example.com".to_string()));
    }

    #[test]
    fn test_parse_params_empty_list() {
        let params: Vec<String> = vec![];
        let result = parse_params(&params);
        assert!(result.is_ok());
        let map = result.unwrap();
        assert!(map.is_empty());
    }

    fn recipe_info(source: RecipeSource, path: &str, description: Option<&str>) -> RecipeInfo {
        RecipeInfo {
            name: "example".to_string(),
            source,
            path: path.to_string(),
            title: None,
            description: description.map(|d| d.to_string()),
        }
    }

    #[test]
    fn test_describe_with_description() {
        let recipe = recipe_info(RecipeSource::Local, "recipe.yaml", Some("Does a thing"));
        assert_eq!(describe(&recipe), "Does a thing");
    }

    #[test]
    fn test_describe_missing_or_empty() {
        let no_desc = recipe_info(RecipeSource::Local, "recipe.yaml", None);
        assert_eq!(describe(&no_desc), "(none)");

        let empty_desc = recipe_info(RecipeSource::Local, "recipe.yaml", Some(""));
        assert_eq!(describe(&empty_desc), "(none)");
    }

    #[test]
    fn test_source_path_local_and_github() {
        let local = recipe_info(RecipeSource::Local, "/home/user/recipe.yaml", None);
        assert_eq!(source_path(&local), "local: /home/user/recipe.yaml");

        let github = recipe_info(RecipeSource::GitHub, "org/repo/recipe.yaml", None);
        assert_eq!(source_path(&github), "github: org/repo/recipe.yaml");
    }
}
