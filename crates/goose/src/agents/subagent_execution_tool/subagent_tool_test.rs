use crate::agents::subagent_execution_tool::subagent_tool::SubagentParams;
use crate::recipe::Recipe;

#[test]
fn test_subagent_params_schema() {
    let schema = rmcp::schemars::schema_for!(SubagentParams);
    let json = serde_json::to_value(&schema).unwrap();
    let properties = json.get("properties").unwrap().as_object().unwrap();
    
    // Check for task_id
    assert!(properties.contains_key("task_id"));
    
    // Check for instructions
    assert!(properties.contains_key("instructions"));
}

#[test]
fn test_try_from_params_with_task_id() {
    let params = SubagentParams {
        task_id: Some("task-123".to_string()),
        instructions: None,
        prompt: None,
        subagent_type: None,
        title: None,
        description: None,
        version: None,
        extensions: None,
        settings: None,
        activities: None,
        author: None,
        parameters: None,
        response: None,
        sub_recipes: None,
        retry: None,
        return_last_only: true,
    };
    
    // Should succeed because task_id is present, even without instructions
    let recipe = Recipe::try_from(params).unwrap();
    assert_eq!(recipe.instructions, Some("Executing existing task".to_string()));
}

#[test]
fn test_try_from_params_missing_instructions() {
    let params = SubagentParams {
        task_id: None,
        instructions: None,
        prompt: None,
        subagent_type: None,
        title: None,
        description: None,
        version: None,
        extensions: None,
        settings: None,
        activities: None,
        author: None,
        parameters: None,
        response: None,
        sub_recipes: None,
        retry: None,
        return_last_only: true,
    };
    
    // Should fail because neither task_id nor instructions are present
    assert!(Recipe::try_from(params).is_err());
}
