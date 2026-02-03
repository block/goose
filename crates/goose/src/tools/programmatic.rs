//! Programmatic Tool Calling - Structured outputs for tool calls

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// A programmatic tool call with structured input/output
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProgrammaticToolCall {
    pub tool_name: String,
    pub input: Value,
    pub schema: Option<Value>,
    pub examples: Vec<ToolExample>,
    pub validation_rules: Vec<ValidationRule>,
}

impl ProgrammaticToolCall {
    pub fn new(tool_name: impl Into<String>, input: Value) -> Self {
        Self {
            tool_name: tool_name.into(),
            input,
            schema: None,
            examples: Vec::new(),
            validation_rules: Vec::new(),
        }
    }

    pub fn with_schema(mut self, schema: Value) -> Self {
        self.schema = Some(schema);
        self
    }

    pub fn with_examples(mut self, examples: Vec<ToolExample>) -> Self {
        self.examples = examples;
        self
    }

    pub fn with_validation(mut self, rules: Vec<ValidationRule>) -> Self {
        self.validation_rules = rules;
        self
    }

    /// Validate input against schema and rules
    pub fn validate(&self) -> Result<(), Vec<String>> {
        let mut errors = Vec::new();

        // Validate against schema if provided
        if let Some(ref schema) = self.schema {
            if let Err(e) = self.validate_against_schema(schema) {
                errors.push(e);
            }
        }

        // Validate against custom rules
        for rule in &self.validation_rules {
            if let Err(e) = rule.validate(&self.input) {
                errors.push(e);
            }
        }

        if errors.is_empty() {
            Ok(())
        } else {
            Err(errors)
        }
    }

    fn validate_against_schema(&self, _schema: &Value) -> Result<(), String> {
        // In a real implementation, use jsonschema crate
        Ok(())
    }
}

/// Example of tool usage for improved accuracy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolExample {
    pub description: String,
    pub input: Value,
    pub expected_output: Option<Value>,
    pub notes: Option<String>,
}

impl ToolExample {
    pub fn new(description: impl Into<String>, input: Value) -> Self {
        Self {
            description: description.into(),
            input,
            expected_output: None,
            notes: None,
        }
    }

    pub fn with_output(mut self, output: Value) -> Self {
        self.expected_output = Some(output);
        self
    }

    pub fn with_notes(mut self, notes: impl Into<String>) -> Self {
        self.notes = Some(notes.into());
        self
    }
}

/// Validation rule for tool inputs
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationRule {
    pub name: String,
    pub rule_type: ValidationRuleType,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ValidationRuleType {
    Required { field: String },
    MinLength { field: String, min: usize },
    MaxLength { field: String, max: usize },
    Pattern { field: String, pattern: String },
    Range { field: String, min: f64, max: f64 },
    OneOf { field: String, values: Vec<String> },
    Custom { validator: String },
}

impl ValidationRule {
    pub fn required(field: impl Into<String>) -> Self {
        let field = field.into();
        Self {
            name: format!("required_{}", field),
            rule_type: ValidationRuleType::Required {
                field: field.clone(),
            },
            message: format!("Field '{}' is required", field),
        }
    }

    pub fn min_length(field: impl Into<String>, min: usize) -> Self {
        let field = field.into();
        Self {
            name: format!("min_length_{}", field),
            rule_type: ValidationRuleType::MinLength {
                field: field.clone(),
                min,
            },
            message: format!("Field '{}' must be at least {} characters", field, min),
        }
    }

    pub fn one_of(field: impl Into<String>, values: Vec<String>) -> Self {
        let field = field.into();
        Self {
            name: format!("one_of_{}", field),
            rule_type: ValidationRuleType::OneOf {
                field: field.clone(),
                values: values.clone(),
            },
            message: format!("Field '{}' must be one of: {:?}", field, values),
        }
    }

    pub fn validate(&self, input: &Value) -> Result<(), String> {
        match &self.rule_type {
            ValidationRuleType::Required { field } => {
                if input.get(field).is_none() || input[field].is_null() {
                    return Err(self.message.clone());
                }
            }
            ValidationRuleType::MinLength { field, min } => {
                if let Some(Value::String(s)) = input.get(field) {
                    if s.len() < *min {
                        return Err(self.message.clone());
                    }
                }
            }
            ValidationRuleType::MaxLength { field, max } => {
                if let Some(Value::String(s)) = input.get(field) {
                    if s.len() > *max {
                        return Err(self.message.clone());
                    }
                }
            }
            ValidationRuleType::OneOf { field, values } => {
                if let Some(Value::String(s)) = input.get(field) {
                    if !values.contains(s) {
                        return Err(self.message.clone());
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
}

/// Result of a programmatic tool call
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallResult {
    pub success: bool,
    pub output: Value,
    pub error: Option<String>,
    pub duration_ms: u64,
    pub tokens_used: Option<usize>,
}

impl ToolCallResult {
    pub fn success(output: Value) -> Self {
        Self {
            success: true,
            output,
            error: None,
            duration_ms: 0,
            tokens_used: None,
        }
    }

    pub fn failure(error: impl Into<String>) -> Self {
        Self {
            success: false,
            output: Value::Null,
            error: Some(error.into()),
            duration_ms: 0,
            tokens_used: None,
        }
    }

    pub fn with_duration(mut self, duration_ms: u64) -> Self {
        self.duration_ms = duration_ms;
        self
    }

    pub fn with_tokens(mut self, tokens: usize) -> Self {
        self.tokens_used = Some(tokens);
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_programmatic_tool_call() {
        let call = ProgrammaticToolCall::new(
            "Bash",
            json!({
                "command": "ls -la"
            }),
        );

        assert_eq!(call.tool_name, "Bash");
    }

    #[test]
    fn test_tool_example() {
        let example = ToolExample::new(
            "List files in directory",
            json!({
                "command": "ls -la /home"
            }),
        )
        .with_output(json!({
            "files": ["file1.txt", "file2.txt"]
        }))
        .with_notes("Returns array of filenames");

        assert!(example.expected_output.is_some());
        assert!(example.notes.is_some());
    }

    #[test]
    fn test_validation_rule_required() {
        let rule = ValidationRule::required("command");

        let valid = json!({"command": "ls"});
        assert!(rule.validate(&valid).is_ok());

        let invalid = json!({"other": "value"});
        assert!(rule.validate(&invalid).is_err());
    }

    #[test]
    fn test_validation_rule_one_of() {
        let rule = ValidationRule::one_of("type", vec!["read".to_string(), "write".to_string()]);

        let valid = json!({"type": "read"});
        assert!(rule.validate(&valid).is_ok());

        let invalid = json!({"type": "delete"});
        assert!(rule.validate(&invalid).is_err());
    }

    #[test]
    fn test_tool_call_result() {
        let result = ToolCallResult::success(json!({"output": "success"}))
            .with_duration(100)
            .with_tokens(50);

        assert!(result.success);
        assert_eq!(result.duration_ms, 100);
        assert_eq!(result.tokens_used, Some(50));
    }
}
