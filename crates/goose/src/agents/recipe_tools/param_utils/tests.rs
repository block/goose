use std::collections::HashMap;

use crate::recipe::SubRecipe;
use serde_json::json;

use crate::agents::recipe_tools::param_utils::prepare_command_params;

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

mod prepare_command_params_tests {
    use super::*;

    #[test]
    fn test_return_command_param() {
        let parameter_array = vec![json!(HashMap::from([(
            "key2".to_string(),
            "value2".to_string()
        )]))];
        let mut sub_recipe = setup_default_sub_recipe();
        sub_recipe.values = Some(HashMap::from([("key1".to_string(), "value1".to_string())]));

        let result = prepare_command_params(&sub_recipe, parameter_array).unwrap();
        assert_eq!(
            vec![HashMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string())
            ]),],
            result
        );
    }

    #[test]
    fn test_return_command_param_when_value_override_passed_param_value() {
        let parameter_array = vec![json!(HashMap::from([(
            "key2".to_string(),
            "different_value".to_string()
        )]))];
        let mut sub_recipe = setup_default_sub_recipe();
        sub_recipe.values = Some(HashMap::from([
            ("key1".to_string(), "value1".to_string()),
            ("key2".to_string(), "value2".to_string()),
        ]));

        let result = prepare_command_params(&sub_recipe, parameter_array).unwrap();
        assert_eq!(
            vec![HashMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key2".to_string(), "value2".to_string())
            ]),],
            result
        );
    }

    #[test]
    fn test_return_empty_command_param() {
        let parameter_array = vec![];
        let mut sub_recipe = setup_default_sub_recipe();
        sub_recipe.values = None;

        let result = prepare_command_params(&sub_recipe, parameter_array).unwrap();
        assert_eq!(result, vec![HashMap::new()]);
    }

    mod multiple_tool_parameters {
        use super::*;

        #[test]
        fn test_return_command_param_when_all_values_from_tool_call_parameters() {
            let parameter_array = vec![
                json!(HashMap::from([
                    ("key1".to_string(), "key1_value1".to_string()),
                    ("key2".to_string(), "key2_value1".to_string())
                ])),
                json!(HashMap::from([
                    ("key1".to_string(), "key1_value2".to_string()),
                    ("key2".to_string(), "key2_value2".to_string())
                ])),
            ];
            let mut sub_recipe = setup_default_sub_recipe();
            sub_recipe.values = None;

            let result = prepare_command_params(&sub_recipe, parameter_array).unwrap();
            assert_eq!(
                vec![
                    HashMap::from([
                        ("key1".to_string(), "key1_value1".to_string()),
                        ("key2".to_string(), "key2_value1".to_string()),
                    ]),
                    HashMap::from([
                        ("key1".to_string(), "key1_value2".to_string()),
                        ("key2".to_string(), "key2_value2".to_string()),
                    ]),
                ],
                result
            );
        }

        #[test]
        fn test_merge_base_values_with_tool_parameters() {
            let parameter_array = vec![
                json!(HashMap::from([(
                    "key2".to_string(),
                    "override_value1".to_string()
                )])),
                json!(HashMap::from([(
                    "key2".to_string(),
                    "override_value2".to_string()
                )])),
            ];
            let mut sub_recipe = setup_default_sub_recipe();
            sub_recipe.values = Some(HashMap::from([
                ("key1".to_string(), "base_value".to_string()),
                ("key2".to_string(), "original_value".to_string()),
            ]));

            let result = prepare_command_params(&sub_recipe, parameter_array).unwrap();
            assert_eq!(
                vec![
                    HashMap::from([
                        ("key1".to_string(), "base_value".to_string()),
                        ("key2".to_string(), "original_value".to_string()),
                    ]),
                    HashMap::from([
                        ("key1".to_string(), "base_value".to_string()),
                        ("key2".to_string(), "original_value".to_string()),
                    ]),
                ],
                result
            );
        }
    }
}
