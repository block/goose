#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use goose::recipe::{RecipeParameterInputType, RecipeParameterRequirement};
    use tempfile::TempDir;

    use crate::recipes::recipe::load_recipe;

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

    mod load_recipe {
        use super::*;
        #[test]
        fn test_load_recipe_success() {
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
            let recipe = load_recipe(recipe_path.to_str().unwrap(), params).unwrap();

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
    }
}
