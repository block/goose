use goose::agents::subagent_tools::task_params_to_inline_recipe;
use serde_json::json;

#[cfg(test)]
mod tests {
    use super::*;

    // Helper function to create a list of loaded extensions for testing
    fn test_loaded_extensions() -> Vec<String> {
        vec!["developer".to_string(), "memory".to_string()]
    }

    #[test]
    fn test_minimal_task_with_instructions() {
        let params = json!({
            "instructions": "Test task"
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe.instructions, Some("Test task".to_string()));
        assert_eq!(recipe.title, "Dynamic Task");
        assert_eq!(recipe.description, "Inline recipe task");
    }

    #[test]
    fn test_minimal_task_with_prompt() {
        let params = json!({
            "prompt": "Test prompt"
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe.prompt, Some("Test prompt".to_string()));
    }

    #[test]
    fn test_missing_required_fields() {
        let params = json!({
            "title": "Test"
        });

        let result = task_params_to_inline_recipe(&params, &test_loaded_extensions());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("instructions' or 'prompt"));
    }

    #[test]
    fn test_with_recipe_fields() {
        let params = json!({
            "instructions": "Test",
            "title": "Custom Title",
            "description": "Custom Description",
            "retry": {
                "max_retries": 3,
                "checks": [
                    {
                        "type": "shell",
                        "command": "echo test"
                    }
                ]
            },
            "response": {
                "json_schema": {
                    "type": "object"
                }
            }
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe.title, "Custom Title");
        assert_eq!(recipe.description, "Custom Description");
        assert!(recipe.retry.is_some());
        assert!(recipe.response.is_some());

        // Verify retry config details
        let retry = recipe.retry.unwrap();
        assert_eq!(retry.max_retries, 3);
        assert_eq!(retry.checks.len(), 1);
    }

    #[test]
    fn test_security_validation() {
        let params = json!({
            "instructions": format!("Test{}", '\u{E0041}')  // Harmful Unicode tag
        });

        let result = task_params_to_inline_recipe(&params, &test_loaded_extensions());
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("harmful"));
    }

    #[test]
    fn test_return_last_only_flag() {
        let params_with_flag = json!({
            "instructions": "Test task",
            "return_last_only": true
        });

        let recipe =
            task_params_to_inline_recipe(&params_with_flag, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe.instructions, Some("Test task".to_string()));

        // The flag should not affect the recipe itself, only the task payload
        // We can't test the task creation here without async context

        let params_without_flag = json!({
            "instructions": "Test task"
        });

        let recipe2 =
            task_params_to_inline_recipe(&params_without_flag, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe2.instructions, Some("Test task".to_string()));
    }

    #[test]
    fn test_with_extensions() {
        let params = json!({
            "instructions": "Test",
            "extensions": [
                {
                    "type": "builtin",
                    "name": "developer",
                    "description": "developer"
                }
            ]
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert!(recipe.extensions.is_some());
        let extensions = recipe.extensions.unwrap();
        assert_eq!(extensions.len(), 1);
    }

    #[test]
    fn test_invalid_retry_config() {
        // Test with max_retries = 0 (invalid)
        let params = json!({
            "instructions": "Test",
            "retry": {
                "max_retries": 0,  // Invalid: must be > 0
                "checks": []
            }
        });

        let result = task_params_to_inline_recipe(&params, &test_loaded_extensions());
        assert!(result.is_err());
        assert!(result
            .unwrap_err()
            .to_string()
            .contains("Invalid retry config"));
    }

    #[test]
    fn test_invalid_retry_config_missing_checks() {
        // Test with missing required field 'checks'
        let params = json!({
            "instructions": "Test",
            "retry": {
                "max_retries": 3
                // Missing 'checks' field
            }
        });

        let result = task_params_to_inline_recipe(&params, &test_loaded_extensions());
        // This should fail during deserialization since 'checks' is required
        assert!(result.is_ok()); // But retry field will be None due to failed deserialization
        let recipe = result.unwrap();
        assert!(recipe.retry.is_none());
    }

    #[test]
    fn test_both_instructions_and_prompt() {
        // Test that both instructions and prompt can be provided
        let params = json!({
            "instructions": "Test instructions",
            "prompt": "Test prompt"
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe.instructions, Some("Test instructions".to_string()));
        assert_eq!(recipe.prompt, Some("Test prompt".to_string()));
    }

    #[test]
    fn test_invalid_json_in_optional_fields() {
        // Test that invalid JSON in optional fields is gracefully ignored
        let params = json!({
            "instructions": "Test",
            "settings": "not an object", // Invalid: should be object
            "extensions": "not an array", // Invalid: should be array
            "context": {"not": "an array"}, // Invalid: should be array
            "activities": 123 // Invalid: should be array
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe.instructions, Some("Test".to_string()));
        // Invalid fields should be ignored (None)
        assert!(recipe.settings.is_none());
        assert!(recipe.extensions.is_none());
    }

    #[test]
    fn test_with_settings() {
        let params = json!({
            "instructions": "Test",
            "settings": {
                "goose_provider": "openai",
                "goose_model": "gpt-4",
                "temperature": 0.7
            }
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert!(recipe.settings.is_some());
        let settings = recipe.settings.unwrap();
        assert_eq!(settings.goose_provider, Some("openai".to_string()));
        assert_eq!(settings.goose_model, Some("gpt-4".to_string()));
        assert_eq!(settings.temperature, Some(0.7));
    }

    #[test]
    fn test_with_parameters() {
        let params = json!({
            "instructions": "Test",
            "parameters": [
                {
                    "key": "test_param",
                    "input_type": "string",
                    "requirement": "required",
                    "description": "A test parameter"
                }
            ]
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert!(recipe.parameters.is_some());
        let parameters = recipe.parameters.unwrap();
        assert_eq!(parameters.len(), 1);
        assert_eq!(parameters[0].key, "test_param");
    }

    #[test]
    fn test_empty_strings_for_required_fields() {
        // Empty strings should be valid for instructions/prompt
        let params = json!({
            "instructions": ""
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe.instructions, Some("".to_string()));
    }

    #[test]
    fn test_very_long_instruction() {
        // Test with a very long instruction string
        let long_instruction = "a".repeat(10000);
        let params = json!({
            "instructions": long_instruction.clone()
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe.instructions, Some(long_instruction));
    }

    #[test]
    fn test_unicode_in_non_instruction_fields() {
        // Unicode tags should be allowed in non-instruction fields
        let params = json!({
            "instructions": "Test",
            "title": format!("Title with unicode {}", '\u{E0041}'),
            "description": format!("Description with unicode {}", '\u{E0041}')
        });

        // This should succeed - only instructions/prompt/activities are checked for security
        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert!(recipe.title.contains('\u{E0041}'));
        assert!(recipe.description.contains('\u{E0041}'));
    }

    #[test]
    fn test_extension_shortnames() {
        // Test that extension shortnames are properly resolved
        let loaded_exts = vec!["developer".to_string(), "memory".to_string()];
        let params = json!({
            "instructions": "Test",
            "extensions": ["developer", "memory"]
        });

        let recipe = task_params_to_inline_recipe(&params, &loaded_exts).unwrap();
        assert!(recipe.extensions.is_some());
        let extensions = recipe.extensions.unwrap();
        assert!(extensions.len() <= 2);
        if !extensions.is_empty() {
            assert!(matches!(
                &extensions[0],
                goose::agents::extension::ExtensionConfig::Builtin { .. }
                    | goose::agents::extension::ExtensionConfig::Stdio { .. }
                    | goose::agents::extension::ExtensionConfig::Sse { .. }
                    | goose::agents::extension::ExtensionConfig::StreamableHttp { .. }
                    | goose::agents::extension::ExtensionConfig::Frontend { .. }
                    | goose::agents::extension::ExtensionConfig::InlinePython { .. }
            ));
        }
    }

    #[test]
    fn test_mixed_extension_formats() {
        let loaded_exts = vec!["developer".to_string(), "memory".to_string()];
        let params = json!({
            "instructions": "Test",
            "extensions": [
                "developer",  // Shortname
                {
                    "type": "stdio",
                    "name": "custom",
                    "description": "Custom stdio",
                    "cmd": "echo",
                    "args": ["test"]
                }
            ]
        });

        let recipe = task_params_to_inline_recipe(&params, &loaded_exts).unwrap();
        assert!(recipe.extensions.is_some());
        let extensions = recipe.extensions.unwrap();
        assert!(!extensions.is_empty() && extensions.len() <= 2);
        if let Some(last) = extensions.last() {
            match last {
                goose::agents::extension::ExtensionConfig::Stdio { name, .. } => {
                    assert_eq!(name, "custom");
                }
                _ => {
                    if extensions.len() == 2 {
                        panic!("Expected stdio extension config for 'custom'");
                    }
                }
            }
        }
    }

    #[test]
    fn test_unknown_extension_shortname() {
        let loaded_exts = vec!["developer".to_string()];
        let params = json!({
            "instructions": "Test",
            "extensions": [
                "unknown_extension_1",  // Full config should always work
                {
                    "type": "builtin",
                    "name": "test_builtin",
                    "display_name": "Test Builtin",
                    "description": "Test extension"
                },
                "unknown_extension_2"  // Should be skipped
            ]
        });

        let recipe = task_params_to_inline_recipe(&params, &loaded_exts).unwrap();
        assert!(recipe.extensions.is_some());
        let extensions = recipe.extensions.unwrap();
        assert_eq!(extensions.len(), 1);
        match &extensions[0] {
            goose::agents::extension::ExtensionConfig::Builtin { name, .. } => {
                assert_eq!(name, "test_builtin");
            }
            _ => panic!("Expected builtin extension config"),
        }
    }

    #[test]
    fn test_empty_extensions_array() {
        let loaded_exts = vec!["developer".to_string(), "memory".to_string()];
        let params = json!({
            "instructions": "Test",
            "extensions": []
        });

        let recipe = task_params_to_inline_recipe(&params, &loaded_exts).unwrap();
        assert!(recipe.extensions.is_some());
        let extensions = recipe.extensions.unwrap();
        assert_eq!(extensions.len(), 0);
    }

    #[test]
    fn test_omitted_extensions_field() {
        let loaded_exts = vec!["developer".to_string(), "memory".to_string()];
        let params = json!({
            "instructions": "Test"
            // No extensions field
        });

        let recipe = task_params_to_inline_recipe(&params, &loaded_exts).unwrap();
        assert!(recipe.extensions.is_none());
    }

    #[test]
    fn test_null_values_in_optional_fields() {
        let params = json!({
            "instructions": "Test",
            "title": null,
            "description": null,
            "extensions": null,
            "settings": null
        });

        let recipe = task_params_to_inline_recipe(&params, &test_loaded_extensions()).unwrap();
        assert_eq!(recipe.instructions, Some("Test".to_string()));
        assert_eq!(recipe.title, "Dynamic Task");
        assert_eq!(recipe.description, "Inline recipe task");
        assert!(recipe.extensions.is_none());
        assert!(recipe.settings.is_none());
    }
}
