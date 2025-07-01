use crate::recipe::Response;
use mcp_core::{
    tool::{Tool, ToolAnnotations},
    ToolCall, Content, ToolError,
};
use crate::agents::tool_execution::ToolCallResult;
use serde_json::{json, Value};
use indoc::formatdoc;

pub const FINAL_OUTPUT_TOOL_NAME: &str = "final_output";
pub const FINAL_OUTPUT_CONTINUATION_MESSAGE: &str = "You MUST call the `final_output` tool with your final output for the user.";

pub struct FinalOutputTool {
    pub response: Response,
    /// The final output collected for the user. It will be a single line string for easy script extraction from output.
    pub final_output: Option<String>
}

impl FinalOutputTool {
    pub fn new(response: Response) -> Self {
        if response.json_schema.is_none() {
            panic!("Cannot create FinalOutputTool: json_schema is required");
        }
        let schema = response.json_schema.as_ref().unwrap();
        jsonschema::meta::validate(schema).unwrap();
        Self { response, final_output: None }
    }

    pub fn tool(&self) -> Tool {
        let instructions = formatdoc! {r#"
            This tool collects the final output for a user and provides validation for structured JSON final output against a predefined schema.

            This tool MUST be used for the final output to the user.
            
            Purpose:
            - Collects the final output for a user
            - Ensures that final outputs conform to the expected JSON structure
            - Provides clear validation feedback when outputs don't match the schema
            
            Usage:
            - Call the `final_output` tool with your JSON final output
            
            The expected JSON schema format is:

            {}
            
            When validation fails, you'll receive:
            - Specific validation errors
            - The expected format
        "#, serde_json::to_string_pretty(self.response.json_schema.as_ref().unwrap()).unwrap()};

        return Tool::new(
            FINAL_OUTPUT_TOOL_NAME.to_string(),
            instructions,
            json!({
                "type": "object",
                "properties": {
                    "final_output": {
                        "type": "string",
                        "description": "The JSON string final output to validate and collect"
                    }
                },
                "required": ["final_output"]
            }),
            Some(ToolAnnotations {
                title: Some("Final Output".to_string()),
                read_only_hint: false,
                destructive_hint: false,
                idempotent_hint: true,
                open_world_hint: false,
            }),
        );
    }

    pub fn system_prompt(&self) -> String {
        formatdoc! {r#"
            # Final Ouptut Instructions

            You MUST use the `final_output` tool to collect the final output for a user.
            The final output MUST be a valid JSON object that matches the following expected schema:

            {}

            ----
        "#, serde_json::to_string_pretty(self.response.json_schema.as_ref().unwrap()).unwrap()}
    }

    async fn validate_json_output(&self, output: &str) -> Result<Value, String> {
        // First, try to parse the output as JSON
        let parsed_value: Value = match serde_json::from_str(output) {
            Ok(value) => value,
            Err(e) => {
                return Err(format!(
                    "Invalid JSON format: {}\n\nExpected format:\n{}\n\nPlease provide valid JSON that matches the expected schema.",
                    e,
                    serde_json::to_string_pretty(self.response.json_schema.as_ref().unwrap()).unwrap_or_else(|_| "Invalid schema".to_string())
                ));
            }
        };

        // Validate against the schema using jsonschema
        let compiled_schema = match jsonschema::validator_for(self.response.json_schema.as_ref().unwrap()) {
            Ok(schema) => schema,
            Err(e) => {
                return Err(format!("Internal error: Failed to compile schema: {}", e));
            }
        };

        // Use iter_errors to get an iterator over validation errors
        let validation_errors: Vec<String> = compiled_schema
            .iter_errors(&parsed_value)
            .map(|error| format!("- {}: {}", error.instance_path, error))
            .collect();

        if validation_errors.is_empty() {
            Ok(parsed_value)
        } else {
            Err(format!(
                "Validation failed:\n{}\n\nExpected format:\n{}\n\nPlease correct your output to match the expected JSON schema and try again.",
                validation_errors.join("\n"),
                serde_json::to_string_pretty(self.response.json_schema.as_ref().unwrap()).unwrap_or_else(|_| "Invalid schema".to_string())
            ))
        }
    }

    pub async fn execute_tool_call(&mut self, tool_call: ToolCall) -> ToolCallResult {
        match tool_call.name.as_str() {
            FINAL_OUTPUT_TOOL_NAME => {
                let args = &tool_call.arguments;
                let final_output = args.get("final_output").and_then(|v| v.as_str());
                match final_output {
                    Some(final_output) => {
                        self.final_output = Some(final_output.to_string());
                        let result = self.validate_json_output(final_output).await;
                        match result {
                            Ok(parsed_value) => {
                                self.final_output = Some(Self::parsed_final_output_string(parsed_value));
                                ToolCallResult::from(Ok(vec![Content::text("Final output successfully collected.".to_string())]))
                            }
                            Err(error) => ToolCallResult::from(Err(ToolError::InvalidParameters(error))),
                        }
                    }
                    None => ToolCallResult::from(Err(ToolError::InvalidParameters("Missing required 'final_output' parameter".to_string()))),
                }
            }
            _ => ToolCallResult::from(Err(ToolError::NotFound(format!("Unknown tool: {}", tool_call.name)))),
        }
    }

    // Formats the parsed JSON as a single line string so its easy to extract from the output
    fn parsed_final_output_string(parsed_json: Value) -> String {
        serde_json::to_string(&parsed_json).unwrap()
    }
}