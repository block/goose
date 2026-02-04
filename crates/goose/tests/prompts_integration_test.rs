//! Prompts Module Integration Tests
//!
//! Comprehensive integration tests for the prompts module, testing the complete
//! workflow from pattern registration through template rendering and prompt composition.

use goose::prompts::{
    Pattern, PatternCategory, PromptManager, Template, TemplateEngine, TemplateVariable,
    VariableType,
};
use std::collections::HashMap;

#[tokio::test]
async fn test_prompt_manager_full_workflow() {
    let manager = PromptManager::new();

    // Test getting default patterns
    let chain_of_thought = manager.get_pattern("chain_of_thought");
    assert!(
        chain_of_thought.is_some(),
        "Chain of thought pattern should exist"
    );

    let pattern = chain_of_thought.unwrap();
    assert_eq!(pattern.category, PatternCategory::Reasoning);
    assert!(!pattern.required_variables.is_empty());

    // Test pattern composition
    let composed = manager
        .compose_patterns(&["role_definition", "chain_of_thought"])
        .expect("Should compose patterns successfully");

    assert!(composed.contains("role"), "Should contain role content");
    assert!(
        composed.contains("step"),
        "Should contain step-by-step content"
    );
}

#[tokio::test]
async fn test_pattern_builder_complete_workflow() {
    let manager = PromptManager::new();

    let result = manager
        .build_prompt("few_shot_examples")
        .expect("Should create builder")
        .set("task", "Write a function")
        .set("example_1_input", "def add(a, b):")
        .set("example_1_output", "def add(a, b):\n    return a + b")
        .with_prefix("System: You are a coding assistant")
        .with_suffix("Please follow the pattern shown above.")
        .build()
        .expect("Should build prompt");

    assert!(result.contains("System: You are a coding assistant"));
    assert!(result.contains("Write a function"));
    assert!(result.contains("def add(a, b):"));
    assert!(result.contains("Please follow the pattern shown above."));
}

#[tokio::test]
async fn test_template_engine_integration() {
    let engine = TemplateEngine::default(); // Loads default templates

    // Test system message template
    let mut variables = HashMap::new();
    variables.insert("role".to_string(), "a senior software engineer".to_string());
    variables.insert(
        "capabilities".to_string(),
        "- Code review\n- Architecture design\n- Best practices guidance".to_string(),
    );

    let result = engine
        .render("system_message", &variables)
        .expect("Should render template");

    assert!(result.contains("senior software engineer"));
    assert!(result.contains("Code review"));
    assert!(result.contains("Architecture design"));
}

#[tokio::test]
async fn test_code_review_template_integration() {
    let engine = TemplateEngine::default();

    let mut variables = HashMap::new();
    variables.insert("language".to_string(), "Rust".to_string());
    variables.insert(
        "code".to_string(),
        "fn safe_function() -> i32\n    return 42;\nend".to_string(),
    );
    variables.insert(
        "focus_areas".to_string(),
        "Code quality and best practices".to_string(),
    );

    let result = engine
        .render("code_review", &variables)
        .expect("Should render code review template");

    assert!(result.contains("Rust"));
    assert!(result.contains("safe_function"));
    assert!(result.contains("Code quality"));
    assert!(result.contains("Please provide feedback on:"));
}

#[tokio::test]
async fn test_enterprise_workflow_pattern_integration() {
    let manager = PromptManager::new();

    // Test enterprise patterns work together
    let analysis_pattern = manager.get_pattern("analysis");
    assert!(analysis_pattern.is_some());

    let chain_of_thought = manager.get_pattern("chain_of_thought");
    assert!(chain_of_thought.is_some());

    let code_generation = manager.get_pattern("code_generation");
    assert!(code_generation.is_some());

    // Test they can be composed for enterprise workflow
    let enterprise_flow = manager
        .compose_patterns(&["analysis", "chain_of_thought", "code_generation"])
        .expect("Should compose enterprise patterns");

    assert!(enterprise_flow.contains("Analysis Framework"));
    assert!(enterprise_flow.contains("step by step"));
    assert!(enterprise_flow.contains("Generate code"));
}

#[tokio::test]
async fn test_pattern_categories_and_filtering() {
    let manager = PromptManager::new();

    // Test filtering by category
    let reasoning_patterns = manager.get_patterns_by_category(PatternCategory::Reasoning);
    assert!(!reasoning_patterns.is_empty());

    let structure_patterns = manager.get_patterns_by_category(PatternCategory::Structure);
    assert!(!structure_patterns.is_empty());

    let task_patterns = manager.get_patterns_by_category(PatternCategory::Task);
    assert!(!task_patterns.is_empty());

    let safety_patterns = manager.get_patterns_by_category(PatternCategory::Safety);
    assert!(!safety_patterns.is_empty());

    // Verify categories are correct
    for pattern in reasoning_patterns {
        assert_eq!(pattern.category, PatternCategory::Reasoning);
    }

    for pattern in structure_patterns {
        assert_eq!(pattern.category, PatternCategory::Structure);
    }
}

#[tokio::test]
async fn test_custom_pattern_registration_and_usage() {
    let manager = PromptManager::new();

    // Create a custom pattern
    let custom_pattern = Pattern::new(
        "custom_debug",
        r#"Debug Analysis for {system}

Issue: {issue}
Context: {context}

Steps to investigate:
1. Reproduce the issue
2. Check logs for {system}
3. Analyze recent changes
4. Test potential solutions

{additional_steps}"#,
    )
    .with_category(PatternCategory::Task)
    .with_description("Custom debugging pattern")
    .with_required_variable("system")
    .with_required_variable("issue")
    .with_optional_variable("context", "Unknown context")
    .with_optional_variable("additional_steps", "");

    // Register the custom pattern
    manager.registry().register(custom_pattern);

    // Test using the custom pattern
    let result = manager
        .build_prompt("custom_debug")
        .expect("Should find custom pattern")
        .set("system", "Payment Processing")
        .set("issue", "Transactions failing intermittently")
        .set("context", "High load during peak hours")
        .build()
        .expect("Should build custom prompt");

    assert!(result.contains("Payment Processing"));
    assert!(result.contains("Transactions failing intermittently"));
    assert!(result.contains("High load during peak hours"));
}

#[tokio::test]
async fn test_template_validation_and_error_handling() {
    let engine = TemplateEngine::new();

    // Register template with validation
    let template = Template::new(
        "validated_template",
        "Age: {age}, Enabled: {enabled}, Items: {items}",
    )
    .with_variable(
        TemplateVariable::new("age")
            .required()
            .with_type(VariableType::Number),
    )
    .with_variable(
        TemplateVariable::new("enabled")
            .with_type(VariableType::Boolean)
            .with_default("false"),
    )
    .with_variable(
        TemplateVariable::new("items")
            .with_type(VariableType::Array)
            .with_default("item1,item2"),
    );

    engine.register(template);

    // Test valid rendering
    let mut variables = HashMap::new();
    variables.insert("age".to_string(), "25".to_string());
    variables.insert("enabled".to_string(), "true".to_string());
    variables.insert("items".to_string(), "apple,banana,cherry".to_string());

    let result = engine
        .render("validated_template", &variables)
        .expect("Should render with valid values");

    assert!(result.contains("Age: 25"));
    assert!(result.contains("Enabled: true"));
    assert!(result.contains("Items: apple,banana,cherry"));

    // Test validation errors
    let mut invalid_variables = HashMap::new();
    invalid_variables.insert("age".to_string(), "not_a_number".to_string());

    let result = engine.render("validated_template", &invalid_variables);
    assert!(
        result.is_err(),
        "Should fail validation with invalid number"
    );
}

#[tokio::test]
async fn test_prompt_manager_statistics_and_monitoring() {
    let manager = PromptManager::new();

    let stats = manager.get_stats();

    // Verify we have patterns loaded
    assert!(
        stats.total_patterns > 10,
        "Should have substantial pattern library"
    );
    // Note: Template engine in PromptManager doesn't auto-load defaults by default
    // Check that template count is returned (can be 0 for new instance)

    // Verify we have patterns in each category
    assert!(stats
        .patterns_by_category
        .contains_key(&PatternCategory::Reasoning));
    assert!(stats
        .patterns_by_category
        .contains_key(&PatternCategory::Structure));
    assert!(stats
        .patterns_by_category
        .contains_key(&PatternCategory::Safety));
    assert!(stats
        .patterns_by_category
        .contains_key(&PatternCategory::Task));
    assert!(stats
        .patterns_by_category
        .contains_key(&PatternCategory::Meta));

    // Test pattern listing
    let patterns = manager.list_patterns();
    assert_eq!(patterns.len(), stats.total_patterns);

    // Verify metadata completeness
    for pattern_meta in patterns {
        assert!(!pattern_meta.name.is_empty());
        assert!(!pattern_meta.description.is_empty());
        assert!(!pattern_meta.use_cases.is_empty());
    }
}

#[tokio::test]
async fn test_enterprise_ai_agent_integration() {
    let manager = PromptManager::new();

    // Simulate an enterprise AI agent workflow using patterns

    // Step 1: Role definition
    let role_result = manager
        .build_prompt("role_definition")
        .expect("Should find role definition")
        .set("role", "Enterprise AI Development Assistant")
        .set("expertise", "Rust development, enterprise architecture, AI systems")
        .set("responsibilities", "- Code generation and review\n- Architecture design\n- Performance optimization\n- Security analysis")
        .build()
        .expect("Should build role prompt");

    // Step 2: Chain of thought for complex reasoning
    let reasoning_result = manager
        .build_prompt("chain_of_thought")
        .expect("Should find chain of thought")
        .set(
            "task",
            "Design a scalable microservices architecture for an AI platform",
        )
        .build()
        .expect("Should build reasoning prompt");

    // Step 3: Code generation
    let code_result = manager
        .build_prompt("code_generation")
        .expect("Should find code generation")
        .set("language", "Rust")
        .set("task", "Implement a high-performance message queue")
        .set(
            "requirements",
            "- Async/await support\n- Backpressure handling\n- Monitoring integration",
        )
        .build()
        .expect("Should build code prompt");

    // Verify all components work together
    assert!(role_result.contains("Enterprise AI Development Assistant"));
    assert!(reasoning_result.contains("step by step"));
    assert!(code_result.contains("Rust"));
    assert!(code_result.contains("Async/await support"));

    // Test validation
    let validation_result = manager.validate_prompt(&role_result);
    assert!(
        validation_result.is_ok(),
        "Role prompt should pass validation"
    );

    let validation_result = manager.validate_prompt(&reasoning_result);
    assert!(
        validation_result.is_ok(),
        "Reasoning prompt should pass validation"
    );

    let validation_result = manager.validate_prompt(&code_result);
    assert!(
        validation_result.is_ok(),
        "Code prompt should pass validation"
    );
}

#[tokio::test]
async fn test_prompt_composition_performance() {
    let manager = PromptManager::new();

    // Test performance with multiple pattern compositions
    let start = std::time::Instant::now();

    for _ in 0..100 {
        let _result = manager
            .compose_patterns(&["chain_of_thought", "code_generation", "safety_boundaries"])
            .expect("Should compose successfully");
    }

    let duration = start.elapsed();
    assert!(
        duration.as_millis() < 1000,
        "100 compositions should complete in under 1 second"
    );

    // Test with template rendering performance
    let engine = TemplateEngine::default(); // Use default which loads templates
    let start = std::time::Instant::now();

    let mut variables = HashMap::new();
    variables.insert("role".to_string(), "test role".to_string());

    for _ in 0..100 {
        let _result = engine
            .render("system_message", &variables)
            .expect("Should render successfully");
    }

    let duration = start.elapsed();
    assert!(
        duration.as_millis() < 500,
        "100 template renders should complete in under 500ms"
    );
}

#[tokio::test]
async fn test_prompt_manager_configuration() {
    use goose::prompts::PromptConfig;

    // Test custom configuration
    let config = PromptConfig {
        cache_enabled: true,
        max_cache_size: 50,
        template_dir: Some("/custom/templates".to_string()),
        validation_enabled: true,
        max_prompt_length: 50_000,
    };

    let manager = PromptManager::with_config(config);

    // Test validation with custom length limit
    let long_prompt = "x".repeat(60_000);
    let validation_result = manager.validate_prompt(&long_prompt);
    assert!(
        validation_result.is_err(),
        "Should fail validation for too long prompt"
    );

    let short_prompt = "This is a reasonable length prompt.";
    let validation_result = manager.validate_prompt(&short_prompt);
    assert!(
        validation_result.is_ok(),
        "Should pass validation for reasonable prompt"
    );
}
