use crate::agents::extension::ExtensionConfig;
use serde::{Deserialize, Serialize};

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
///     instructions: "Act as a helpful assistant".to_string(),
///     prompt: None,
///     extensions: None,
///     context: None,
///     activities: None,
///     author: None,
/// };
/// ```
#[derive(Serialize, Deserialize, Debug)]
pub struct Recipe {
    // Required fields
    #[serde(default = "default_version")]
    pub version: String, // version of the file format, sem ver

    pub title: String, // short title of the recipe

    pub description: String, // a longer description of the recipe

    pub instructions: String, // the instructions for the model

    // Optional fields
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
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Author {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<String>, // creator/contact information of the recipe

    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata: Option<String>, // any additional metadata for the author
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

    /// Builds the Recipe instance
    ///
    /// Returns an error if any required fields are missing
    pub fn build(self) -> Result<Recipe, &'static str> {
        let title = self.title.ok_or("Title is required")?;
        let description = self.description.ok_or("Description is required")?;
        let instructions = self.instructions.ok_or("Instructions are required")?;

        Ok(Recipe {
            version: self.version,
            title,
            description,
            instructions,
            prompt: self.prompt,
            extensions: self.extensions,
            context: self.context,
            activities: self.activities,
            author: self.author,
        })
    }
}
