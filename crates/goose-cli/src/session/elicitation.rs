//! CLI form handling for MCP elicitation requests.
//!
//! This module provides interactive form rendering for JSON Schema-based
//! elicitation requests from MCP servers.

use serde_json::Value;
use std::collections::HashMap;
use std::io::{self, Write};

/// Result of an elicitation form interaction
pub enum ElicitationResult {
    /// User submitted the form with data
    Submitted(Value),
    /// User cancelled the form (Ctrl+C)
    #[allow(dead_code)]
    Cancelled,
}

/// Render and collect input for an elicitation form based on a JSON Schema.
///
/// Supports the following JSON Schema types:
/// - string (with optional enum for select)
/// - boolean
/// - number/integer (with optional min/max)
///
/// Returns the collected form data as a JSON Value, or None if cancelled.
pub fn render_elicitation_form(
    message: &str,
    schema: &Value,
) -> Result<ElicitationResult, io::Error> {
    // Display the message from the MCP server
    println!();
    println!("{}", console::style("📋 Information Request").cyan().bold());
    println!("{}", console::style(message).cyan());
    println!();

    let properties = match schema.get("properties") {
        Some(Value::Object(props)) => props,
        _ => {
            println!("{}", console::style("No fields to display").dim());
            return Ok(ElicitationResult::Submitted(Value::Object(
                serde_json::Map::new(),
            )));
        }
    };

    let required: Vec<&str> = schema
        .get("required")
        .and_then(|r| r.as_array())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect())
        .unwrap_or_default();

    let mut form_data: HashMap<String, Value> = HashMap::new();

    for (key, prop) in properties {
        let is_required = required.contains(&key.as_str());
        let prop_type = prop
            .get("type")
            .and_then(|t| t.as_str())
            .unwrap_or("string");
        let description = prop.get("description").and_then(|d| d.as_str());

        // Build the prompt label
        let label = if is_required {
            format!("{} {}", key, console::style("*").red())
        } else {
            key.to_string()
        };

        // Show description if available
        if let Some(desc) = description {
            println!("{}", console::style(desc).dim());
        }

        // Check for enum values (select)
        if let Some(enum_values) = prop.get("enum").and_then(|e| e.as_array()) {
            let value = prompt_select(&label, enum_values, is_required)?;
            if let Some(v) = value {
                form_data.insert(key.clone(), v);
            }
        } else {
            match prop_type {
                "boolean" => {
                    let value = prompt_boolean(&label, description)?;
                    form_data.insert(key.clone(), Value::Bool(value));
                }
                "number" | "integer" => {
                    let min = prop.get("minimum").and_then(|m| m.as_f64());
                    let max = prop.get("maximum").and_then(|m| m.as_f64());
                    let value =
                        prompt_number(&label, is_required, min, max, prop_type == "integer")?;
                    if let Some(v) = value {
                        form_data.insert(key.clone(), v);
                    }
                }
                _ => {
                    // Default to string input
                    let min_length = prop.get("minLength").and_then(|m| m.as_u64());
                    let max_length = prop.get("maxLength").and_then(|m| m.as_u64());
                    let value = prompt_string(&label, is_required, min_length, max_length)?;
                    if let Some(v) = value {
                        form_data.insert(key.clone(), Value::String(v));
                    }
                }
            }
        }
    }

    // Convert HashMap to JSON object
    let result: serde_json::Map<String, Value> = form_data.into_iter().collect();
    Ok(ElicitationResult::Submitted(Value::Object(result)))
}

fn prompt_string(
    label: &str,
    required: bool,
    min_length: Option<u64>,
    max_length: Option<u64>,
) -> Result<Option<String>, io::Error> {
    loop {
        let hint = build_string_hint(min_length, max_length);
        let prompt = if hint.is_empty() {
            format!("{}: ", label)
        } else {
            format!("{} {}: ", label, console::style(hint).dim())
        };

        print!("{}", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let value = input.trim().to_string();

        if value.is_empty() {
            if required {
                println!("{}", console::style("This field is required").red());
                continue;
            }
            return Ok(None);
        }

        // Validate length constraints
        if let Some(min) = min_length {
            if (value.len() as u64) < min {
                println!(
                    "{}",
                    console::style(format!("Minimum length is {}", min)).red()
                );
                continue;
            }
        }
        if let Some(max) = max_length {
            if (value.len() as u64) > max {
                println!(
                    "{}",
                    console::style(format!("Maximum length is {}", max)).red()
                );
                continue;
            }
        }

        return Ok(Some(value));
    }
}

fn prompt_number(
    label: &str,
    required: bool,
    min: Option<f64>,
    max: Option<f64>,
    is_integer: bool,
) -> Result<Option<Value>, io::Error> {
    loop {
        let hint = build_number_hint(min, max);
        let prompt = if hint.is_empty() {
            format!("{}: ", label)
        } else {
            format!("{} {}: ", label, console::style(hint).dim())
        };

        print!("{}", prompt);
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let value = input.trim();

        if value.is_empty() {
            if required {
                println!("{}", console::style("This field is required").red());
                continue;
            }
            return Ok(None);
        }

        let num: f64 = match value.parse() {
            Ok(n) => n,
            Err(_) => {
                println!("{}", console::style("Please enter a valid number").red());
                continue;
            }
        };

        // Validate range constraints
        if let Some(m) = min {
            if num < m {
                println!(
                    "{}",
                    console::style(format!("Minimum value is {}", m)).red()
                );
                continue;
            }
        }
        if let Some(m) = max {
            if num > m {
                println!(
                    "{}",
                    console::style(format!("Maximum value is {}", m)).red()
                );
                continue;
            }
        }

        if is_integer {
            return Ok(Some(Value::Number(serde_json::Number::from(num as i64))));
        } else {
            return Ok(Some(Value::Number(
                serde_json::Number::from_f64(num).unwrap_or_else(|| serde_json::Number::from(0)),
            )));
        }
    }
}

fn prompt_boolean(label: &str, description: Option<&str>) -> Result<bool, io::Error> {
    let prompt_text = description.unwrap_or(label);
    match cliclack::confirm(prompt_text)
        .initial_value(false)
        .interact()
    {
        Ok(value) => Ok(value),
        Err(e) => {
            if e.kind() == io::ErrorKind::Interrupted {
                Ok(false)
            } else {
                Err(e)
            }
        }
    }
}

fn prompt_select(
    label: &str,
    options: &[Value],
    required: bool,
) -> Result<Option<Value>, io::Error> {
    let string_options: Vec<String> = options
        .iter()
        .filter_map(|v| match v {
            Value::String(s) => Some(s.clone()),
            Value::Number(n) => Some(n.to_string()),
            Value::Bool(b) => Some(b.to_string()),
            _ => None,
        })
        .collect();

    if string_options.is_empty() {
        return Ok(None);
    }

    let mut select = cliclack::select(label);

    if !required {
        select = select.item("", "(none)", "Skip this field");
    }

    for opt in &string_options {
        select = select.item(opt.as_str(), opt, "");
    }

    match select.interact() {
        Ok(selected) => {
            if selected.is_empty() {
                Ok(None)
            } else {
                // Find the original value from options to preserve type
                let original = options.iter().find(|v| match v {
                    Value::String(s) => s == selected,
                    Value::Number(n) => n.to_string() == selected,
                    Value::Bool(b) => b.to_string() == selected,
                    _ => false,
                });
                Ok(original.cloned())
            }
        }
        Err(e) => {
            if e.kind() == io::ErrorKind::Interrupted {
                Ok(None)
            } else {
                Err(e)
            }
        }
    }
}

fn build_string_hint(min_length: Option<u64>, max_length: Option<u64>) -> String {
    match (min_length, max_length) {
        (Some(min), Some(max)) => format!("({}-{} chars)", min, max),
        (Some(min), None) => format!("(min {} chars)", min),
        (None, Some(max)) => format!("(max {} chars)", max),
        (None, None) => String::new(),
    }
}

fn build_number_hint(min: Option<f64>, max: Option<f64>) -> String {
    match (min, max) {
        (Some(min), Some(max)) => format!("({}-{})", min, max),
        (Some(min), None) => format!("(min {})", min),
        (None, Some(max)) => format!("(max {})", max),
        (None, None) => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_build_string_hint() {
        assert_eq!(build_string_hint(Some(1), Some(10)), "(1-10 chars)");
        assert_eq!(build_string_hint(Some(5), None), "(min 5 chars)");
        assert_eq!(build_string_hint(None, Some(100)), "(max 100 chars)");
        assert_eq!(build_string_hint(None, None), "");
    }

    #[test]
    fn test_build_number_hint() {
        assert_eq!(build_number_hint(Some(0.0), Some(100.0)), "(0-100)");
        assert_eq!(build_number_hint(Some(1.0), None), "(min 1)");
        assert_eq!(build_number_hint(None, Some(50.0)), "(max 50)");
        assert_eq!(build_number_hint(None, None), "");
    }

    #[test]
    fn test_empty_schema_returns_empty_object() {
        let schema = json!({});
        let result = render_elicitation_form("Test message", &schema);
        assert!(result.is_ok());
        if let Ok(ElicitationResult::Submitted(value)) = result {
            assert!(value.as_object().unwrap().is_empty());
        }
    }

    #[test]
    fn test_schema_without_properties_returns_empty_object() {
        let schema = json!({
            "type": "object"
        });
        let result = render_elicitation_form("Test message", &schema);
        assert!(result.is_ok());
        if let Ok(ElicitationResult::Submitted(value)) = result {
            assert!(value.as_object().unwrap().is_empty());
        }
    }
}
