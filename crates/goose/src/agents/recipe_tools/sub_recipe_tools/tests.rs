#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use crate::recipe::SubRecipe;
    use serde_json::json;
    use serde_json::Value;
    use tempfile::TempDir;

    fn setup_default_sub_recipe() -> SubRecipe {
        let sub_recipe = SubRecipe {
            name: "test_sub_recipe".to_string(),
            path: "test_sub_recipe.yaml".to_string(),
            values: Some(HashMap::from([("key1".to_string(), "value1".to_string())])),
            sequential_when_repeated: true,
            description: Some("Test subrecipe".to_string()),
        };
        sub_recipe
    }

    mod get_input_schema {
        use super::*;
        use crate::agents::recipe_tools::sub_recipe_tools::get_input_schema;

        fn prepare_sub_recipe(sub_recipe_file_content: &str) -> (SubRecipe, TempDir) {
            let mut sub_recipe = setup_default_sub_recipe();
            let temp_dir = tempfile::tempdir().unwrap();
            let temp_file = temp_dir.path().join(sub_recipe.path.clone());
            std::fs::write(&temp_file, sub_recipe_file_content).unwrap();
            sub_recipe.path = temp_file.to_string_lossy().to_string();
            (sub_recipe, temp_dir)
        }

        fn verify_task_parameters(result: Value, expected_task_parameters_items: Value) {
            let task_parameters = result
                .get("properties")
                .unwrap()
                .as_object()
                .unwrap()
                .get("task_parameters")
                .unwrap()
                .as_object()
                .unwrap();
            let task_parameters_items = task_parameters.get("items").unwrap();
            assert_eq!(&expected_task_parameters_items, task_parameters_items);
        }

        const SUB_RECIPE_FILE_CONTENT_WITH_TWO_PARAMS: &str = r#"{
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

        #[test]
        fn test_with_one_param_in_tool_input() {
            let (mut sub_recipe, _temp_dir) =
                prepare_sub_recipe(SUB_RECIPE_FILE_CONTENT_WITH_TWO_PARAMS);
            sub_recipe.values = Some(HashMap::from([("key1".to_string(), "value1".to_string())]));

            let result = get_input_schema(&sub_recipe).unwrap();

            verify_task_parameters(
                result,
                json!({
                    "type": "object",
                    "properties": {
                        "key2": { "type": "number", "description": "An optional parameter" }
                    },
                    "required": []
                }),
            );
        }

        #[test]
        fn test_without_param_in_tool_input() {
            let (mut sub_recipe, _temp_dir) =
                prepare_sub_recipe(SUB_RECIPE_FILE_CONTENT_WITH_TWO_PARAMS);
            sub_recipe.values = Some(HashMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string()),
            ]));

            let result = get_input_schema(&sub_recipe).unwrap();

            assert_eq!(
                None,
                result
                    .get("properties")
                    .unwrap()
                    .as_object()
                    .unwrap()
                    .get("task_parameters")
            );
        }

        #[test]
        fn test_with_all_params_in_tool_input() {
            let (mut sub_recipe, _temp_dir) =
                prepare_sub_recipe(SUB_RECIPE_FILE_CONTENT_WITH_TWO_PARAMS);
            sub_recipe.values = None;

            let result = get_input_schema(&sub_recipe).unwrap();

            verify_task_parameters(
                result,
                json!({
                    "type": "object",
                    "properties": {
                        "key1": { "type": "string", "description": "A test parameter" },
                        "key2": { "type": "number", "description": "An optional parameter" }
                    },
                    "required": ["key1"]
                }),
            );
        }
    }
}
