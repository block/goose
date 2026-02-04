//! Template System
//!
//! Provides template rendering with variable substitution for prompt patterns.
//! Supports Handlebars-like syntax with {variable} placeholders.

use super::errors::PromptError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Variable type for validation
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum VariableType {
    /// String variable
    #[default]
    String,
    /// Number variable
    Number,
    /// Boolean variable
    Boolean,
    /// Array of strings
    Array,
    /// Object (JSON-like)
    Object,
    /// Any type (no validation)
    Any,
}

/// Template variable definition
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateVariable {
    /// Variable name
    pub name: String,
    /// Variable type
    pub var_type: VariableType,
    /// Whether this variable is required
    pub required: bool,
    /// Default value if not provided
    pub default: Option<String>,
    /// Description of the variable
    pub description: String,
    /// Example value
    pub example: Option<String>,
}

impl TemplateVariable {
    /// Create a new variable
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            var_type: VariableType::String,
            required: false,
            default: None,
            description: String::new(),
            example: None,
        }
    }

    /// Set as required
    pub fn required(mut self) -> Self {
        self.required = true;
        self
    }

    /// Set variable type
    pub fn with_type(mut self, var_type: VariableType) -> Self {
        self.var_type = var_type;
        self
    }

    /// Set default value
    pub fn with_default(mut self, default: impl Into<String>) -> Self {
        self.default = Some(default.into());
        self
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set example
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }

    /// Validate a value against this variable's type
    pub fn validate(&self, value: &str) -> Result<(), PromptError> {
        match self.var_type {
            VariableType::String => Ok(()),
            VariableType::Number => {
                value
                    .parse::<f64>()
                    .map_err(|_| PromptError::invalid_variable(&self.name, "Not a valid number"))?;
                Ok(())
            }
            VariableType::Boolean => match value.to_lowercase().as_str() {
                "true" | "false" | "yes" | "no" | "1" | "0" => Ok(()),
                _ => Err(PromptError::invalid_variable(
                    &self.name,
                    "Not a valid boolean",
                )),
            },
            VariableType::Array => {
                // Basic validation - check if it looks like a comma-separated list
                if value.contains(',') || !value.is_empty() {
                    Ok(())
                } else {
                    Err(PromptError::invalid_variable(
                        &self.name,
                        "Array cannot be empty",
                    ))
                }
            }
            VariableType::Object => {
                // Basic validation - check if it looks like JSON
                if value.starts_with('{') && value.ends_with('}') {
                    Ok(())
                } else {
                    Err(PromptError::invalid_variable(
                        &self.name,
                        "Should be a JSON object",
                    ))
                }
            }
            VariableType::Any => Ok(()),
        }
    }
}

/// A prompt template
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Template {
    /// Template name
    pub name: String,
    /// Template content with {variable} placeholders
    pub content: String,
    /// Template description
    pub description: String,
    /// Variables used in this template
    pub variables: Vec<TemplateVariable>,
    /// Template category/tags
    pub tags: Vec<String>,
    /// Template version
    pub version: String,
}

impl Template {
    /// Create a new template
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            content: content.into(),
            description: String::new(),
            variables: Vec::new(),
            tags: Vec::new(),
            version: "1.0".to_string(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a variable
    pub fn with_variable(mut self, variable: TemplateVariable) -> Self {
        self.variables.push(variable);
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Set version
    pub fn with_version(mut self, version: impl Into<String>) -> Self {
        self.version = version.into();
        self
    }

    /// Render the template with provided variables
    pub fn render(&self, variables: &HashMap<String, String>) -> Result<String, PromptError> {
        // Validate required variables
        for var in &self.variables {
            if var.required && !variables.contains_key(&var.name) && var.default.is_none() {
                return Err(PromptError::missing_variable(&var.name));
            }

            // Validate provided values
            if let Some(value) = variables.get(&var.name) {
                var.validate(value)?;
            }
        }

        let mut result = self.content.clone();

        // Replace variables
        for var in &self.variables {
            let placeholder = format!("{{{}}}", var.name);

            let value = if let Some(provided) = variables.get(&var.name) {
                provided.clone()
            } else if let Some(default) = &var.default {
                default.clone()
            } else {
                continue; // Skip optional variables without values
            };

            result = result.replace(&placeholder, &value);
        }

        // Check for unreplaced placeholders (variables not defined in template)
        if result.contains('{') && result.contains('}') {
            let remaining: Vec<&str> = result
                .split('{')
                .skip(1)
                .filter_map(|s| s.split('}').next())
                .collect();

            if !remaining.is_empty() {
                return Err(PromptError::template_parse(format!(
                    "Undefined variables: {}",
                    remaining.join(", ")
                )));
            }
        }

        Ok(result)
    }

    /// Get list of required variables
    pub fn required_variables(&self) -> Vec<&str> {
        self.variables
            .iter()
            .filter(|v| v.required && v.default.is_none())
            .map(|v| v.name.as_str())
            .collect()
    }

    /// Get list of all variables
    pub fn all_variables(&self) -> Vec<&str> {
        self.variables.iter().map(|v| v.name.as_str()).collect()
    }
}

/// Template engine for managing and rendering templates
pub struct TemplateEngine {
    templates: RwLock<HashMap<String, Template>>,
}

impl TemplateEngine {
    /// Create a new template engine
    pub fn new() -> Self {
        Self {
            templates: RwLock::new(HashMap::new()),
        }
    }

    /// Register a template
    pub fn register(&self, template: Template) {
        let mut templates = self.templates.write().unwrap();
        templates.insert(template.name.clone(), template);
    }

    /// Get a template by name
    pub fn get(&self, name: &str) -> Option<Template> {
        let templates = self.templates.read().unwrap();
        templates.get(name).cloned()
    }

    /// Render a template
    pub fn render(
        &self,
        name: &str,
        variables: &HashMap<String, String>,
    ) -> Result<String, PromptError> {
        let template = self
            .get(name)
            .ok_or_else(|| PromptError::TemplateNotFound(name.to_string()))?;

        template.render(variables)
    }

    /// List all templates
    pub fn list(&self) -> Vec<String> {
        let templates = self.templates.read().unwrap();
        templates.keys().cloned().collect()
    }

    /// Remove a template
    pub fn remove(&self, name: &str) -> Option<Template> {
        let mut templates = self.templates.write().unwrap();
        templates.remove(name)
    }

    /// Get template count
    pub fn template_count(&self) -> usize {
        let templates = self.templates.read().unwrap();
        templates.len()
    }

    /// Load default templates
    pub fn load_defaults(&self) {
        // System message templates
        self.register(
            Template::new(
                "system_message",
                r#"You are {role}.

{context}

Your capabilities include:
{capabilities}

Guidelines:
{guidelines}

When responding, please:
{instructions}"#,
            )
            .with_description("Basic system message template")
            .with_variable(
                TemplateVariable::new("role")
                    .required()
                    .with_description("The role/persona for the AI")
                    .with_example("a helpful coding assistant"),
            )
            .with_variable(
                TemplateVariable::new("context")
                    .with_description("Additional context information")
                    .with_default(""),
            )
            .with_variable(
                TemplateVariable::new("capabilities")
                    .with_description("List of capabilities")
                    .with_default("- Answering questions\n- Providing explanations\n- Helping with tasks"),
            )
            .with_variable(
                TemplateVariable::new("guidelines")
                    .with_description("Behavioral guidelines")
                    .with_default("- Be helpful and accurate\n- Ask for clarification when needed\n- Admit uncertainty when appropriate"),
            )
            .with_variable(
                TemplateVariable::new("instructions")
                    .with_description("Specific response instructions")
                    .with_default("- Be clear and concise\n- Provide examples when helpful\n- Follow best practices"),
            )
            .with_tag("system")
            .with_tag("general"),
        );

        // Code review template
        self.register(
            Template::new(
                "code_review",
                r#"Please review the following {language} code:

```{language}
{code}
```

Review criteria:
{criteria}

Focus areas:
{focus_areas}

Please provide feedback on:
1. Code correctness and functionality
2. Security considerations
3. Performance implications
4. Code style and readability
5. Best practices adherence

{additional_instructions}"#,
            )
            .with_description("Template for code review requests")
            .with_variable(
                TemplateVariable::new("language")
                    .required()
                    .with_description("Programming language")
                    .with_example("Python"),
            )
            .with_variable(
                TemplateVariable::new("code")
                    .required()
                    .with_description("Code to review"),
            )
            .with_variable(
                TemplateVariable::new("criteria")
                    .with_description("Specific review criteria")
                    .with_default("- Functionality\n- Security\n- Performance\n- Maintainability"),
            )
            .with_variable(
                TemplateVariable::new("focus_areas")
                    .with_description("Areas to focus on")
                    .with_default("General code quality"),
            )
            .with_variable(
                TemplateVariable::new("additional_instructions")
                    .with_description("Additional review instructions")
                    .with_default(""),
            )
            .with_tag("code")
            .with_tag("review"),
        );

        // Task planning template
        self.register(
            Template::new(
                "task_planning",
                r#"Task: {task}

Context:
{context}

Requirements:
{requirements}

Please create a step-by-step plan:

1. **Analysis Phase**
   - Understand the requirements
   - Identify key components
   - Note potential challenges

2. **Planning Phase**
   - Break down into subtasks
   - Identify dependencies
   - Estimate effort

3. **Implementation Phase**
   - Execute the plan
   - Monitor progress
   - Adjust as needed

4. **Validation Phase**
   - Test the results
   - Verify requirements met
   - Document outcomes

Additional considerations:
{considerations}"#,
            )
            .with_description("Template for task planning")
            .with_variable(
                TemplateVariable::new("task")
                    .required()
                    .with_description("The main task to plan"),
            )
            .with_variable(
                TemplateVariable::new("context")
                    .with_description("Background context")
                    .with_default(""),
            )
            .with_variable(
                TemplateVariable::new("requirements")
                    .required()
                    .with_description("Task requirements"),
            )
            .with_variable(
                TemplateVariable::new("considerations")
                    .with_description("Additional considerations")
                    .with_default("- Time constraints\n- Resource availability\n- Risk factors"),
            )
            .with_tag("planning")
            .with_tag("project"),
        );

        // Error analysis template
        self.register(
            Template::new(
                "error_analysis",
                r#"Error Analysis Report

**Error:** {error_message}

**Context:** {context}

**Environment:**
{environment}

**Analysis Framework:**

1. **Error Classification**
   - Type: {error_type}
   - Severity: {severity}
   - Impact: {impact}

2. **Root Cause Analysis**
   - What happened?
   - Why did it happen?
   - What were the contributing factors?

3. **Investigation Steps**
   - Reproduce the error
   - Check logs and traces
   - Analyze code paths
   - Review recent changes

4. **Solution Strategy**
   - Immediate fixes
   - Long-term improvements
   - Prevention measures

5. **Next Steps**
   - Action items
   - Testing plan
   - Monitoring requirements

{additional_notes}"#,
            )
            .with_description("Template for error analysis")
            .with_variable(
                TemplateVariable::new("error_message")
                    .required()
                    .with_description("The error message or description"),
            )
            .with_variable(
                TemplateVariable::new("context")
                    .with_description("Context where error occurred")
                    .with_default(""),
            )
            .with_variable(
                TemplateVariable::new("environment")
                    .with_description("Environment details")
                    .with_default(""),
            )
            .with_variable(
                TemplateVariable::new("error_type")
                    .with_description("Type of error")
                    .with_default("Unknown"),
            )
            .with_variable(
                TemplateVariable::new("severity")
                    .with_description("Error severity")
                    .with_default("Medium"),
            )
            .with_variable(
                TemplateVariable::new("impact")
                    .with_description("Impact of the error")
                    .with_default(""),
            )
            .with_variable(
                TemplateVariable::new("additional_notes")
                    .with_description("Additional notes or context")
                    .with_default(""),
            )
            .with_tag("error")
            .with_tag("analysis"),
        );
    }
}

impl Default for TemplateEngine {
    fn default() -> Self {
        let engine = Self::new();
        engine.load_defaults();
        engine
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_creation() {
        let template = Template::new("test", "Hello {name}!")
            .with_description("Test template")
            .with_variable(
                TemplateVariable::new("name")
                    .required()
                    .with_description("Name to greet"),
            );

        assert_eq!(template.name, "test");
        assert_eq!(template.content, "Hello {name}!");
        assert_eq!(template.variables.len(), 1);
        assert!(template.variables[0].required);
    }

    #[test]
    fn test_template_render() {
        let template = Template::new("test", "Hello {name}, you are {age} years old!")
            .with_variable(TemplateVariable::new("name").required())
            .with_variable(
                TemplateVariable::new("age")
                    .with_type(VariableType::Number)
                    .with_default("25"),
            );

        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "Alice".to_string());

        let result = template.render(&variables).unwrap();
        assert_eq!(result, "Hello Alice, you are 25 years old!");
    }

    #[test]
    fn test_template_render_missing_required() {
        let template = Template::new("test", "Hello {name}!")
            .with_variable(TemplateVariable::new("name").required());

        let variables = HashMap::new();
        let result = template.render(&variables);
        assert!(result.is_err());
    }

    #[test]
    fn test_variable_validation() {
        let var = TemplateVariable::new("age").with_type(VariableType::Number);

        assert!(var.validate("25").is_ok());
        assert!(var.validate("25.5").is_ok());
        assert!(var.validate("not_a_number").is_err());

        let bool_var = TemplateVariable::new("flag").with_type(VariableType::Boolean);
        assert!(bool_var.validate("true").is_ok());
        assert!(bool_var.validate("false").is_ok());
        assert!(bool_var.validate("yes").is_ok());
        assert!(bool_var.validate("maybe").is_err());
    }

    #[test]
    fn test_template_engine() {
        let engine = TemplateEngine::new();

        let template = Template::new("greeting", "Hello {name}!")
            .with_variable(TemplateVariable::new("name").required());

        engine.register(template);

        let mut variables = HashMap::new();
        variables.insert("name".to_string(), "World".to_string());

        let result = engine.render("greeting", &variables).unwrap();
        assert_eq!(result, "Hello World!");
    }

    #[test]
    fn test_template_engine_defaults() {
        let engine = TemplateEngine::default();

        assert!(engine.template_count() > 0);
        assert!(engine.get("system_message").is_some());
        assert!(engine.get("code_review").is_some());
    }

    #[test]
    fn test_template_required_variables() {
        let template = Template::new("test", "Hello {name}, you are {age}")
            .with_variable(TemplateVariable::new("name").required())
            .with_variable(TemplateVariable::new("age").with_default("25"));

        let required = template.required_variables();
        assert_eq!(required, vec!["name"]);

        let all = template.all_variables();
        assert_eq!(all, vec!["name", "age"]);
    }
}
