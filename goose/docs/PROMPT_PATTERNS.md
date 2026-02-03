# Goose Prompt Patterns

## Overview

The Prompts module provides a library of pre-built prompt patterns and a template system for effective AI interactions. Based on best practices from prompt engineering research and production system prompts.

## Features

- **14 Pre-built Patterns**: Reasoning, structure, safety, task, and meta patterns
- **Template System**: Variable substitution with validation
- **Pattern Composition**: Combine multiple patterns
- **Category Filtering**: Find patterns by use case
- **PatternBuilder API**: Fluent interface for building prompts

## Quick Start

```rust
use goose::prompts::{PromptManager, PatternCategory};

// Create prompt manager
let manager = PromptManager::new();

// Get a pattern
let pattern = manager.get_pattern("chain_of_thought")?;

// Build a prompt
let prompt = manager.build_prompt("chain_of_thought")?
    .set("task", "Implement a binary search algorithm")
    .build()?;

// Compose multiple patterns
let composed = manager.compose_patterns(&[
    "role_definition",
    "chain_of_thought",
    "safety_boundaries"
])?;
```

## Pattern Categories

### Reasoning Patterns

Patterns that guide the AI's thinking process.

| Pattern | Description | Use Cases |
|---------|-------------|-----------|
| `chain_of_thought` | Step-by-step reasoning | Complex problems, debugging, math |
| `tree_of_thought` | Multi-approach exploration | Design decisions, architecture |
| `self_consistency` | Multiple reasoning paths | Verification, critical decisions |

### Structure Patterns

Patterns that define format and persona.

| Pattern | Description | Use Cases |
|---------|-------------|-----------|
| `role_definition` | Define AI role and constraints | Chatbots, specialized assistants |
| `output_format` | Specify output structure | APIs, data extraction |
| `few_shot_examples` | Learning from examples | Format teaching, style matching |

### Safety Patterns

Patterns for responsible AI behavior.

| Pattern | Description | Use Cases |
|---------|-------------|-----------|
| `safety_boundaries` | Ethical guidelines | Content moderation |
| `uncertainty_acknowledgment` | Honest uncertainty | Factual queries, advice |

### Task Patterns

Patterns for specific task types.

| Pattern | Description | Use Cases |
|---------|-------------|-----------|
| `code_generation` | Structured code writing | Functions, features |
| `code_review` | Comprehensive review | Quality assurance |
| `summarization` | Content summarization | Documents, meetings |
| `analysis` | Analytical framework | Business, technical analysis |

### Meta Patterns

Patterns about the AI's own behavior.

| Pattern | Description | Use Cases |
|---------|-------------|-----------|
| `self_reflection` | Self-checking | Complex questions |
| `clarification_request` | Ask for clarity | Ambiguous requests |
| `iterative_refinement` | Build through iteration | Creative, quality-critical work |

## Pattern Details

### Chain of Thought

```rust
let prompt = manager.build_prompt("chain_of_thought")?
    .set("task", "Calculate the compound interest on $1000 at 5% for 3 years")
    .build()?;
```

**Template:**
```
Think through this problem step by step:

1. First, understand what is being asked
2. Identify the key components and constraints
3. Break down the problem into smaller parts
4. Solve each part systematically
5. Combine the solutions
6. Verify the final answer

{task}

Let me work through this step by step...
```

### Role Definition

```rust
let prompt = manager.build_prompt("role_definition")?
    .set("role", "senior software architect")
    .set("expertise", "distributed systems and cloud architecture")
    .set("responsibilities", "- Designing scalable systems\n- Reviewing architecture decisions")
    .set("response_style", "detailed and technical")
    .set("focus_areas", "scalability and maintainability")
    .build()?;
```

### Code Generation

```rust
let prompt = manager.build_prompt("code_generation")?
    .set("language", "Rust")
    .set("task", "Create a function to validate email addresses")
    .set("requirements", "- Use regex for validation\n- Return Result type\n- Include unit tests")
    .build()?;
```

### Code Review

```rust
let prompt = manager.build_prompt("code_review")?
    .set("language", "Python")
    .set("code", r#"
def process_data(items):
    result = []
    for item in items:
        if item > 0:
            result.append(item * 2)
    return result
"#)
    .build()?;
```

## Template System

### Creating Templates

```rust
use goose::prompts::{Template, TemplateVariable, VariableType};

let template = Template::new("greeting", "Hello {name}, welcome to {place}!")
    .with_description("A greeting template")
    .with_variable(
        TemplateVariable::new("name")
            .required()
            .with_description("Person's name")
    )
    .with_variable(
        TemplateVariable::new("place")
            .with_default("Goose")
            .with_description("Location name")
    );
```

### Variable Types

```rust
pub enum VariableType {
    String,   // Default, any text
    Number,   // Numeric values
    Boolean,  // true/false, yes/no
    Array,    // Comma-separated list
    Object,   // JSON object
    Any,      // No validation
}
```

### Rendering Templates

```rust
let engine = TemplateEngine::default();

let mut vars = HashMap::new();
vars.insert("name".to_string(), "Alice".to_string());

let result = engine.render("greeting", &vars)?;
// "Hello Alice, welcome to Goose!"
```

## Pattern Composition

Combine patterns for comprehensive prompts:

```rust
// Compose patterns
let prompt = manager.compose_patterns(&[
    "role_definition",
    "chain_of_thought",
    "safety_boundaries",
    "output_format"
])?;

// Or use builder
let prompt = manager.build_prompt("role_definition")?
    .set("role", "helpful coding assistant")
    .set("expertise", "Python and data science")
    .with_suffix(manager.get_pattern("chain_of_thought")?.content)
    .build()?;
```

## Custom Patterns

### Registering Custom Patterns

```rust
use goose::prompts::{Pattern, PatternCategory};

let custom = Pattern::new(
    "my_pattern",
    "Custom prompt content with {variable}"
)
.with_category(PatternCategory::Custom)
.with_description("My custom pattern")
.with_required_variable("variable")
.with_use_case("Specific use case");

manager.registry().register(custom);
```

### Pattern Categories

```rust
let reasoning = manager.get_patterns_by_category(PatternCategory::Reasoning);
let safety = manager.get_patterns_by_category(PatternCategory::Safety);
```

## Configuration

### PromptConfig

```rust
pub struct PromptConfig {
    /// Enable prompt caching
    pub cache_enabled: bool,

    /// Maximum cache size
    pub max_cache_size: usize,

    /// Default template directory
    pub template_dir: Option<String>,

    /// Enable validation of generated prompts
    pub validation_enabled: bool,

    /// Maximum prompt length (characters)
    pub max_prompt_length: usize,
}
```

## Best Practices

### 1. Start with Reasoning

For complex tasks, begin with a reasoning pattern:

```rust
let prompt = manager.compose_patterns(&[
    "chain_of_thought",  // Encourages step-by-step thinking
    "code_generation",   // Then the specific task
])?;
```

### 2. Define Clear Boundaries

Always include safety patterns for production:

```rust
let prompt = manager.compose_patterns(&[
    "role_definition",
    "safety_boundaries",
    "task_pattern",
])?;
```

### 3. Use Examples for Format

When you need specific output format:

```rust
let prompt = manager.build_prompt("few_shot_examples")?
    .set("task", "Extract entities from text")
    .set("example_1_input", "John works at Google")
    .set("example_1_output", r#"{"name": "John", "company": "Google"}"#)
    .set("example_2_input", "Sarah is a doctor in Boston")
    .set("example_2_output", r#"{"name": "Sarah", "profession": "doctor", "location": "Boston"}"#)
    .build()?;
```

### 4. Acknowledge Uncertainty

For factual queries:

```rust
let prompt = manager.compose_patterns(&[
    "uncertainty_acknowledgment",
    "your_task_pattern",
])?;
```

### 5. Request Clarification

For ambiguous requests:

```rust
let prompt = manager.build_prompt("clarification_request")?
    .set("topic", "API integration requirements")
    .set("questions", "1. Which API version?\n2. Authentication method?\n3. Rate limits?")
    .set("assumptions", "- REST API\n- JSON format")
    .build()?;
```

## Statistics

```rust
let stats = manager.get_stats();
println!("Total patterns: {}", stats.total_patterns);
println!("Templates loaded: {}", stats.templates_loaded);

for (category, count) in stats.patterns_by_category {
    println!("{:?}: {} patterns", category, count);
}
```

## Testing

```bash
# Run prompts unit tests
cargo test --package goose prompts::

# Run integration tests
cargo test --package goose --test prompts_integration_test
```

## See Also

- [Enterprise Integration Action Plan](07_ENTERPRISE_INTEGRATION_ACTION_PLAN.md)
- [Comprehensive Audit Report](08_COMPREHENSIVE_AUDIT_REPORT.md)
- [Prompt Engineering Guide](https://docs.anthropic.com/claude/docs/prompt-engineering)
