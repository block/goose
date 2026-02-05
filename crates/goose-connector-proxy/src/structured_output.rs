//! Structured output prompt injection and response parsing.
//!
//! Since the custom LLM does not support native structured output (response_format),
//! this module:
//! 1. Injects JSON schema into the system prompt
//! 2. Instructs the LLM to output only valid JSON matching the schema
//! 3. Cleans up the response (strips code fences, validates JSON)

use serde_json::Value;

const STRUCTURED_OUTPUT_SYSTEM_PROMPT: &str = r#"# Response Format Instructions

You MUST respond with a valid JSON object that conforms to the following JSON schema.
Do NOT include any text before or after the JSON.
Do NOT wrap the JSON in markdown code fences.
Your entire response must be ONLY the JSON object.

## Schema: {schema_name}

```json
{schema_json}
```

## Rules
- Output ONLY the JSON object. No markdown code fences. No explanatory text.
- All required fields must be present.
- Field types must match the schema exactly."#;

const JSON_MODE_SYSTEM_PROMPT: &str = r#"# Response Format Instructions

You MUST respond with a valid JSON object.
Do NOT include any text before or after the JSON.
Do NOT wrap the JSON in markdown code fences.
Your entire response must be ONLY valid JSON."#;

/// Build system prompt augmentation for structured output.
///
/// Returns `None` if `response_format` is not applicable.
pub fn build_structured_output_prompt(response_format: &Value) -> Option<String> {
    let fmt_type = response_format.get("type")?.as_str()?;

    match fmt_type {
        "json_schema" => {
            let json_schema = response_format
                .get("json_schema")
                .cloned()
                .unwrap_or_default();
            let schema_name = json_schema
                .get("name")
                .and_then(|v| v.as_str())
                .unwrap_or("Response");
            let schema = json_schema
                .get("schema")
                .cloned()
                .unwrap_or_default();
            let schema_json =
                serde_json::to_string_pretty(&schema).unwrap_or_default();
            Some(
                STRUCTURED_OUTPUT_SYSTEM_PROMPT
                    .replace("{schema_name}", schema_name)
                    .replace("{schema_json}", &schema_json),
            )
        }
        "json_object" => Some(JSON_MODE_SYSTEM_PROMPT.to_string()),
        _ => None,
    }
}

/// Clean structured output response.
///
/// Strips markdown code fences and extraneous whitespace.
/// Returns the cleaned JSON string if valid, or the original content.
pub fn parse_structured_output(content: &str) -> String {
    let mut cleaned = content.trim().to_string();

    // Remove markdown code fences
    if cleaned.starts_with("```json") {
        cleaned = cleaned["```json".len()..].trim().to_string();
    } else if cleaned.starts_with("```") {
        cleaned = cleaned["```".len()..].trim().to_string();
    }

    if cleaned.ends_with("```") {
        cleaned = cleaned[..cleaned.len() - "```".len()].trim().to_string();
    }

    // Validate it's valid JSON
    if serde_json::from_str::<Value>(&cleaned).is_ok() {
        cleaned
    } else {
        content.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_json_schema_prompt() {
        let rf = serde_json::json!({
            "type": "json_schema",
            "json_schema": {
                "name": "TestOutput",
                "schema": {
                    "type": "object",
                    "properties": {
                        "answer": {"type": "string"}
                    }
                }
            }
        });
        let prompt = build_structured_output_prompt(&rf).unwrap();
        assert!(prompt.contains("TestOutput"));
        assert!(prompt.contains("\"answer\""));
    }

    #[test]
    fn test_build_json_object_prompt() {
        let rf = serde_json::json!({"type": "json_object"});
        let prompt = build_structured_output_prompt(&rf).unwrap();
        assert!(prompt.contains("valid JSON"));
    }

    #[test]
    fn test_build_text_prompt_returns_none() {
        let rf = serde_json::json!({"type": "text"});
        assert!(build_structured_output_prompt(&rf).is_none());
    }

    #[test]
    fn test_parse_structured_output_clean_json() {
        let content = r#"{"answer": "hello"}"#;
        assert_eq!(parse_structured_output(content), content);
    }

    #[test]
    fn test_parse_structured_output_with_code_fence() {
        let content = "```json\n{\"answer\": \"hello\"}\n```";
        assert_eq!(parse_structured_output(content), "{\"answer\": \"hello\"}");
    }

    #[test]
    fn test_parse_structured_output_with_plain_fence() {
        let content = "```\n{\"answer\": \"hello\"}\n```";
        assert_eq!(parse_structured_output(content), "{\"answer\": \"hello\"}");
    }

    #[test]
    fn test_parse_structured_output_invalid_json() {
        let content = "not json at all";
        assert_eq!(parse_structured_output(content), content);
    }
}
