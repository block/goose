#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::recipe::SubRecipe;

    fn setup_sub_recipe() -> SubRecipe {
        let sub_recipe = SubRecipe {
            name: "test_sub_recipe".to_string(),
            path: "test_sub_recipe.yaml".to_string(),
            values: Some(HashMap::from([("key1".to_string(), "value1".to_string())])),
        };
        sub_recipe
    }
    mod prepare_command_params_tests {
        use std::collections::HashMap;

        use crate::{
            agents::recipe_tools::sub_recipe_tools::{
                prepare_command_params, tests::tests::setup_sub_recipe,
            },
            recipe::SubRecipe,
        };

        #[test]
        fn test_prepare_command_params_basic() {
            let mut params = HashMap::new();
            params.insert("key2".to_string(), "value2".to_string());

            let sub_recipe = setup_sub_recipe();

            let params_value = serde_json::to_value(params).unwrap();
            let result = prepare_command_params(&sub_recipe, params_value).unwrap();
            assert_eq!(result.len(), 2);
            assert_eq!(result.get("key1"), Some(&"value1".to_string()));
            assert_eq!(result.get("key2"), Some(&"value2".to_string()));
        }

        #[test]
        fn test_prepare_command_params_empty() {
            let sub_recipe = SubRecipe {
                name: "test_sub_recipe".to_string(),
                path: "test_sub_recipe.yaml".to_string(),
                values: None,
            };
            let params: HashMap<String, String> = HashMap::new();
            let params_value = serde_json::to_value(params).unwrap();
            let result = prepare_command_params(&sub_recipe, params_value).unwrap();
            assert_eq!(result.len(), 0);
        }
    }

    mod get_input_schema_tests {
        use crate::{
            agents::recipe_tools::sub_recipe_tools::{
                get_input_schema, tests::tests::setup_sub_recipe,
            },
            recipe::SubRecipe,
        };

        #[test]
        fn test_get_input_schema_with_parameters() {
            let sub_recipe = setup_sub_recipe();

            let sub_recipe_file_content = r#"{
                "version": "1.0.0",
                "title": "Test Recipe",
                "description": "A test recipe",
                "prompt": "Test prompt",
                "parameters": [
                    {
                        "key": "key1",
                        "input_type": "string",
                        "requirement": "required",
                        "description": "A test parameter"
                    },
                    {
                        "key": "key2",
                        "input_type": "number",
                        "requirement": "optional",
                        "description": "An optional parameter"
                    }
                ]
            }"#;

            let temp_dir = tempfile::tempdir().unwrap();
            let temp_file = temp_dir.path().join("test_sub_recipe.yaml");
            std::fs::write(&temp_file, sub_recipe_file_content).unwrap();

            let mut sub_recipe = sub_recipe;
            sub_recipe.path = temp_file.to_string_lossy().to_string();

            let result = get_input_schema(&sub_recipe).unwrap();

            // Verify the schema structure
            assert_eq!(result["type"], "object");
            assert!(result["properties"].is_object());

            let properties = result["properties"].as_object().unwrap();
            assert_eq!(properties.len(), 1);

            let key2_prop = &properties["key2"];
            assert_eq!(key2_prop["type"], "number");
            assert_eq!(key2_prop["description"], "An optional parameter");

            let required = result["required"].as_array().unwrap();
            assert_eq!(required.len(), 0);
        }

        #[test]
        fn test_get_input_schema_no_parameters_values() {
            let sub_recipe = SubRecipe {
                name: "test_sub_recipe".to_string(),
                path: "test_sub_recipe.yaml".to_string(),
                values: None,
            };

            let sub_recipe_file_content = r#"{
                "version": "1.0.0",
                "title": "Test Recipe",
                "description": "A test recipe",
                "prompt": "Test prompt",
                "parameters": [
                    {
                        "key": "key1",
                        "input_type": "string",
                        "requirement": "required",
                        "description": "A test parameter"
                    }
                ]
            }"#;

            let temp_dir = tempfile::tempdir().unwrap();
            let temp_file = temp_dir.path().join("test_sub_recipe.yaml");
            std::fs::write(&temp_file, sub_recipe_file_content).unwrap();

            let mut sub_recipe = sub_recipe;
            sub_recipe.path = temp_file.to_string_lossy().to_string();

            let result = get_input_schema(&sub_recipe).unwrap();

            assert_eq!(result["type"], "object");
            assert!(result["properties"].is_object());

            let properties = result["properties"].as_object().unwrap();
            assert_eq!(properties.len(), 1);

            let key1_prop = &properties["key1"];
            assert_eq!(key1_prop["type"], "string");
            assert_eq!(key1_prop["description"], "A test parameter");
            assert_eq!(result["required"].as_array().unwrap().len(), 1);
            assert_eq!(result["required"][0], "key1");
        }
    }

    mod create_sub_recipe_task_tests {
        use super::*;
        use crate::agents::recipe_tools::sub_recipe_tools::create_sub_recipe_task;
        use crate::agents::sub_recipe_execution_tool::lib::Task;
        use serde_json::json;

        #[tokio::test]
        async fn test_create_sub_recipe_task_with_timeout() {
            let mut sub_recipe = setup_sub_recipe();
            sub_recipe.values = Some(HashMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("task_timeout".to_string(), "3600".to_string()),
            ]));

            let params = json!({
                "key2": "value2"
            });

            let result = create_sub_recipe_task(&sub_recipe, params).await.unwrap();
            let task: Task = serde_json::from_str(&result).unwrap();

            assert_eq!(task.task_type, "sub_recipe");
            let sub_recipe_obj = task.payload.get("sub_recipe").unwrap();
            assert_eq!(sub_recipe_obj.get("name").unwrap(), "test_sub_recipe");
            assert_eq!(sub_recipe_obj.get("task_timeout").unwrap(), 3600);
            
            let command_params = sub_recipe_obj.get("command_parameters").unwrap();
            assert_eq!(command_params.get("key1").unwrap(), "value1");
            assert_eq!(command_params.get("key2").unwrap(), "value2");
        }

        #[tokio::test]
        async fn test_create_sub_recipe_task_without_timeout() {
            let sub_recipe = setup_sub_recipe();
            let params = json!({
                "key2": "value2"
            });

            let result = create_sub_recipe_task(&sub_recipe, params).await.unwrap();
            let task: Task = serde_json::from_str(&result).unwrap();

            assert_eq!(task.task_type, "sub_recipe");
            let sub_recipe_obj = task.payload.get("sub_recipe").unwrap();
            assert!(sub_recipe_obj.get("task_timeout").is_none());
        }

        #[tokio::test]
        async fn test_create_sub_recipe_task_invalid_timeout() {
            let mut sub_recipe = setup_sub_recipe();
            sub_recipe.values = Some(HashMap::from([
                ("task_timeout".to_string(), "not_a_number".to_string()),
            ]));

            let params = json!({});

            let result = create_sub_recipe_task(&sub_recipe, params).await.unwrap();
            let task: Task = serde_json::from_str(&result).unwrap();

            let sub_recipe_obj = task.payload.get("sub_recipe").unwrap();
            assert!(sub_recipe_obj.get("task_timeout").is_none());
        }
    }
}
