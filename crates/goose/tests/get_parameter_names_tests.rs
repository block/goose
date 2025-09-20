use goose::agents::extension_manager::get_parameter_names;
use rmcp::model::Tool;
use rmcp::object;

// Tests get_parameter_names to ensure it:
// - extracts property names from a Tool's input_schema.properties
// - returns an empty vector when properties are missing or not an object
#[test]
fn test_get_parameter_names_extracts_keys_and_handles_missing() {
    // Tool with properties
    let tool_with_props = Tool::new(
        "get_current_weather",
        "Get the current weather in a given location",
        object!({
            "properties": {
                "location": {
                    "type": "string",
                    "description": "The city and state, e.g. San Francisco, CA"
                },
                "unit": {
                    "type": "string",
                    "description": "The unit of temperature to return",
                    "enum": ["celsius", "fahrenheit"]
                },
                "verbose": {
                    "type": "boolean",
                    "description": "Whether to include detailed readings"
                }
            },
            "required": ["location"]
        }),
    );

    let mut names = get_parameter_names(&tool_with_props);
    names.sort();
    assert_eq!(names, vec!["location", "unit", "verbose"]);

    // Tool without a properties object → should yield empty vector
    let tool_without_props = Tool::new(
        "ping",
        "Simple ping",
        object!({
            // no properties here
        }),
    );

    let names_empty = get_parameter_names(&tool_without_props);
    assert!(names_empty.is_empty());
}
