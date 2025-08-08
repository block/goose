use anyhow::Result;
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;

use crate::agents::extension::ExtensionConfig;
use crate::agents::types::RetryConfig;
use serde::de::Deserializer;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod build_recipe;
pub mod read_recipe_file_content;
pub mod template_recipe;

pub const BUILT_IN_RECIPE_DIR_PARAM: &str = "recipe_dir";

fn default_version() -> String {
    "1.0.0".to_string()
}

/// A Recipe represents a personalized, user-generated agent configuration that defines
/// specific behaviors and capabilities within the Goose system.
///
/// # Fields
///
/// ## Required Fields
/// * `version` - Semantic version of the Recipe file format (defaults to "1.0.0")
/// * `title` - Short, descriptive name of the Recipe
/// * `description` - Detailed description explaining the Recipe's purpose and functionality
/// * `Instructions` - Instructions that defines the Recipe's behavior
///
/// ## Optional Fields
/// * `prompt` - the initial prompt to the session to start with
/// * `extensions` - List of extension configurations required by the Recipe
/// * `context` - Supplementary context information for the Recipe
/// * `activities` - Activity labels that appear when loading the Recipe
/// * `author` - Information about the Recipe's creator and metadata
/// * `parameters` - Additional parameters for the Recipe
/// * `response` - Response configuration including JSON schema validation
/// * `retry` - Retry configuration for automated validation and recovery
/// # Example
///
///
/// use goose::recipe::Recipe;
///
/// // Using the builder pattern
/// let recipe = Recipe::builder()
///     .title("Example Agent")
///     .description("An example Recipe configuration")
///     .instructions("Act as a helpful assistant")
///     .build()
///     .expect("Missing required fields");
///
/// // Or using struct initialization
/// let recipe = Recipe {
///     version: "1.0.0".to_string(),
///     title: "Example Agent".to_string(),
///     description: "An example Recipe configuration".to_string(),
///     instructions: Some("Act as a helpful assistant".to_string()),
///     prompt: None,
///     extensions: None,
///     context: None,
///     activities: None,
///     author: None,
///     settings: None,
///     parameters: None,
///     response: None,
///     sub_recipes: None,
///     retry: None,
/// };
///
#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Recipe {
    // Required fields
    #[serde(default = "default_version")]
    pub version: String, // version of the file format, sem ver

    pub title: String, // short title of the recipe

    pub description: String, // a longer description of the recipe

    // Optional fields
    // Note: at least one of instructions or prompt need to be set
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instructions: Option<String>, // the instructions for the model

    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompt: Option<String>, // the prompt to start the session with

    #[serde(skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Vec<ExtensionConfig>>, // a list of extensions to enable

    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<Vec<String>>, // any additional context

    #[serde(skip_serializing_if = "Option::is_none")]
    pub settings: Option<Settings>, // settings for the recipe

    #[serde(skip_serializing_if = "Option::is_none")]
    pub activities: Option<Vec<String>>, // the activity pills that show up when loading the

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>, // any additional author information

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<RecipeParameter>>, // any additional parameters for the recipe

    #[serde(skip_serializing_if = "Option::is_none")]
    pub response: Option<Response>, // response configuration including JSON schema

    #[serde(skip_serializing_if = "Option::is_none")]
    pub sub_recipes: Option<Vec<SubRecipe>>, // sub-recipes for the recipe

    #[serde(skip_serializing_if = "Option::is_none")]
    pub retry: Option<RetryConfig>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Author {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>, // creator/contact information of the recipe

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>, // any additional metadata for the author
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Settings {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub goose_provider: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub goose_model: Option<String>,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct Response {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub json_schema: Option<serde_json::Value>,
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct SubRecipe {
    pub name: String,
    pub path: String,
    #[serde(default, deserialize_with = "deserialize_value_map_as_string")]
    pub values: Option<HashMap<String, String>>,
    #[serde(default)]
    pub sequential_when_repeated: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

fn deserialize_value_map_as_string<'de, D>(
    deserializer: D,
) -> Result<Option<HashMap<String, String>>, D::Error>
where
    D: Deserializer<'de>,
{
    // First, try to deserialize a map of values
    let opt_raw: Option<HashMap<String, Value>> = Option::deserialize(deserializer)?;

    match opt_raw {
        Some(raw_map) => {
            let mut result = HashMap::new();
            for (k, v) in raw_map {
                let s = match v {
                    Value::String(s) => s,
                    _ => serde_json::to_string(&v).map_err(serde::de::Error::custom)?,
                };
                result.insert(k, s);
            }
            Ok(Some(result))
        }
        None => Ok(None),
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipeParameterRequirement {
    Required,
    Optional,
    UserPrompt,
}

impl fmt::Display for RecipeParameterRequirement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).unwrap().trim_matches('"')
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum RecipeParameterInputType {
    String,
    Number,
    Boolean,
    Date,
    File,
    Select,
}

impl fmt::Display for RecipeParameterInputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}",
            serde_json::to_string(self).unwrap().trim_matches('"')
        )
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, ToSchema)]
pub struct RecipeParameter {
    pub key: String,
    pub input_type: RecipeParameterInputType,
    pub requirement: RecipeParameterRequirement,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<String>>,
}

/// Builder for creating Recipe instances
pub struct RecipeBuilder {
    // Required fields with default values
    version: String,
    title: Option<String>,
    description: Option<String>,
    instructions: Option<String>,

    // Optional fields
    prompt: Option<String>,
    extensions: Option<Vec<ExtensionConfig>>,
    context: Option<Vec<String>>,
    settings: Option<Settings>,
    activities: Option<Vec<String>>,
    author: Option<Author>,
    parameters: Option<Vec<RecipeParameter>>,
    response: Option<Response>,
    sub_recipes: Option<Vec<SubRecipe>>,
    retry: Option<RetryConfig>,
}

impl Recipe {
    /// Creates a new RecipeBuilder to construct a Recipe instance
    ///
    /// # Example
    ///
    ///
    /// use goose::recipe::Recipe;
    ///
    /// let recipe = Recipe::builder()
    ///     .title("My Recipe")
    ///     .description("A helpful assistant")
    ///     .instructions("Act as a helpful assistant")
    ///     .build()
    ///     .expect("Failed to build Recipe: missing required fields");
    ///
    pub fn builder() -> RecipeBuilder {
        RecipeBuilder {
            version: default_version(),
            title: None,
            description: None,
            instructions: None,
            prompt: None,
            extensions: None,
            context: None,
            settings: None,
            activities: None,
            author: None,
            parameters: None,
            response: None,
            sub_recipes: None,
            retry: None,
        }
    }
    pub fn from_content(content: &str) -> Result<Self> {
        let recipe: Recipe =
            if let Ok(json_value) = serde_json::from_str::<serde_json::Value>(content) {
                if let Some(nested_recipe) = json_value.get("recipe") {
                    serde_json::from_value(nested_recipe.clone())?
                } else {
                    serde_json::from_str(content)?
                }
            } else if let Ok(yaml_value) = serde_yaml::from_str::<serde_yaml::Value>(content) {
                if let Some(nested_recipe) = yaml_value.get("recipe") {
                    serde_yaml::from_value(nested_recipe.clone())?
                } else {
                    serde_yaml::from_str(content)?
                }
            } else {
                return Err(anyhow::anyhow!(
                    "Unsupported format. Expected JSON or YAML."
                ));
            };

        if let Some(ref retry_config) = recipe.retry {
            if let Err(validation_error) = retry_config.validate() {
                return Err(anyhow::anyhow!(
                    "Invalid retry configuration: {}",
                    validation_error
                ));
            }
        }

        Ok(recipe)
    }
}

impl RecipeBuilder {
    /// Sets the version of the Recipe
    pub fn version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Sets the title of the Recipe (required)
    pub fn title(mut self, title: impl Into<String>) -> Self {
        self.title = Some(title.into());
        self
    }

    /// Sets the description of the Recipe (required)
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the instructions for the Recipe (required)
    pub fn instructions(mut self, instructions: impl Into<String>) -> Self {
        self.instructions = Some(instructions.into());
        self
    }

    pub fn prompt(mut self, prompt: impl Into<String>) -> Self {
        self.prompt = Some(prompt.into());
        self
    }

    /// Sets the extensions for the Recipe
    pub fn extensions(mut self, extensions: Vec<ExtensionConfig>) -> Self {
        self.extensions = Some(extensions);
        self
    }

    /// Sets the context for the Recipe
    pub fn context(mut self, context: Vec<String>) -> Self {
        self.context = Some(context);
        self
    }

    pub fn settings(mut self, settings: Settings) -> Self {
        self.settings = Some(settings);
        self
    }

    /// Sets the activities for the Recipe
    pub fn activities(mut self, activities: Vec<String>) -> Self {
        self.activities = Some(activities);
        self
    }

    /// Sets the author information for the Recipe
    pub fn author(mut self, author: Author) -> Self {
        self.author = Some(author);
        self
    }

    /// Sets the parameters for the Recipe
    pub fn parameters(mut self, parameters: Vec<RecipeParameter>) -> Self {
        self.parameters = Some(parameters);
        self
    }

    pub fn response(mut self, response: Response) -> Self {
        self.response = Some(response);
        self
    }

    pub fn sub_recipes(mut self, sub_recipes: Vec<SubRecipe>) -> Self {
        self.sub_recipes = Some(sub_recipes);
        self
    }

    /// Sets the retry configuration for the Recipe
    pub fn retry(mut self, retry: RetryConfig) -> Self {
        self.retry = Some(retry);
        self
    }

    /// Builds the Recipe instance
    ///
    /// Returns an error if any required fields are missing
    pub fn build(self) -> Result<Recipe, &'static str> {
        let title = self.title.ok_or("Title is required")?;
        let description = self.description.ok_or("Description is required")?;

        if self.instructions.is_none() && self.prompt.is_none() {
            return Err("At least one of 'prompt' or 'instructions' is required");
        }

        Ok(Recipe {
            version: self.version,
            title,
            description,
            instructions: self.instructions,
            prompt: self.prompt,
            extensions: self.extensions,
            context: self.context,
            settings: self.settings,
            activities: self.activities,
            author: self.author,
            parameters: self.parameters,
            response: self.response,
            sub_recipes: self.sub_recipes,
            retry: self.retry,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_content_with_json() {
        let content = r#"{
            "version": "1.0.0",
            "title": "Test Recipe",
            "description": "A test recipe",
            "prompt": "Test prompt",
            "instructions": "Test instructions",
            "extensions": [
                {
                    "type": "stdio",
                    "name": "test_extension",
                    "cmd": "test_cmd",
                    "args": ["arg1", "arg2"],
                    "timeout": 300,
                    "description": "Test extension"
                }
            ],
            "parameters": [
                {
                    "key": "test_param",
                    "input_type": "string",
                    "requirement": "required",
                    "description": "A test parameter"
                }
            ],
            "response": {
                "json_schema": {
                    "type": "object",
                    "properties": {
                        "name": {
                            "type": "string"
                        },
                        "age": {
                            "type": "number"
                        }
                    },
                    "required": ["name"]
                }
            },
            "sub_recipes": [
                {
                    "name": "test_sub_recipe",
                    "path": "test_sub_recipe.yaml",
                    "values": {
                        "sub_recipe_param": "sub_recipe_value"
                    }
                }
            ]
        }"#;

        let recipe = Recipe::from_content(content).unwrap();
        assert_eq!(recipe.version, "1.0.0");
        assert_eq!(recipe.title, "Test Recipe");
        assert_eq!(recipe.description, "A test recipe");
        assert_eq!(recipe.instructions, Some("Test instructions".to_string()));
        assert_eq!(recipe.prompt, Some("Test prompt".to_string()));

        assert!(recipe.extensions.is_some());
        let extensions = recipe.extensions.unwrap();
        assert_eq!(extensions.len(), 1);

        assert!(recipe.parameters.is_some());
        let parameters = recipe.parameters.unwrap();
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].key, "test_param");
        assert!(matches!(
            parameters[0].input_type,
            RecipeParameterInputType::String
        ));
        assert!(matches!(
            parameters[0].requirement,
            RecipeParameterRequirement::Required
        ));

        assert!(recipe.response.is_some());
        let response = recipe.response.unwrap();
        assert!(response.json_schema.is_some());
        let json_schema = response.json_schema.unwrap();
        assert_eq!(json_schema["type"], "object");
        assert!(json_schema["properties"].is_object());
        assert_eq!(json_schema["properties"]["name"]["type"], "string");
        assert_eq!(json_schema["properties"]["age"]["type"], "number");
        assert_eq!(json_schema["required"], serde_json::json!(["name"]));

        assert!(recipe.sub_recipes.is_some());
        let sub_recipes = recipe.sub_recipes.unwrap();
        assert_eq!(sub_recipes.len(), 1);
        assert_eq!(sub_recipes[0].name, "test_sub_recipe");
        assert_eq!(sub_recipes[0].path, "test_sub_recipe.yaml");
        assert_eq!(
            sub_recipes[0].values,
            Some(HashMap::from([(
                "sub_recipe_param".to_string(),
                "sub_recipe_value".to_string()
            )]))
        );
    }

    #[test]
    fn test_from_content_with_yaml() {
        let content = r#"version: 1.0.0
title: Test Recipe
description: A test recipe
prompt: Test prompt
instructions: Test instructions
extensions:
  - type: stdio
    name: test_extension
    cmd: test_cmd
    args: [arg1, arg2]
    timeout: 300
    description: Test extension
parameters:
  - key: test_param
    input_type: string
    requirement: required
    description: A test parameter
response:
  json_schema:
    type: object
    properties:
      name:
        type: string
      age:
        type: number
    required:
      - name
sub_recipes:
  - name: test_sub_recipe
    path: test_sub_recipe.yaml
    values:
      sub_recipe_param: sub_recipe_value"#;

        let recipe = Recipe::from_content(content).unwrap();
        assert_eq!(recipe.version, "1.0.0");
        assert_eq!(recipe.title, "Test Recipe");
        assert_eq!(recipe.description, "A test recipe");
        assert_eq!(recipe.instructions, Some("Test instructions".to_string()));
        assert_eq!(recipe.prompt, Some("Test prompt".to_string()));

        assert!(recipe.extensions.is_some());
        let extensions = recipe.extensions.unwrap();
        assert_eq!(extensions.len(), 1);

        assert!(recipe.parameters.is_some());
        let parameters = recipe.parameters.unwrap();
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].key, "test_param");
        assert!(matches!(
            parameters[0].input_type,
            RecipeParameterInputType::String
        ));
        assert!(matches!(
            parameters[0].requirement,
            RecipeParameterRequirement::Required
        ));

        assert!(recipe.response.is_some());
        let response = recipe.response.unwrap();
        assert!(response.json_schema.is_some());
        let json_schema = response.json_schema.unwrap();
        assert_eq!(json_schema["type"], "object");
        assert!(json_schema["properties"].is_object());
        assert_eq!(json_schema["properties"]["name"]["type"], "string");
        assert_eq!(json_schema["properties"]["age"]["type"], "number");
        assert_eq!(json_schema["required"], serde_json::json!(["name"]));

        assert!(recipe.sub_recipes.is_some());
        let sub_recipes = recipe.sub_recipes.unwrap();
        assert_eq!(sub_recipes.len(), 1);
        assert_eq!(sub_recipes[0].name, "test_sub_recipe");
        assert_eq!(sub_recipes[0].path, "test_sub_recipe.yaml");
        assert_eq!(
            sub_recipes[0].values,
            Some(HashMap::from([(
                "sub_recipe_param".to_string(),
                "sub_recipe_value".to_string()
            )]))
        );
    }

    #[test]
    fn test_from_content_invalid_json() {
        let content = "{ invalid json }";

        let result = Recipe::from_content(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_content_missing_required_fields() {
        let content = r#"{
            "version": "1.0.0",
            "description": "A test recipe"
        }"#;

        let result = Recipe::from_content(content);
        assert!(result.is_err());
    }

    #[test]
    fn test_from_content_with_author() {
        let content = r#"{
            "version": "1.0.0",
            "title": "Test Recipe",
            "description": "A test recipe",
            "instructions": "Test instructions",
            "author": {
                "contact": "test@example.com"
            }
        }"#;

        let recipe = Recipe::from_content(content).unwrap();

        assert!(recipe.author.is_some());
        let author = recipe.author.unwrap();
        assert_eq!(author.contact, Some("test@example.com".to_string()));
    }

    #[test]
    fn test_inline_python_extension() {
        let content = r#"{
            "version": "1.0.0",
            "title": "Test Recipe",
            "description": "A test recipe",
            "instructions": "Test instructions",
            "extensions": [
                {
                    "type": "inline_python",
                    "name": "test_python",
                    "code": "print('hello world')",
                    "timeout": 300,
                    "description": "Test python extension",
                    "dependencies": ["numpy", "matplotlib"]
                }
            ]
        }"#;

        let recipe = Recipe::from_content(content).unwrap();

        assert!(recipe.extensions.is_some());
        let extensions = recipe.extensions.unwrap();
        assert_eq!(extensions.len(), 1);

        match &extensions[0] {
            ExtensionConfig::InlinePython {
                name,
                code,
                description,
                timeout,
                dependencies,
            } => {
                assert_eq!(name, "test_python");
                assert_eq!(code, "print('hello world')");
                assert_eq!(description.as_deref(), Some("Test python extension"));
                assert_eq!(timeout, &Some(300));
                assert!(dependencies.is_some());
                let deps = dependencies.as_ref().unwrap();
                assert!(deps.contains(&"numpy".to_string()));
                assert!(deps.contains(&"matplotlib".to_string()));
            }
            _ => panic!("Expected InlinePython extension"),
        }
    }

    #[test]
    fn test_from_content_with_activities() {
        let content = r#"{
            "version": "1.0.0",
            "title": "Test Recipe",
            "description": "A test recipe",
            "instructions": "Test instructions",
            "activities": ["activity1", "activity2"]
        }"#;

        let recipe = Recipe::from_content(content).unwrap();

        assert!(recipe.activities.is_some());
        let activities = recipe.activities.unwrap();
        assert_eq!(activities, vec!["activity1", "activity2"]);
    }

    #[test]
    fn test_from_content_with_nested_recipe_yaml() {
        let content = r#"name: test_recipe
recipe:
  title: Nested Recipe Test
  description: A test recipe with nested structure
  instructions: Test instructions for nested recipe
  activities:
    - Test activity 1
    - Test activity 2
  prompt: Test prompt
  extensions: []
isGlobal: true"#;

        let recipe = Recipe::from_content(content).unwrap();
        assert_eq!(recipe.title, "Nested Recipe Test");
        assert_eq!(recipe.description, "A test recipe with nested structure");
        assert_eq!(
            recipe.instructions,
            Some("Test instructions for nested recipe".to_string())
        );
        assert_eq!(recipe.prompt, Some("Test prompt".to_string()));
        assert!(recipe.activities.is_some());
        let activities = recipe.activities.unwrap();
        assert_eq!(activities, vec!["Test activity 1", "Test activity 2"]);
        assert!(recipe.extensions.is_some());
        let extensions = recipe.extensions.unwrap();
        assert_eq!(extensions.len(), 0);
    }
}
