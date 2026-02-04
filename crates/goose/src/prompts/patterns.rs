//! Prompt Patterns
//!
//! Pre-built prompt patterns for effective AI interactions based on best practices
//! extracted from system prompts and prompt engineering research.
//!
//! Categories of patterns:
//! - Reasoning: Chain of thought, tree of thought, self-consistency
//! - Structure: Role definition, output formatting, examples
//! - Safety: Guardrails, boundaries, ethical guidelines
//! - Task: Code generation, analysis, summarization
//! - Meta: Self-reflection, uncertainty handling, clarification

use super::errors::PromptError;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::RwLock;

/// Pattern categories for organization
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PatternCategory {
    /// Reasoning patterns (chain of thought, etc.)
    Reasoning,
    /// Structure patterns (role, format, examples)
    Structure,
    /// Safety patterns (guardrails, boundaries)
    Safety,
    /// Task-specific patterns
    Task,
    /// Meta patterns (self-reflection, uncertainty)
    Meta,
    /// Custom patterns
    #[default]
    Custom,
}

/// Metadata about a pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternMetadata {
    /// Pattern name
    pub name: String,
    /// Pattern category
    pub category: PatternCategory,
    /// Brief description
    pub description: String,
    /// When to use this pattern
    pub use_cases: Vec<String>,
    /// Patterns this works well with
    pub combines_with: Vec<String>,
    /// Example usage
    pub example: Option<String>,
}

/// A prompt pattern
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    /// Unique pattern name
    pub name: String,
    /// Pattern category
    pub category: PatternCategory,
    /// Pattern description
    pub description: String,
    /// The pattern content (may contain placeholders like {variable})
    pub content: String,
    /// Required variables
    pub required_variables: Vec<String>,
    /// Optional variables with defaults
    pub optional_variables: HashMap<String, String>,
    /// Patterns that work well with this one
    pub combines_with: Vec<String>,
    /// Use case descriptions
    pub use_cases: Vec<String>,
    /// Example of pattern in use
    pub example: Option<String>,
}

impl Pattern {
    /// Create a new pattern
    pub fn new(name: impl Into<String>, content: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            category: PatternCategory::Custom,
            description: String::new(),
            content: content.into(),
            required_variables: Vec::new(),
            optional_variables: HashMap::new(),
            combines_with: Vec::new(),
            use_cases: Vec::new(),
            example: None,
        }
    }

    /// Set the pattern category
    pub fn with_category(mut self, category: PatternCategory) -> Self {
        self.category = category;
        self
    }

    /// Set the description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a required variable
    pub fn with_required_variable(mut self, name: impl Into<String>) -> Self {
        self.required_variables.push(name.into());
        self
    }

    /// Add an optional variable with default
    pub fn with_optional_variable(
        mut self,
        name: impl Into<String>,
        default: impl Into<String>,
    ) -> Self {
        self.optional_variables.insert(name.into(), default.into());
        self
    }

    /// Add a use case
    pub fn with_use_case(mut self, use_case: impl Into<String>) -> Self {
        self.use_cases.push(use_case.into());
        self
    }

    /// Add a pattern this combines with
    pub fn combines_with_pattern(mut self, pattern_name: impl Into<String>) -> Self {
        self.combines_with.push(pattern_name.into());
        self
    }

    /// Set example
    pub fn with_example(mut self, example: impl Into<String>) -> Self {
        self.example = Some(example.into());
        self
    }

    /// Get metadata for this pattern
    pub fn metadata(&self) -> PatternMetadata {
        PatternMetadata {
            name: self.name.clone(),
            category: self.category,
            description: self.description.clone(),
            use_cases: self.use_cases.clone(),
            combines_with: self.combines_with.clone(),
            example: self.example.clone(),
        }
    }

    /// Render the pattern with variables
    pub fn render(&self, variables: &HashMap<String, String>) -> Result<String, PromptError> {
        // Check required variables
        for var in &self.required_variables {
            if !variables.contains_key(var) && !self.optional_variables.contains_key(var) {
                return Err(PromptError::MissingVariable(var.clone()));
            }
        }

        let mut result = self.content.clone();

        // Replace variables
        for (name, value) in variables {
            let placeholder = format!("{{{}}}", name);
            result = result.replace(&placeholder, value);
        }

        // Apply defaults for unset optional variables
        for (name, default) in &self.optional_variables {
            let placeholder = format!("{{{}}}", name);
            if result.contains(&placeholder) {
                result = result.replace(&placeholder, default);
            }
        }

        Ok(result)
    }
}

/// Builder for constructing prompts from patterns
pub struct PatternBuilder {
    pattern: Pattern,
    variables: HashMap<String, String>,
    prefix: Option<String>,
    suffix: Option<String>,
}

impl PatternBuilder {
    /// Create a new builder from a pattern
    pub fn new(pattern: Pattern) -> Self {
        Self {
            pattern,
            variables: HashMap::new(),
            prefix: None,
            suffix: None,
        }
    }

    /// Set a variable
    pub fn set(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.variables.insert(name.into(), value.into());
        self
    }

    /// Set multiple variables
    pub fn set_all(mut self, variables: HashMap<String, String>) -> Self {
        self.variables.extend(variables);
        self
    }

    /// Add a prefix
    pub fn with_prefix(mut self, prefix: impl Into<String>) -> Self {
        self.prefix = Some(prefix.into());
        self
    }

    /// Add a suffix
    pub fn with_suffix(mut self, suffix: impl Into<String>) -> Self {
        self.suffix = Some(suffix.into());
        self
    }

    /// Build the final prompt
    pub fn build(self) -> Result<String, PromptError> {
        let mut result = String::new();

        if let Some(prefix) = self.prefix {
            result.push_str(&prefix);
            result.push_str("\n\n");
        }

        result.push_str(&self.pattern.render(&self.variables)?);

        if let Some(suffix) = self.suffix {
            result.push_str("\n\n");
            result.push_str(&suffix);
        }

        Ok(result)
    }
}

/// Registry of prompt patterns
pub struct PatternRegistry {
    patterns: RwLock<HashMap<String, Pattern>>,
}

impl PatternRegistry {
    /// Create an empty registry
    pub fn new() -> Self {
        Self {
            patterns: RwLock::new(HashMap::new()),
        }
    }

    /// Create a registry with default patterns
    pub fn with_defaults() -> Self {
        let registry = Self::new();
        registry.load_default_patterns();
        registry
    }

    /// Register a pattern
    pub fn register(&self, pattern: Pattern) {
        let mut patterns = self.patterns.write().unwrap();
        patterns.insert(pattern.name.clone(), pattern);
    }

    /// Get a pattern by name
    pub fn get(&self, name: &str) -> Option<Pattern> {
        let patterns = self.patterns.read().unwrap();
        patterns.get(name).cloned()
    }

    /// Get all patterns in a category
    pub fn get_by_category(&self, category: PatternCategory) -> Vec<Pattern> {
        let patterns = self.patterns.read().unwrap();
        patterns
            .values()
            .filter(|p| p.category == category)
            .cloned()
            .collect()
    }

    /// List all patterns
    pub fn list(&self) -> Vec<PatternMetadata> {
        let patterns = self.patterns.read().unwrap();
        patterns.values().map(|p| p.metadata()).collect()
    }

    /// Remove a pattern
    pub fn remove(&self, name: &str) -> Option<Pattern> {
        let mut patterns = self.patterns.write().unwrap();
        patterns.remove(name)
    }

    /// Load default patterns
    fn load_default_patterns(&self) {
        // ================================================================
        // REASONING PATTERNS
        // ================================================================

        self.register(
            Pattern::new(
                "chain_of_thought",
                r#"Think through this problem step by step:

1. First, understand what is being asked
2. Identify the key components and constraints
3. Break down the problem into smaller parts
4. Solve each part systematically
5. Combine the solutions
6. Verify the final answer

{task}

Let me work through this step by step..."#,
            )
            .with_category(PatternCategory::Reasoning)
            .with_description("Encourages systematic step-by-step reasoning")
            .with_required_variable("task")
            .with_use_case("Complex problem solving")
            .with_use_case("Mathematical reasoning")
            .with_use_case("Debugging code")
            .combines_with_pattern("self_consistency"),
        );

        self.register(
            Pattern::new(
                "tree_of_thought",
                r#"Explore multiple approaches to solve this problem:

## Approach 1: {approach_1}
- Pros:
- Cons:
- Feasibility:

## Approach 2: {approach_2}
- Pros:
- Cons:
- Feasibility:

## Approach 3: {approach_3}
- Pros:
- Cons:
- Feasibility:

After evaluating all approaches, select the best one and explain why.

Problem: {task}"#,
            )
            .with_category(PatternCategory::Reasoning)
            .with_description("Explores multiple solution paths before committing")
            .with_required_variable("task")
            .with_optional_variable("approach_1", "Direct approach")
            .with_optional_variable("approach_2", "Alternative approach")
            .with_optional_variable("approach_3", "Creative approach")
            .with_use_case("Design decisions")
            .with_use_case("Architecture planning")
            .with_use_case("Strategy selection"),
        );

        self.register(
            Pattern::new(
                "self_consistency",
                r#"I will solve this problem multiple times using different reasoning paths, then compare the answers for consistency.

## Attempt 1:
{task}

## Attempt 2 (different approach):
{task}

## Attempt 3 (verify):
{task}

## Conclusion:
Compare the three attempts. If they agree, we have high confidence. If they differ, analyze why and determine the correct answer."#,
            )
            .with_category(PatternCategory::Reasoning)
            .with_description("Increases reliability through multiple independent solutions")
            .with_required_variable("task")
            .with_use_case("Mathematical calculations")
            .with_use_case("Fact verification")
            .with_use_case("Critical decisions"),
        );

        // ================================================================
        // STRUCTURE PATTERNS
        // ================================================================

        self.register(
            Pattern::new(
                "role_definition",
                r#"You are a {role} with expertise in {expertise}.

Your responsibilities include:
{responsibilities}

When responding, you should:
- Draw on your expertise in {expertise}
- Provide {response_style} responses
- Focus on {focus_areas}

You should NOT:
- {constraint_1}
- {constraint_2}"#,
            )
            .with_category(PatternCategory::Structure)
            .with_description("Defines a clear role and persona for the AI")
            .with_required_variable("role")
            .with_required_variable("expertise")
            .with_optional_variable(
                "responsibilities",
                "- Answering questions accurately\n- Providing helpful guidance",
            )
            .with_optional_variable("response_style", "clear and concise")
            .with_optional_variable("focus_areas", "practical solutions")
            .with_optional_variable("constraint_1", "Make claims without evidence")
            .with_optional_variable("constraint_2", "Provide harmful information")
            .with_use_case("Chatbot personas")
            .with_use_case("Specialized assistants")
            .combines_with_pattern("output_format"),
        );

        self.register(
            Pattern::new(
                "output_format",
                r#"Please format your response as follows:

{format_specification}

Example format:
```
{example_format}
```

Ensure your response follows this exact structure."#,
            )
            .with_category(PatternCategory::Structure)
            .with_description("Specifies exact output format requirements")
            .with_required_variable("format_specification")
            .with_optional_variable("example_format", "{\n  \"key\": \"value\"\n}")
            .with_use_case("API responses")
            .with_use_case("Data extraction")
            .with_use_case("Structured outputs"),
        );

        self.register(
            Pattern::new(
                "few_shot_examples",
                r#"Here are some examples of how to handle similar tasks:

## Example 1:
Input: {example_1_input}
Output: {example_1_output}

## Example 2:
Input: {example_2_input}
Output: {example_2_output}

## Example 3:
Input: {example_3_input}
Output: {example_3_output}

Now apply the same approach to:
Input: {task}"#,
            )
            .with_category(PatternCategory::Structure)
            .with_description("Provides examples to guide response format and style")
            .with_required_variable("task")
            .with_required_variable("example_1_input")
            .with_required_variable("example_1_output")
            .with_optional_variable("example_2_input", "")
            .with_optional_variable("example_2_output", "")
            .with_optional_variable("example_3_input", "")
            .with_optional_variable("example_3_output", "")
            .with_use_case("Teaching new formats")
            .with_use_case("Consistent outputs")
            .with_use_case("Style matching"),
        );

        // ================================================================
        // SAFETY PATTERNS
        // ================================================================

        self.register(
            Pattern::new(
                "safety_boundaries",
                r#"Important safety guidelines:

I will NOT:
- Generate harmful, illegal, or unethical content
- Provide instructions for dangerous activities
- Share personal information about real individuals
- Generate malicious code or security exploits
- Produce content that could harm individuals or groups

If asked to do any of the above, I will politely decline and explain why.

With these boundaries in mind, I'll help with: {task}"#,
            )
            .with_category(PatternCategory::Safety)
            .with_description("Establishes clear safety boundaries")
            .with_required_variable("task")
            .with_use_case("General safety")
            .with_use_case("Content moderation")
            .combines_with_pattern("role_definition"),
        );

        self.register(
            Pattern::new(
                "uncertainty_acknowledgment",
                r#"When responding, I will:
- Clearly state my confidence level
- Use phrases like "I believe", "I'm not certain", or "Based on my knowledge" when appropriate
- Acknowledge the limits of my knowledge (training cutoff: {knowledge_cutoff})
- Recommend verification from authoritative sources for critical information
- Distinguish between facts, opinions, and speculation

{task}"#,
            )
            .with_category(PatternCategory::Safety)
            .with_description("Encourages honest acknowledgment of uncertainty")
            .with_required_variable("task")
            .with_optional_variable("knowledge_cutoff", "2024")
            .with_use_case("Factual queries")
            .with_use_case("Medical/legal advice")
            .with_use_case("Current events"),
        );

        // ================================================================
        // TASK PATTERNS
        // ================================================================

        self.register(
            Pattern::new(
                "code_generation",
                r#"Generate code with the following requirements:

**Language:** {language}
**Task:** {task}

**Requirements:**
{requirements}

**Code style:**
- Write clean, readable code
- Include appropriate comments
- Follow {language} best practices
- Handle errors appropriately
- Consider edge cases

Please provide the code with explanations for key decisions."#,
            )
            .with_category(PatternCategory::Task)
            .with_description("Structured code generation with requirements")
            .with_required_variable("language")
            .with_required_variable("task")
            .with_optional_variable(
                "requirements",
                "- Implement the core functionality\n- Include error handling",
            )
            .with_use_case("Writing functions")
            .with_use_case("Building features")
            .combines_with_pattern("chain_of_thought"),
        );

        self.register(
            Pattern::new(
                "code_review",
                r#"Please review the following code:

```{language}
{code}
```

Analyze the code for:
1. **Correctness**: Does it work as intended?
2. **Bugs**: Are there any potential bugs or edge cases?
3. **Security**: Are there any security vulnerabilities?
4. **Performance**: Are there any performance issues?
5. **Readability**: Is the code clean and well-organized?
6. **Best Practices**: Does it follow {language} conventions?

Provide specific suggestions for improvement with code examples where helpful."#,
            )
            .with_category(PatternCategory::Task)
            .with_description("Comprehensive code review checklist")
            .with_required_variable("language")
            .with_required_variable("code")
            .with_use_case("Code reviews")
            .with_use_case("Quality assurance")
            .with_use_case("Learning"),
        );

        self.register(
            Pattern::new(
                "summarization",
                r#"Please summarize the following content:

{content}

**Summary requirements:**
- Length: {length}
- Focus: {focus}
- Audience: {audience}
- Style: {style}

Include:
- Key points and main ideas
- Important details
- Conclusions or takeaways"#,
            )
            .with_category(PatternCategory::Task)
            .with_description("Structured summarization with customizable parameters")
            .with_required_variable("content")
            .with_optional_variable("length", "2-3 paragraphs")
            .with_optional_variable("focus", "main ideas")
            .with_optional_variable("audience", "general")
            .with_optional_variable("style", "professional")
            .with_use_case("Document summarization")
            .with_use_case("Meeting notes")
            .with_use_case("Research papers"),
        );

        self.register(
            Pattern::new(
                "analysis",
                r#"Analyze the following:

{subject}

**Analysis Framework:**

1. **Overview**: What is this? What's its purpose?

2. **Key Components**: What are the main parts or elements?

3. **Strengths**: What works well?

4. **Weaknesses**: What could be improved?

5. **Patterns**: What patterns or trends do you observe?

6. **Implications**: What are the consequences or effects?

7. **Recommendations**: What actions or changes would you suggest?

{additional_context}"#,
            )
            .with_category(PatternCategory::Task)
            .with_description("Comprehensive analysis framework")
            .with_required_variable("subject")
            .with_optional_variable("additional_context", "")
            .with_use_case("Business analysis")
            .with_use_case("Technical evaluation")
            .with_use_case("Research"),
        );

        // ================================================================
        // META PATTERNS
        // ================================================================

        self.register(
            Pattern::new(
                "self_reflection",
                r#"Before providing my final answer, I'll reflect on my reasoning:

**Initial Response:**
{initial_response}

**Self-Check Questions:**
1. Did I understand the question correctly?
2. Are my assumptions valid?
3. Did I consider alternative interpretations?
4. Are there any gaps in my reasoning?
5. Could my answer be improved?

**Refined Response:**
Based on this reflection..."#,
            )
            .with_category(PatternCategory::Meta)
            .with_description("Encourages self-checking before final response")
            .with_optional_variable("initial_response", "[Your initial thoughts here]")
            .with_use_case("Complex questions")
            .with_use_case("Quality improvement")
            .combines_with_pattern("chain_of_thought"),
        );

        self.register(
            Pattern::new(
                "clarification_request",
                r#"Before I proceed, I'd like to clarify a few things to ensure I provide the most helpful response:

Regarding: {topic}

**Questions:**
{questions}

**Assumptions I'm making (please correct if wrong):**
{assumptions}

Once you confirm or clarify, I'll provide a comprehensive response."#,
            )
            .with_category(PatternCategory::Meta)
            .with_description("Requests clarification when requirements are ambiguous")
            .with_required_variable("topic")
            .with_optional_variable("questions", "1. [Question 1]\n2. [Question 2]")
            .with_optional_variable("assumptions", "- [Assumption 1]\n- [Assumption 2]")
            .with_use_case("Ambiguous requests")
            .with_use_case("Complex projects")
            .with_use_case("Requirements gathering"),
        );

        self.register(
            Pattern::new(
                "iterative_refinement",
                r#"I'll approach this iteratively:

**Version 1 (Initial Attempt):**
{task}

**Review & Improvements:**
What could be better about version 1?

**Version 2 (Refined):**
Incorporating the improvements...

**Final Check:**
Is this the best I can do? Any remaining issues?

**Final Version:**
The polished result..."#,
            )
            .with_category(PatternCategory::Meta)
            .with_description("Builds quality through iteration")
            .with_required_variable("task")
            .with_use_case("Creative writing")
            .with_use_case("Design work")
            .with_use_case("Quality-critical outputs"),
        );
    }
}

impl Default for PatternRegistry {
    fn default() -> Self {
        Self::with_defaults()
    }
}

/// Pattern library - collection of related patterns
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PatternLibrary {
    /// Library name
    pub name: String,
    /// Library description
    pub description: String,
    /// Patterns in this library
    pub patterns: Vec<Pattern>,
}

impl PatternLibrary {
    /// Create a new library
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            patterns: Vec::new(),
        }
    }

    /// Add a pattern to the library
    pub fn add_pattern(&mut self, pattern: Pattern) {
        self.patterns.push(pattern);
    }

    /// Get a pattern by name
    pub fn get(&self, name: &str) -> Option<&Pattern> {
        self.patterns.iter().find(|p| p.name == name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::new("test", "Test content")
            .with_category(PatternCategory::Reasoning)
            .with_description("A test pattern")
            .with_required_variable("var1")
            .with_optional_variable("var2", "default");

        assert_eq!(pattern.name, "test");
        assert_eq!(pattern.category, PatternCategory::Reasoning);
        assert!(pattern.required_variables.contains(&"var1".to_string()));
        assert!(pattern.optional_variables.contains_key("var2"));
    }

    #[test]
    fn test_pattern_render() {
        let pattern = Pattern::new("test", "Hello {name}, you are {age} years old")
            .with_required_variable("name")
            .with_optional_variable("age", "unknown");

        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "World".to_string());

        let result = pattern.render(&vars).unwrap();
        assert!(result.contains("Hello World"));
        assert!(result.contains("unknown years old"));
    }

    #[test]
    fn test_pattern_render_missing_required() {
        let pattern = Pattern::new("test", "Hello {name}").with_required_variable("name");

        let vars = HashMap::new();
        let result = pattern.render(&vars);
        assert!(result.is_err());
    }

    #[test]
    fn test_pattern_builder() {
        let pattern = Pattern::new("test", "Hello {name}!").with_required_variable("name");

        let result = PatternBuilder::new(pattern)
            .set("name", "World")
            .with_prefix("Prefix")
            .with_suffix("Suffix")
            .build()
            .unwrap();

        assert!(result.contains("Prefix"));
        assert!(result.contains("Hello World!"));
        assert!(result.contains("Suffix"));
    }

    #[test]
    fn test_pattern_registry_defaults() {
        let registry = PatternRegistry::with_defaults();

        // Check that default patterns are loaded
        assert!(registry.get("chain_of_thought").is_some());
        assert!(registry.get("role_definition").is_some());
        assert!(registry.get("code_generation").is_some());
        assert!(registry.get("safety_boundaries").is_some());
    }

    #[test]
    fn test_pattern_registry_operations() {
        let registry = PatternRegistry::new();

        let pattern = Pattern::new("custom", "Custom content");
        registry.register(pattern);

        assert!(registry.get("custom").is_some());
        assert!(registry.remove("custom").is_some());
        assert!(registry.get("custom").is_none());
    }

    #[test]
    fn test_get_by_category() {
        let registry = PatternRegistry::with_defaults();

        let reasoning_patterns = registry.get_by_category(PatternCategory::Reasoning);
        assert!(!reasoning_patterns.is_empty());

        for pattern in reasoning_patterns {
            assert_eq!(pattern.category, PatternCategory::Reasoning);
        }
    }

    #[test]
    fn test_pattern_metadata() {
        let pattern = Pattern::new("test", "content")
            .with_category(PatternCategory::Task)
            .with_description("Test description")
            .with_use_case("Use case 1");

        let metadata = pattern.metadata();
        assert_eq!(metadata.name, "test");
        assert_eq!(metadata.category, PatternCategory::Task);
        assert_eq!(metadata.description, "Test description");
        assert!(metadata.use_cases.contains(&"Use case 1".to_string()));
    }
}
