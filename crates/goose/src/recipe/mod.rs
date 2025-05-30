use std::fmt;

use crate::agents::extension::ExtensionConfig;
use crate::prompt_template::render_inline_once;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
///
/// # Example
///
/// ```
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
///     parameters: None,
/// };
/// ```
#[derive(Serialize, Deserialize, Debug)]
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
    pub activities: Option<Vec<String>>, // the activity pills that show up when loading the

    #[serde(skip_serializing_if = "Option::is_none")]
    pub author: Option<Author>, // any additional author information

    #[serde(skip_serializing_if = "Option::is_none")]
    pub parameters: Option<Vec<RecipeParameter>>, // any additional parameters for the recipe
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Author {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>, // creator/contact information of the recipe

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>, // any additional metadata for the author
}

#[derive(Serialize, Deserialize, Debug)]
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

#[derive(Serialize, Deserialize, Debug)]
#[serde(rename_all = "snake_case")]
pub enum RecipeParameterInputType {
    String,
    Number,
    Boolean,
    Date,
    File,
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

#[derive(Serialize, Deserialize, Debug)]
pub struct RecipeParameter {
    pub key: String,
    pub input_type: RecipeParameterInputType,
    pub requirement: RecipeParameterRequirement,
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<String>,
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
    activities: Option<Vec<String>>,
    author: Option<Author>,
    parameters: Option<Vec<RecipeParameter>>,
}

impl Recipe {
    /// Creates a new RecipeBuilder to construct a Recipe instance
    ///
    /// # Example
    ///
    /// ```
    /// use goose::recipe::Recipe;
    ///
    /// let recipe = Recipe::builder()
    ///     .title("My Recipe")
    ///     .description("A helpful assistant")
    ///     .instructions("Act as a helpful assistant")
    ///     .build()
    ///     .expect("Failed to build Recipe: missing required fields");
    /// ```
    pub fn builder() -> RecipeBuilder {
        RecipeBuilder {
            version: default_version(),
            title: None,
            description: None,
            instructions: None,
            prompt: None,
            extensions: None,
            context: None,
            activities: None,
            author: None,
            parameters: None,
        }
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
            activities: self.activities,
            author: self.author,
            parameters: self.parameters,
        })
    }
}

// Applies template parameters to all fields in a Recipe.
pub fn apply_recipe_parameters(recipe: &mut Recipe, params: &HashMap<String, String>) {
    let recipe_json = serde_json::to_string(recipe).expect("Failed to serialize recipe");
    let rendered_json = render_inline_once(&recipe_json, params).expect("Failed to render template");
    *recipe = serde_json::from_str(&rendered_json).expect("Failed to deserialize rendered recipe");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_apply_recipe_parameters() {
        let mut recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "Test Recipe".to_string(),
            description: "A test recipe".to_string(),
            instructions: Some("Hello {{ name }}, welcome to {{ location }}!".to_string()),
            prompt: Some("Your task is {{ task }}".to_string()),
            activities: Some(vec![
                "Learn about {{ topic }}".to_string(),
                "Practice {{ skill }}".to_string(),
            ]),
            context: Some(vec![
                "Context: {{ context_info }}".to_string(),
            ]),
            extensions: None,
            author: None,
            parameters: None,
        };

        let mut params = HashMap::new();
        params.insert("name".to_string(), "Alice".to_string());
        params.insert("location".to_string(), "Wonderland".to_string());
        params.insert("task".to_string(), "solve puzzles".to_string());
        params.insert("topic".to_string(), "logic".to_string());
        params.insert("skill".to_string(), "critical thinking".to_string());
        params.insert("context_info".to_string(), "fantasy world".to_string());

        apply_recipe_parameters(&mut recipe, &params);

        assert_eq!(recipe.instructions, Some("Hello Alice, welcome to Wonderland!".to_string()));
        assert_eq!(recipe.prompt, Some("Your task is solve puzzles".to_string()));
        assert_eq!(recipe.activities, Some(vec![
            "Learn about logic".to_string(),
            "Practice critical thinking".to_string(),
        ]));
        assert_eq!(recipe.context, Some(vec![
            "Context: fantasy world".to_string(),
        ]));
    }

    #[test]
    fn test_apply_recipe_parameters_empty_fields() {
        let mut recipe = Recipe {
            version: "1.0.0".to_string(),
            title: "Test Recipe".to_string(),
            description: "A test recipe".to_string(),
            instructions: None,
            prompt: None,
            activities: None,
            context: None,
            extensions: None,
            author: None,
            parameters: None,
        };

        let mut params = HashMap::new();
        params.insert("unused".to_string(), "value".to_string());

        apply_recipe_parameters(&mut recipe, &params);

        assert_eq!(recipe.instructions, None);
        assert_eq!(recipe.prompt, None);
        assert_eq!(recipe.activities, None);
        assert_eq!(recipe.context, None);
    }

    #[test]
    fn test_apply_recipe_parameters_handles_all_fields() {
        let mut recipe = Recipe {
            version: "{{ version }}".to_string(),
            title: "{{ title }}".to_string(),
            description: "{{ description }}".to_string(),
            instructions: Some("{{ instructions }}".to_string()),
            prompt: Some("{{ prompt }}".to_string()),
            activities: Some(vec!["{{ activity }}".to_string()]),
            context: Some(vec!["{{ context }}".to_string()]),
            extensions: None,
            author: None,
            parameters: None,
        };

        let mut params = HashMap::new();
        params.insert("version".to_string(), "2.0.0".to_string());
        params.insert("title".to_string(), "Rendered Title".to_string());
        params.insert("description".to_string(), "Rendered Description".to_string());
        params.insert("instructions".to_string(), "Rendered Instructions".to_string());
        params.insert("prompt".to_string(), "Rendered Prompt".to_string());
        params.insert("activity".to_string(), "Rendered Activity".to_string());
        params.insert("context".to_string(), "Rendered Context".to_string());

        apply_recipe_parameters(&mut recipe, &params);

        // Verify that ALL fields were rendered, including version, title, description
        assert_eq!(recipe.version, "2.0.0");
        assert_eq!(recipe.title, "Rendered Title");
        assert_eq!(recipe.description, "Rendered Description");
        assert_eq!(recipe.instructions, Some("Rendered Instructions".to_string()));
        assert_eq!(recipe.prompt, Some("Rendered Prompt".to_string()));
        assert_eq!(recipe.activities, Some(vec!["Rendered Activity".to_string()]));
        assert_eq!(recipe.context, Some(vec!["Rendered Context".to_string()]));
    }
}
