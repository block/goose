use crate::recipes::search_recipe::load_recipe_file_with_byte_limit;
use goose::agents::extension::ExtensionConfig;
use goose::recipe::Recipe;
use regex::{NoExpand, Regex};
use std::{collections::HashSet, path::PathBuf};

const MAX_SUB_RECIPE_DEPTH: usize = 64;
const MAX_DISCOVERED_RECIPES: usize = 256;
const MAX_DISCOVERY_LOADED_BYTES: usize = 1024 * 1024;

#[derive(Clone, Copy)]
struct DiscoveryLimits {
    max_depth: usize,
    max_recipes: usize,
    max_loaded_bytes: usize,
}

const DISCOVERY_LIMITS: DiscoveryLimits = DiscoveryLimits {
    max_depth: MAX_SUB_RECIPE_DEPTH,
    max_recipes: MAX_DISCOVERED_RECIPES,
    max_loaded_bytes: MAX_DISCOVERY_LOADED_BYTES,
};

struct PendingRecipe {
    path: String,
    depth: usize,
}

/// Represents a secret requirement discovered from a recipe extension
#[derive(Debug, Clone, PartialEq)]
pub struct SecretRequirement {
    /// The environment variable name (e.g., "GITHUB_TOKEN")
    pub key: String,
    /// The name of the extension that requires this secret
    pub extension_name: String,
}

impl SecretRequirement {
    pub fn new(extension_name: String, key: String) -> Self {
        Self {
            key,
            extension_name,
        }
    }

    /// Returns a human-readable description of what this secret is for
    pub fn description(&self) -> String {
        format!("Required by {} extension", self.extension_name)
    }
}

/// Discovers all secrets required by MCP extensions in a recipe and its sub-recipes
///
/// This function scans the recipe and its bounded sub-recipe graph for extensions
/// and collects their declared env_keys, creating SecretRequirement structs for each
/// unique environment variable.
///
/// # Arguments
/// * `recipe` - The recipe to analyze for secret requirements
///
/// # Returns
/// A vector of SecretRequirement objects, deduplicated by key name
pub fn discover_recipe_secrets(recipe: &Recipe) -> Vec<SecretRequirement> {
    discover_recipe_secrets_with_limits(recipe, DISCOVERY_LIMITS)
}

/// Extract secrets from a list of extensions
fn extract_secrets_from_extensions(
    extensions: &[ExtensionConfig],
    seen_keys: &mut HashSet<String>,
) -> Vec<SecretRequirement> {
    let mut secrets = Vec::new();

    for ext in extensions {
        let (extension_name, env_keys, client_secret_key) = match ext {
            ExtensionConfig::Stdio { name, env_keys, .. } => (name, env_keys, None),
            ExtensionConfig::StreamableHttp {
                name,
                env_keys,
                client_secret_key,
                ..
            } => (name, env_keys, client_secret_key.as_ref()),
            ExtensionConfig::Builtin { name, .. } => (name, &Vec::new(), None),
            ExtensionConfig::Platform { name, .. } => (name, &Vec::new(), None),
            ExtensionConfig::Frontend { name, .. } => (name, &Vec::new(), None),
            ExtensionConfig::InlinePython { name, .. } => (name, &Vec::new(), None),
            // SSE is unsupported - skip
            ExtensionConfig::Sse { name, .. } => {
                tracing::warn!(name = %name, "SSE is unsupported, skipping");
                continue;
            }
        };

        for key in env_keys.iter().chain(client_secret_key) {
            if seen_keys.insert(key.clone()) {
                let secret_req = SecretRequirement::new(extension_name.clone(), key.clone());
                secrets.push(secret_req);
            }
        }
    }

    secrets
}

fn discover_recipe_secrets_with_limits(
    recipe: &Recipe,
    limits: DiscoveryLimits,
) -> Vec<SecretRequirement> {
    let mut secrets: Vec<SecretRequirement> = Vec::new();
    let mut seen_keys = HashSet::new();
    let mut seen_requests = HashSet::new();
    let mut seen_files = HashSet::<PathBuf>::new();
    let mut loaded_recipes = 0;
    let mut loaded_bytes: usize = 0;

    if let Some(extensions) = &recipe.extensions {
        secrets.extend(extract_secrets_from_extensions(extensions, &mut seen_keys));
    }

    let mut pending = Vec::new();
    push_sub_recipes(&mut pending, recipe, None, 1);

    while let Some(next) = pending.pop() {
        if next.depth > limits.max_depth || !seen_requests.insert(next.path.clone()) {
            continue;
        }

        if let Ok(canonical_path) = std::fs::canonicalize(&next.path) {
            if seen_files.contains(&canonical_path) {
                continue;
            }
        }

        if loaded_recipes == limits.max_recipes {
            break;
        }
        loaded_recipes += 1;

        let remaining_bytes = limits.max_loaded_bytes - loaded_bytes;
        let Ok(recipe_file) = load_recipe_file_with_byte_limit(&next.path, remaining_bytes) else {
            continue;
        };
        let Some(next_loaded_bytes) = loaded_bytes.checked_add(recipe_file.content.len()) else {
            break;
        };
        if next_loaded_bytes > limits.max_loaded_bytes {
            break;
        }
        loaded_bytes = next_loaded_bytes;

        let file_identity = recipe_file
            .file_path
            .canonicalize()
            .unwrap_or_else(|_| recipe_file.file_path.clone());
        if !seen_files.insert(file_identity) {
            continue;
        }

        let Ok(loaded_recipe) = serde_yaml::from_str::<Recipe>(&recipe_file.content) else {
            continue;
        };
        if let Some(extensions) = &loaded_recipe.extensions {
            secrets.extend(extract_secrets_from_extensions(extensions, &mut seen_keys));
        }
        if next.depth < limits.max_depth {
            push_sub_recipes(
                &mut pending,
                &loaded_recipe,
                Some(recipe_file.parent_dir.to_string_lossy().as_ref()),
                next.depth + 1,
            );
        }
    }

    secrets
}

fn push_sub_recipes(
    pending: &mut Vec<PendingRecipe>,
    recipe: &Recipe,
    parent_dir: Option<&str>,
    depth: usize,
) {
    let Some(sub_recipes) = &recipe.sub_recipes else {
        return;
    };
    let re = Regex::new(r"\{\{\s*recipe_dir\s*\}\}").expect("valid regex");
    for sub_recipe in sub_recipes.iter().rev() {
        let path = match parent_dir {
            Some(parent_dir) => re
                .replace_all(&sub_recipe.path, NoExpand(parent_dir))
                .into_owned(),
            None => sub_recipe.path.clone(),
        };
        pending.push(PendingRecipe { path, depth });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use goose::agents::extension::{Envs, ExtensionConfig};
    use goose::recipe::{Recipe, SubRecipe};
    use std::collections::HashMap;

    fn sub_recipe(path: impl Into<String>) -> SubRecipe {
        SubRecipe {
            name: "child".to_string(),
            path: path.into(),
            values: None,
            sequential_when_repeated: false,
            description: None,
        }
    }

    fn recipe_with_secret(secret: Option<&str>, sub_recipes: Vec<SubRecipe>) -> Recipe {
        Recipe {
            version: "1.0.0".to_string(),
            title: "Test Recipe".to_string(),
            description: "Test recipe".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: secret.map(|key| {
                vec![ExtensionConfig::Stdio {
                    name: format!("{key}-extension"),
                    cmd: "test".to_string(),
                    args: vec![],
                    envs: Envs::new(HashMap::new()),
                    env_keys: vec![key.to_string()],
                    timeout: None,
                    cwd: None,
                    description: "test".to_string(),
                    bundled: None,
                    available_tools: Vec::new(),
                }]
            }),
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: (!sub_recipes.is_empty()).then_some(sub_recipes),
            retry: None,
        }
    }

    fn write_recipe(path: &std::path::Path, recipe: &Recipe) -> usize {
        let content = serde_yaml::to_string(recipe).unwrap();
        std::fs::write(path, &content).unwrap();
        content.len()
    }

    fn create_test_recipe_with_extensions() -> Recipe {
        Recipe {
            version: "1.0.0".to_string(),
            title: "Test Recipe".to_string(),
            description: "A test recipe with MCP extensions".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: Some(vec![
                ExtensionConfig::StreamableHttp {
                    name: "github-mcp".to_string(),
                    uri: "http://localhost:8080/mcp".to_string(),
                    envs: Envs::new(HashMap::new()),
                    env_keys: vec!["GITHUB_TOKEN".to_string(), "GITHUB_API_URL".to_string()],
                    description: "github-mcp".to_string(),
                    timeout: None,
                    socket: None,
                    client_id: None,
                    client_secret_key: None,
                    scopes: vec![],
                    bundled: None,
                    available_tools: Vec::new(),
                    headers: HashMap::new(),
                },
                ExtensionConfig::Stdio {
                    name: "slack-mcp".to_string(),
                    cmd: "slack-mcp".to_string(),
                    args: vec![],
                    envs: Envs::new(HashMap::new()),
                    env_keys: vec!["SLACK_TOKEN".to_string()],
                    timeout: None,
                    cwd: None,
                    description: "slack-mcp".to_string(),
                    bundled: None,
                    available_tools: Vec::new(),
                },
                ExtensionConfig::Builtin {
                    name: "builtin-ext".to_string(),
                    display_name: None,
                    description: "builtin-ext".to_string(),
                    timeout: None,
                    bundled: None,
                    available_tools: Vec::new(),
                },
            ]),
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
            retry: None,
        }
    }

    #[test]
    fn test_discover_recipe_secrets() {
        let recipe = create_test_recipe_with_extensions();
        let secrets = discover_recipe_secrets(&recipe);

        assert_eq!(secrets.len(), 3);

        let github_token = secrets.iter().find(|s| s.key == "GITHUB_TOKEN").unwrap();
        assert_eq!(github_token.key, "GITHUB_TOKEN");
        assert_eq!(github_token.extension_name, "github-mcp");
        assert_eq!(
            github_token.description(),
            "Required by github-mcp extension"
        );

        let github_api = secrets.iter().find(|s| s.key == "GITHUB_API_URL").unwrap();
        assert_eq!(github_api.key, "GITHUB_API_URL");
        assert_eq!(github_api.extension_name, "github-mcp");

        let slack_token = secrets.iter().find(|s| s.key == "SLACK_TOKEN").unwrap();
        assert_eq!(slack_token.key, "SLACK_TOKEN");
        assert_eq!(slack_token.extension_name, "slack-mcp");
    }

    #[test]
    fn test_discover_recipe_secrets_empty_recipe() {
        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "Empty Recipe".to_string(),
            description: "A recipe with no extensions".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
            retry: None,
        };

        let secrets = discover_recipe_secrets(&recipe);
        assert_eq!(secrets.len(), 0);
    }

    #[test]
    fn test_discover_recipe_secrets_deduplication() {
        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "Test Recipe".to_string(),
            description: "A test recipe with duplicate secrets".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: Some(vec![
                ExtensionConfig::StreamableHttp {
                    name: "service-a".to_string(),
                    uri: "http://localhost:8080/mcp".to_string(),
                    envs: Envs::new(HashMap::new()),
                    env_keys: vec!["API_KEY".to_string()],
                    description: "service-a".to_string(),
                    timeout: None,
                    socket: None,
                    client_id: None,
                    client_secret_key: None,
                    scopes: vec![],
                    bundled: None,
                    available_tools: Vec::new(),
                    headers: HashMap::new(),
                },
                ExtensionConfig::Stdio {
                    name: "service-b".to_string(),
                    cmd: "service-b".to_string(),
                    args: vec![],
                    envs: Envs::new(HashMap::new()),
                    env_keys: vec!["API_KEY".to_string()], // Same original key, different extension
                    timeout: None,
                    cwd: None,
                    description: "service-b".to_string(),
                    bundled: None,
                    available_tools: Vec::new(),
                },
            ]),
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
            retry: None,
        };

        let secrets = discover_recipe_secrets(&recipe);
        assert_eq!(secrets.len(), 1);

        let api_key = secrets.iter().find(|s| s.key == "API_KEY").unwrap();
        assert_eq!(api_key.key, "API_KEY");
        assert!(api_key.extension_name == "service-a" || api_key.extension_name == "service-b");
    }

    #[test]
    fn test_discover_recipe_secrets_includes_client_secret_key() {
        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "OAuth Recipe".to_string(),
            description: "A recipe with a pre-registered OAuth client".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: Some(vec![ExtensionConfig::StreamableHttp {
                name: "oauth-ext".to_string(),
                uri: "http://localhost:8080/mcp".to_string(),
                envs: Envs::new(HashMap::new()),
                env_keys: vec!["API_TOKEN".to_string()],
                description: "oauth-ext".to_string(),
                timeout: None,
                socket: None,
                client_id: Some("registered-client".to_string()),
                client_secret_key: Some("OAUTH_CLIENT_SECRET".to_string()),
                scopes: vec![],
                bundled: None,
                available_tools: Vec::new(),
                headers: HashMap::new(),
            }]),
            sub_recipes: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            retry: None,
        };

        let secrets = discover_recipe_secrets(&recipe);
        let keys: Vec<&str> = secrets.iter().map(|s| s.key.as_str()).collect();

        assert!(keys.contains(&"API_TOKEN"));
        assert!(keys.contains(&"OAUTH_CLIENT_SECRET"));
        let client_secret = secrets
            .iter()
            .find(|s| s.key == "OAUTH_CLIENT_SECRET")
            .unwrap();
        assert_eq!(client_secret.extension_name, "oauth-ext");
    }

    #[test]
    fn test_secret_requirement_creation() {
        let req = SecretRequirement::new("test-ext".to_string(), "API_TOKEN".to_string());

        assert_eq!(req.key, "API_TOKEN");
        assert_eq!(req.extension_name, "test-ext");
        assert_eq!(req.description(), "Required by test-ext extension");
    }

    #[test]
    fn test_discover_recipe_secrets_with_sub_recipes() {
        let recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "Parent Recipe".to_string(),
            description: "A recipe with sub-recipes".to_string(),
            instructions: Some("Test instructions".to_string()),
            prompt: None,
            extensions: Some(vec![ExtensionConfig::StreamableHttp {
                name: "parent-ext".to_string(),
                uri: "http://localhost:8080/mcp".to_string(),
                envs: Envs::new(HashMap::new()),
                env_keys: vec!["PARENT_TOKEN".to_string()],
                description: "parent-ext".to_string(),
                timeout: None,
                socket: None,
                client_id: None,
                client_secret_key: None,
                scopes: vec![],
                bundled: None,
                available_tools: Vec::new(),
                headers: HashMap::new(),
            }]),
            sub_recipes: Some(vec![SubRecipe {
                name: "child-recipe".to_string(),
                path: "path/to/child.yaml".to_string(),
                values: None,
                sequential_when_repeated: false,
                description: None,
            }]),
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            retry: None,
        };

        let secrets = discover_recipe_secrets(&recipe);

        assert_eq!(secrets.len(), 1);

        let parent_secret = secrets.iter().find(|s| s.key == "PARENT_TOKEN").unwrap();
        assert_eq!(parent_secret.extension_name, "parent-ext");
    }

    #[test]
    fn nested_sub_recipe_secrets_preserve_depth_first_order() {
        let temp_dir = tempfile::tempdir().unwrap();
        let child_path = temp_dir.path().join("child.yaml");
        let grandchild_path = temp_dir.path().join("grandchild.yaml");
        write_recipe(
            &grandchild_path,
            &recipe_with_secret(Some("GRANDCHILD_TOKEN"), vec![]),
        );
        write_recipe(
            &child_path,
            &recipe_with_secret(
                Some("CHILD_TOKEN"),
                vec![sub_recipe("{{ recipe_dir }}/grandchild.yaml")],
            ),
        );
        let root = recipe_with_secret(
            Some("ROOT_TOKEN"),
            vec![sub_recipe(child_path.to_string_lossy())],
        );

        let keys: Vec<_> = discover_recipe_secrets(&root)
            .into_iter()
            .map(|secret| secret.key)
            .collect();

        assert_eq!(keys, ["ROOT_TOKEN", "CHILD_TOKEN", "GRANDCHILD_TOKEN"]);
    }

    #[test]
    fn aliases_and_cycles_do_not_consume_the_recipe_budget() {
        let temp_dir = tempfile::tempdir().unwrap();
        let child_path = temp_dir.path().join("child.yaml");
        let sibling_path = temp_dir.path().join("sibling.yaml");
        write_recipe(
            &child_path,
            &recipe_with_secret(
                Some("CHILD_TOKEN"),
                vec![sub_recipe("{{ recipe_dir }}/child.yaml")],
            ),
        );
        write_recipe(
            &sibling_path,
            &recipe_with_secret(Some("SIBLING_TOKEN"), vec![]),
        );
        let alias_path = temp_dir.path().join(".").join("child.yaml");
        let root = recipe_with_secret(
            None,
            vec![
                sub_recipe(child_path.to_string_lossy()),
                sub_recipe(alias_path.to_string_lossy()),
                sub_recipe(sibling_path.to_string_lossy()),
            ],
        );

        let keys: Vec<_> = discover_recipe_secrets_with_limits(
            &root,
            DiscoveryLimits {
                max_depth: 8,
                max_recipes: 2,
                max_loaded_bytes: MAX_DISCOVERY_LOADED_BYTES,
            },
        )
        .into_iter()
        .map(|secret| secret.key)
        .collect();

        assert_eq!(keys, ["CHILD_TOKEN", "SIBLING_TOKEN"]);
    }

    #[test]
    fn discovery_enforces_depth_count_and_aggregate_byte_boundaries() {
        let temp_dir = tempfile::tempdir().unwrap();
        let first_path = temp_dir.path().join("first.yaml");
        let second_path = temp_dir.path().join("second.yaml");
        let third_path = temp_dir.path().join("third.yaml");
        let third_size = write_recipe(
            &third_path,
            &recipe_with_secret(Some("THIRD_TOKEN"), vec![]),
        );
        let second_size = write_recipe(
            &second_path,
            &recipe_with_secret(
                Some("SECOND_TOKEN"),
                vec![sub_recipe(third_path.to_string_lossy())],
            ),
        );
        let first_size = write_recipe(
            &first_path,
            &recipe_with_secret(
                Some("FIRST_TOKEN"),
                vec![sub_recipe(second_path.to_string_lossy())],
            ),
        );
        let root = recipe_with_secret(None, vec![sub_recipe(first_path.to_string_lossy())]);

        let depth_limited = discover_recipe_secrets_with_limits(
            &root,
            DiscoveryLimits {
                max_depth: 2,
                max_recipes: 3,
                max_loaded_bytes: first_size + second_size + third_size,
            },
        );
        assert_eq!(
            depth_limited
                .iter()
                .map(|secret| secret.key.as_str())
                .collect::<Vec<_>>(),
            ["FIRST_TOKEN", "SECOND_TOKEN"]
        );

        let count_limited = discover_recipe_secrets_with_limits(
            &root,
            DiscoveryLimits {
                max_depth: 3,
                max_recipes: 2,
                max_loaded_bytes: first_size + second_size + third_size,
            },
        );
        assert_eq!(
            count_limited
                .iter()
                .map(|secret| secret.key.as_str())
                .collect::<Vec<_>>(),
            ["FIRST_TOKEN", "SECOND_TOKEN"]
        );

        let exact_bytes = discover_recipe_secrets_with_limits(
            &root,
            DiscoveryLimits {
                max_depth: 3,
                max_recipes: 3,
                max_loaded_bytes: first_size + second_size + third_size,
            },
        );
        assert_eq!(exact_bytes.len(), 3);

        let below_boundary = discover_recipe_secrets_with_limits(
            &root,
            DiscoveryLimits {
                max_depth: 3,
                max_recipes: 3,
                max_loaded_bytes: first_size + second_size + third_size - 1,
            },
        );
        assert_eq!(below_boundary.len(), 2);
    }

    #[test]
    fn discovery_rejects_an_oversized_child_before_loading_its_content() {
        let temp_dir = tempfile::tempdir().unwrap();
        let oversized_path = temp_dir.path().join("oversized.yaml");
        let small_path = temp_dir.path().join("small.yaml");
        let small_size = write_recipe(
            &small_path,
            &recipe_with_secret(Some("SMALL_TOKEN"), vec![]),
        );
        let mut oversized = recipe_with_secret(Some("OVERSIZED_TOKEN"), vec![]);
        oversized.instructions = Some("x".repeat(small_size));
        assert!(write_recipe(&oversized_path, &oversized) > small_size);
        let root = recipe_with_secret(
            None,
            vec![
                sub_recipe(oversized_path.to_string_lossy()),
                sub_recipe(small_path.to_string_lossy()),
            ],
        );

        let secrets = discover_recipe_secrets_with_limits(
            &root,
            DiscoveryLimits {
                max_depth: 1,
                max_recipes: 2,
                max_loaded_bytes: small_size,
            },
        );

        assert_eq!(
            secrets
                .iter()
                .map(|secret| secret.key.as_str())
                .collect::<Vec<_>>(),
            ["SMALL_TOKEN"]
        );
    }

    #[test]
    fn very_deep_sub_recipe_chain_is_bounded_without_recursion() {
        let temp_dir = tempfile::tempdir().unwrap();
        let depth = 2048;
        for index in 0..depth {
            let next = (index + 1 < depth).then(|| {
                sub_recipe(
                    temp_dir
                        .path()
                        .join(format!("{}.yaml", index + 1))
                        .to_string_lossy(),
                )
            });
            write_recipe(
                &temp_dir.path().join(format!("{index}.yaml")),
                &recipe_with_secret(Some(&format!("TOKEN_{index}")), next.into_iter().collect()),
            );
        }
        let root = recipe_with_secret(
            None,
            vec![sub_recipe(temp_dir.path().join("0.yaml").to_string_lossy())],
        );

        let secrets = discover_recipe_secrets(&root);

        assert_eq!(secrets.len(), MAX_SUB_RECIPE_DEPTH);
        assert_eq!(secrets.first().unwrap().key, "TOKEN_0");
        assert_eq!(
            secrets.last().unwrap().key,
            format!("TOKEN_{}", MAX_SUB_RECIPE_DEPTH - 1)
        );
    }
}
