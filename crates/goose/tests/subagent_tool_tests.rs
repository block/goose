use goose::agents::subagent_tool::{create_subagent_tool, SUBAGENT_TOOL_NAME};
use goose::recipe::{Recipe, SubRecipe};
use std::collections::HashMap;
use tempfile::TempDir;

const RECIPE_TWO_PARAMS: &str = r#"
version: "1.0.0"
title: "Test Task"
description: "A test task"
instructions: "Process {{ first }} and {{ second }}"
parameters:
  - key: first
    input_type: string
    requirement: required
    description: "First param"
  - key: second
    input_type: string
    requirement: required
    description: "Second param"
"#;

fn write_recipe(temp_dir: &TempDir, name: &str, content: &str) -> String {
    let path = temp_dir.path().join(format!("{}.yaml", name));
    std::fs::write(&path, content).unwrap();
    path.to_string_lossy().to_string()
}

fn make_subrecipe(path: String, name: &str, values: Option<HashMap<String, String>>) -> SubRecipe {
    SubRecipe {
        name: name.to_string(),
        path,
        values,
        sequential_when_repeated: false,
        description: Some(format!("{} description", name)),
    }
}

#[test]
fn test_tool_description_includes_subrecipe_params_and_filters_presets() {
    let temp_dir = TempDir::new().unwrap();
    let path = write_recipe(&temp_dir, "mytask", RECIPE_TWO_PARAMS);

    let no_presets = make_subrecipe(path.clone(), "mytask", None);
    let tool = create_subagent_tool(&[no_presets]);
    let desc = tool.description.as_ref().unwrap();
    assert!(desc.contains("mytask"));
    assert!(desc.contains("first [required]"));
    assert!(desc.contains("second [required]"));

    let mut preset = HashMap::new();
    preset.insert("second".to_string(), "preset_value".to_string());
    let with_presets = make_subrecipe(path, "deploy", Some(preset));
    let tool = create_subagent_tool(&[with_presets]);
    let params_section = tool
        .description
        .as_ref()
        .unwrap()
        .split("(params:")
        .nth(1)
        .unwrap_or("");
    assert!(params_section.contains("first"));
    assert!(!params_section.contains("second"));
}

#[test]
fn test_adhoc_recipe_builder_and_security_check() {
    let recipe = Recipe::builder()
        .version("1.0.0")
        .title("Adhoc Task")
        .description("An ad-hoc task")
        .instructions("Do the thing")
        .build()
        .unwrap();

    assert_eq!(recipe.title, "Adhoc Task");
    assert_eq!(recipe.instructions.as_ref().unwrap(), "Do the thing");
    assert!(!recipe.check_for_security_warnings());
}

#[test]
fn test_adhoc_tool_schema_properties() {
    let tool = create_subagent_tool(&[]);

    assert_eq!(tool.name, SUBAGENT_TOOL_NAME);
    assert!(tool.description.as_ref().unwrap().contains("Ad-hoc"));
    assert!(!tool
        .description
        .as_ref()
        .unwrap()
        .contains("Available subrecipes"));

    let props = tool
        .input_schema
        .get("properties")
        .unwrap()
        .as_object()
        .unwrap();
    assert!(props.contains_key("instructions"));
    assert!(props.contains_key("subrecipe"));
    assert!(props.contains_key("parameters"));
    assert!(props.contains_key("extensions"));
    assert!(props.contains_key("settings"));
    assert!(props.contains_key("summary"));
}
