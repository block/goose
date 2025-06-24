#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use goose::recipe::{RecipeParameterInputType, RecipeParameterRequirement};
    use tempfile::TempDir;

    use crate::recipes::recipe::load_recipe_as_template;

    fn setup_recipe_file(instructions_and_parameters: &str) -> (TempDir, PathBuf) {
        let recipe_content = format!(
            r#"{{
            "version": "1.0.0",
            "title": "Test Recipe",
            "description": "A test recipe",
            {}
        }}"#,
            instructions_and_parameters
        );
        let temp_dir = tempfile::tempdir().unwrap();
        let recipe_path: std::path::PathBuf = temp_dir.path().join("test_recipe.json");

        std::fs::write(&recipe_path, recipe_content).unwrap();
        (temp_dir, recipe_path)
    }

    mod load_recipe_as_template_tests {
        use super::*;
        #[test]
        fn test_load_recipe_as_template_success() {
            let instructions_and_parameters = r#"
                "instructions": "Test instructions with {{ my_name }}",
                "parameters": [
                    {
                        "key": "my_name",
                        "input_type": "string",
                        "requirement": "required",
                        "description": "A test parameter"
                    }
                ]"#;

            let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

            let params = vec![("my_name".to_string(), "value".to_string())];
            let recipe = load_recipe_as_template(recipe_path.to_str().unwrap(), params).unwrap();

            assert_eq!(recipe.title, "Test Recipe");
            assert_eq!(recipe.description, "A test recipe");
            assert_eq!(recipe.instructions.unwrap(), "Test instructions with value");
            // Verify parameters match recipe definition
            assert_eq!(recipe.parameters.as_ref().unwrap().len(), 1);
            let param = &recipe.parameters.as_ref().unwrap()[0];
            assert_eq!(param.key, "my_name");
            assert!(matches!(param.input_type, RecipeParameterInputType::String));
            assert!(matches!(
                param.requirement,
                RecipeParameterRequirement::Required
            ));
            assert_eq!(param.description, "A test parameter");
        }

        #[test]
        fn test_load_recipe_as_template_success_variable_in_prompt() {
            let instructions_and_parameters = r#"
                "instructions": "Test instructions",
                "prompt": "My prompt {{ my_name }}",
                "parameters": [
                    {
                        "key": "my_name",
                        "input_type": "string",
                        "requirement": "required",
                        "description": "A test parameter"
                    }
                ]"#;

            let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

            let params = vec![("my_name".to_string(), "value".to_string())];
            let recipe = load_recipe_as_template(recipe_path.to_str().unwrap(), params).unwrap();

            assert_eq!(recipe.title, "Test Recipe");
            assert_eq!(recipe.description, "A test recipe");
            assert_eq!(recipe.instructions.unwrap(), "Test instructions");
            assert_eq!(recipe.prompt.unwrap(), "My prompt value");
            let param = &recipe.parameters.as_ref().unwrap()[0];
            assert_eq!(param.key, "my_name");
            assert!(matches!(param.input_type, RecipeParameterInputType::String));
            assert!(matches!(
                param.requirement,
                RecipeParameterRequirement::Required
            ));
            assert_eq!(param.description, "A test parameter");
        }

        #[test]
        fn test_load_recipe_as_template_wrong_parameters_in_recipe_file() {
            let instructions_and_parameters = r#"
                "instructions": "Test instructions with {{ expected_param1 }} {{ expected_param2 }}",
                "parameters": [
                    {
                        "key": "wrong_param_key",
                        "input_type": "string",
                        "requirement": "required",
                        "description": "A test parameter"
                    }
                ]"#;
            let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

            let load_recipe_result =
                load_recipe_as_template(recipe_path.to_str().unwrap(), Vec::new());
            assert!(load_recipe_result.is_err());
            let err = load_recipe_result.unwrap_err();
            println!("{}", err.to_string());
            assert!(err
                .to_string()
                .contains("Unnecessary parameter definitions: wrong_param_key."));
            assert!(err
                .to_string()
                .contains("Missing definitions for parameters in the recipe file:"));
            assert!(err.to_string().contains("expected_param1"));
            assert!(err.to_string().contains("expected_param2"));
        }

        #[test]
        fn test_load_recipe_as_template_with_default_values_in_recipe_file() {
            let instructions_and_parameters = r#"
                "instructions": "Test instructions with {{ param_with_default }} {{ param_without_default }}",
                "parameters": [
                    {
                        "key": "param_with_default",
                        "input_type": "string",
                        "requirement": "optional",
                        "default": "my_default_value",
                        "description": "A test parameter"
                    },
                    {
                        "key": "param_without_default",
                        "input_type": "string",
                        "requirement": "required",
                        "description": "A test parameter"
                    }
                ]"#;
            let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);
            let params = vec![("param_without_default".to_string(), "value1".to_string())];

            let recipe = load_recipe_as_template(recipe_path.to_str().unwrap(), params).unwrap();

            assert_eq!(recipe.title, "Test Recipe");
            assert_eq!(recipe.description, "A test recipe");
            assert_eq!(
                recipe.instructions.unwrap(),
                "Test instructions with my_default_value value1"
            );
        }

        #[test]
        fn test_load_recipe_as_template_optional_parameters_with_empty_default_values_in_recipe_file(
        ) {
            let instructions_and_parameters = r#"
                "instructions": "Test instructions with {{ optional_param }}",
                "parameters": [
                    {
                        "key": "optional_param",
                        "input_type": "string",
                        "requirement": "optional",
                        "description": "A test parameter",
                        "default": "",
                    }
                ]"#;
            let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

            let recipe =
                load_recipe_as_template(recipe_path.to_str().unwrap(), Vec::new()).unwrap();
            assert_eq!(recipe.title, "Test Recipe");
            assert_eq!(recipe.description, "A test recipe");
            assert_eq!(recipe.instructions.unwrap(), "Test instructions with ");
        }

        #[test]
        fn test_load_recipe_as_template_optional_parameters_without_default_values_in_recipe_file()
        {
            let instructions_and_parameters = r#"
                "instructions": "Test instructions with {{ optional_param }}",
                "parameters": [
                    {
                        "key": "optional_param",
                        "input_type": "string",
                        "requirement": "optional",
                        "description": "A test parameter"
                    }
                ]"#;
            let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

            let load_recipe_result =
                load_recipe_as_template(recipe_path.to_str().unwrap(), Vec::new());
            assert!(load_recipe_result.is_err());
            let err = load_recipe_result.unwrap_err();
            println!("{}", err.to_string());
            assert!(err.to_string().to_lowercase().contains("missing"));
        }

        #[test]
        fn test_load_recipe_as_template_wrong_input_type_in_recipe_file() {
            let instructions_and_parameters = r#"
                "instructions": "Test instructions with {{ param }}",
                "parameters": [
                    {
                        "key": "param",
                        "input_type": "some_invalid_type",
                        "requirement": "required",
                        "description": "A test parameter"
                    }
                ]"#;
            let params = vec![("param".to_string(), "value".to_string())];
            let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

            let load_recipe_result = load_recipe_as_template(recipe_path.to_str().unwrap(), params);
            assert!(load_recipe_result.is_err());
            let err = load_recipe_result.unwrap_err();
            let err_msg = err.to_string();
            eprint!("Error: {}", err_msg);
            assert!(err_msg.contains("unknown variant `some_invalid_type`"));
        }

        #[test]
        fn test_load_recipe_as_template_success_without_parameters() {
            let instructions_and_parameters = r#"
                "instructions": "Test instructions"
                "#;
            let (_temp_dir, recipe_path) = setup_recipe_file(instructions_and_parameters);

            let recipe =
                load_recipe_as_template(recipe_path.to_str().unwrap(), Vec::new()).unwrap();
            assert_eq!(recipe.instructions.unwrap(), "Test instructions");
            assert!(recipe.parameters.is_none());
        }

        #[test]
        fn test_template_inheritance() {
            let temp_dir = tempfile::tempdir().unwrap();
            let temp_path = temp_dir.path();
            let parent_content = r#"
                version: 1.0.0
                title: Parent
                description: Parent recipe
                prompt: |
                    show me the news for day: {{ date }}
                    {% block prompt -%}
                    What is the capital of France?
                    {%- endblock %}
                    {% if is_enabled %}
                        Feature is enabled.
                    {% else %}
                        Feature is disabled.
                    {% endif %}
                parameters:
                    - key: date
                      input_type: string
                      requirement: required
                      description: date specified by the user
                    - key: is_enabled
                      input_type: boolean
                      requirement: required
                      description: whether the feature is enabled
            "#;

            let parent_path = temp_path.join("parent.yaml");
            std::fs::write(&parent_path, parent_content).unwrap();
            let child_content = r#"
                {% extends "parent.yaml" -%}
                {% block prompt -%}
                What is the capital of Germany?
                {%- endblock %}
            "#;
            let child_path = temp_path.join("child.yaml");
            std::fs::write(&child_path, child_content).unwrap();

            let params = vec![
                ("date".to_string(), "today".to_string()),
                ("is_enabled".to_string(), "true".to_string()),
            ];
            let parent_result =
                load_recipe_as_template(parent_path.to_str().unwrap(), params.clone());
            assert!(parent_result.is_ok());
            let parent_recipe = parent_result.unwrap();
            assert_eq!(parent_recipe.description, "Parent recipe");
            assert_eq!(
                parent_recipe.prompt.unwrap(),
                "show me the news for day: today\nWhat is the capital of France?\n\n    Feature is enabled.\n"
            );
            assert_eq!(parent_recipe.parameters.as_ref().unwrap().len(), 2);
            assert_eq!(parent_recipe.parameters.as_ref().unwrap()[0].key, "date");
            assert_eq!(
                parent_recipe.parameters.as_ref().unwrap()[1].key,
                "is_enabled"
            );

            let child_result = load_recipe_as_template(child_path.to_str().unwrap(), params);
            assert!(child_result.is_ok());
            let child_recipe = child_result.unwrap();
            assert_eq!(child_recipe.title, "Parent");
            assert_eq!(child_recipe.description, "Parent recipe");
            assert_eq!(
                child_recipe.prompt.unwrap().trim(),
                "show me the news for day: today\nWhat is the capital of Germany?\n\n    Feature is enabled."
            );
            assert_eq!(child_recipe.parameters.as_ref().unwrap().len(), 2);
            assert_eq!(child_recipe.parameters.as_ref().unwrap()[0].key, "date");
            assert_eq!(
                child_recipe.parameters.as_ref().unwrap()[1].key,
                "is_enabled"
            );
        }
    }
}
