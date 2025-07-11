use std::collections::HashMap;

use crate::recipe::{Execution, ExecutionRun, SubRecipe};
use serde_json::json;

use crate::agents::recipe_tools::param_utils::prepare_command_params;

fn setup_default_sub_recipe() -> SubRecipe {
    let sub_recipe = SubRecipe {
        name: "test_sub_recipe".to_string(),
        path: "test_sub_recipe.yaml".to_string(),
        timeout_in_seconds: None,
        values: Some(HashMap::from([("key1".to_string(), "value1".to_string())])),
        executions: None,
    };
    sub_recipe
}

fn create_execution_values(key: &str, values: Vec<String>) -> Execution {
    let runs = values
        .iter()
        .map(|value| ExecutionRun {
            values: Some(HashMap::from([(key.to_string(), value.to_string())])),
        })
        .collect();
    Execution {
        parallel: true,
        runs: Some(runs),
    }
}

mod prepare_command_params_tests {
    use super::*;

    mod without_execution_runs {
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
            assert_eq!(result.len(), 0);
        }
    }

    mod with_execution_runs {
        use super::*;

        #[test]
        fn test_return_command_param() {
            let parameter_array = vec![json!(HashMap::from([(
                "key3".to_string(),
                "value3".to_string()
            )]))];
            let mut sub_recipe = setup_default_sub_recipe();
            sub_recipe.values = Some(HashMap::from([("key1".to_string(), "value1".to_string())]));
            sub_recipe.executions =
                Some(create_execution_values("key2", vec!["value2".to_string()]));

            let result = prepare_command_params(&sub_recipe, parameter_array).unwrap();
            assert_eq!(
                vec![HashMap::from([
                    ("key1".to_string(), "value1".to_string()),
                    ("key2".to_string(), "value2".to_string()),
                    ("key3".to_string(), "value3".to_string())
                ]),],
                result
            );
        }

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
            sub_recipe.executions = None;

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
        fn test_return_command_param_when_all_from_values_in_sub_recipe() {
            let parameter_array = vec![];
            let mut sub_recipe = setup_default_sub_recipe();
            sub_recipe.values = Some(HashMap::from([
                ("key1".to_string(), "value1".to_string()),
                ("key3".to_string(), "value3".to_string()),
            ]));
            sub_recipe.executions = Some(create_execution_values(
                "key2",
                vec!["key2_value1".to_string(), "key2_value2".to_string()],
            ));

            let result = prepare_command_params(&sub_recipe, parameter_array).unwrap();
            assert_eq!(
                vec![
                    HashMap::from([
                        ("key1".to_string(), "value1".to_string()),
                        ("key2".to_string(), "key2_value1".to_string()),
                        ("key3".to_string(), "value3".to_string()),
                    ]),
                    HashMap::from([
                        ("key1".to_string(), "value1".to_string()),
                        ("key2".to_string(), "key2_value2".to_string()),
                        ("key3".to_string(), "value3".to_string()),
                    ])
                ],
                result
            );
        }

        #[test]
        fn test_return_command_param_when_tool_call_parameters_has_one_item_and_execution_runs_has_multiple_items(
        ) {
            let parameter_array = vec![json!(HashMap::from([(
                "key3".to_string(),
                "value3".to_string()
            ),]))];
            let mut sub_recipe = setup_default_sub_recipe();
            sub_recipe.values = Some(HashMap::from([("key1".to_string(), "value1".to_string())]));
            sub_recipe.executions = Some(create_execution_values(
                "key2",
                vec!["key2_value1".to_string(), "key2_value2".to_string()],
            ));

            let result = prepare_command_params(&sub_recipe, parameter_array).unwrap();
            assert_eq!(
                vec![
                    HashMap::from([
                        ("key1".to_string(), "value1".to_string()),
                        ("key2".to_string(), "key2_value1".to_string()),
                        ("key3".to_string(), "value3".to_string()),
                    ]),
                    HashMap::from([
                        ("key1".to_string(), "value1".to_string()),
                        ("key2".to_string(), "key2_value2".to_string()),
                        ("key3".to_string(), "value3".to_string()),
                    ])
                ],
                result
            );
        }

        #[test]
        fn test_throw_error_when_execution_runs_value_length_not_match_with_tool_call_parameters() {
            let parameter_array = vec![
                json!(HashMap::from([("key3".to_string(), "value3".to_string())])),
                json!(HashMap::from([("key4".to_string(), "value4".to_string())])),
            ];
            let mut sub_recipe = setup_default_sub_recipe();
            sub_recipe.values = Some(HashMap::from([("key1".to_string(), "value1".to_string())]));
            sub_recipe.executions = Some(create_execution_values(
                "key2",
                vec!["key2_value1".to_string()],
            ));

            let result = prepare_command_params(&sub_recipe, parameter_array);

            assert!(result.is_err());
        }
    }
}
