#[cfg(test)]
mod tests {
    use goose::recipe::Recipe;

    // Define test recipe constants directly in the test module
    const TEST_PLAN_WORK_RECIPE: &str = include_str!("../goose_swarm/plan_work.yaml");
    const TEST_SWARM_DRONE_RECIPE: &str = include_str!("../goose_swarm/swarm_drone.yaml");
    const TEST_EVALUATE_RECIPE: &str = include_str!("../goose_swarm/evaluate.yaml");

    #[test]
    fn test_plan_work_recipe_parses() {
        let result = serde_yaml::from_str::<Recipe>(TEST_PLAN_WORK_RECIPE);
        assert!(
            result.is_ok(),
            "Failed to parse plan_work.yaml: {:?}",
            result.err()
        );

        let recipe = result.unwrap();
        assert_eq!(recipe.title, "Plan Work");
        assert_eq!(
            recipe.description,
            "Break down GitHub issues into parallel executable tasks"
        );
        assert!(recipe.instructions.is_some());
        assert!(recipe.prompt.is_some());
    }

    #[test]
    fn test_swarm_drone_recipe_parses() {
        let result = serde_yaml::from_str::<Recipe>(TEST_SWARM_DRONE_RECIPE);
        assert!(
            result.is_ok(),
            "Failed to parse swarm_drone.yaml: {:?}",
            result.err()
        );

        let recipe = result.unwrap();
        assert_eq!(recipe.title, "Swarm Drone");
        assert_eq!(
            recipe.description,
            "Execute a GitHub task issue and create a PR"
        );
        assert!(recipe.instructions.is_some());
        assert!(recipe.prompt.is_some());
    }

    #[test]
    fn test_evaluate_recipe_parses() {
        let result = serde_yaml::from_str::<Recipe>(TEST_EVALUATE_RECIPE);
        assert!(
            result.is_ok(),
            "Failed to parse evaluate.yaml: {:?}",
            result.err()
        );

        let recipe = result.unwrap();
        assert!(recipe.instructions.is_some());
        assert!(recipe.prompt.is_some());
    }

    #[test]
    fn test_recipe_template_substitution() {
        // Test that our recipe prompt templates work with substitution
        let recipe: Recipe = serde_yaml::from_str(TEST_PLAN_WORK_RECIPE).unwrap();
        let prompt_template = recipe.prompt.unwrap();

        // Verify template has placeholders (using single braces as per the YAML)
        assert!(
            prompt_template.contains("{{ context }}"),
            "Should contain context placeholder"
        );
        assert!(
            prompt_template.contains("{{ repo_dir }}"),
            "Should contain repo_dir placeholder"
        );
        assert!(
            prompt_template.contains("{{ work_dir }}"),
            "Should contain work_dir placeholder"
        );
        assert!(
            prompt_template.contains("{{ available_nodes }}"),
            "Should contain available_nodes placeholder"
        );

        // Test substitution
        let mut prompt = prompt_template.clone();
        prompt = prompt.replace("{{ context }}", "test context");
        prompt = prompt.replace("{{ repo_dir }}", "/test/repo");

        assert!(prompt.contains("test context"));
        assert!(prompt.contains("/test/repo"));
        assert!(!prompt.contains("{{ context }}"));
        assert!(!prompt.contains("{{ repo_dir }}"));
    }
}
