use include_dir::{include_dir, Dir};
use serde::Serialize;
use std::path::PathBuf;
use tera::{Context, Error as TeraError, Tera};

// The prompts directory needs to be embedded in the binary (so it works when distributed)
static PROMPTS_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/src/prompts");

pub fn load_prompt<T: Serialize>(template: &str, context_data: &T) -> Result<String, TeraError> {
    let mut tera = Tera::default();
    tera.add_raw_template("inline_template", template)?;
    let context = Context::from_serialize(context_data)?;
    let rendered = tera.render("inline_template", &context)?;
    Ok(rendered.trim().to_string())
}

pub fn load_prompt_file<T: Serialize>(
    template_file: impl Into<PathBuf>,
    context_data: &T,
) -> Result<String, TeraError> {
    let template_path = template_file.into();

    // Get the file content from the embedded directory
    let template_content = if let Some(file) = PROMPTS_DIR.get_file(template_path.to_str().unwrap())
    {
        String::from_utf8_lossy(file.contents()).into_owned()
    } else {
        return Err(TeraError::chain(
            "Failed to find template file",
            std::io::Error::new(
                std::io::ErrorKind::NotFound,
                "Template file not found in embedded directory",
            ),
        ));
    };

    load_prompt(&template_content, context_data)
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcp_core::tool::Tool;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn test_load_prompt() {
        let template = "Hello, {{ name }}! You are {{ age }} years old.";
        let mut context = HashMap::new();
        context.insert("name".to_string(), "Alice".to_string());
        context.insert("age".to_string(), 30.to_string());

        let result = load_prompt(template, &context).unwrap();
        assert_eq!(result, "Hello, Alice! You are 30 years old.");
    }

    #[test]
    fn test_load_prompt_missing_variable() {
        let template = "Hello, {{ name }}! You are {{ age }} years old.";
        let mut context = HashMap::new();
        context.insert("name".to_string(), "Alice".to_string());
        // 'age' is missing from context
        let result = load_prompt(template, &context);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_prompt_file() {
        // since we are embedding the prompts directory, the file path needs to be relative to the prompts directory
        let file_path = PathBuf::from("mock.md");
        let mut context = HashMap::new();
        context.insert("name".to_string(), "Alice".to_string());
        context.insert("age".to_string(), 30.to_string());
        let result = load_prompt_file(file_path, &context).unwrap();
        assert_eq!(
            result,
            "This prompt is only used for testing.\n\nHello, Alice! You are 30 years old."
        );
    }

    #[test]
    fn test_load_prompt_file_missing_file() {
        let file_path = PathBuf::from("non_existent_template.txt");
        let context: HashMap<String, String> = HashMap::new(); // Add type annotation here

        let result = load_prompt_file(file_path, &context);
        assert!(result.is_err());
    }

    #[test]
    fn test_load_prompt_with_tools() {
        let template = "### Tool Descriptions\n{% for tool in tools %}\n{{tool.name}}: {{tool.description}}{% endfor %}";

        let tools = vec![
            Tool::new(
                "calculator",
                "Performs basic math operations",
                json!({
                    "type": "object",
                    "properties": {
                        "operation": {"type": "string"},
                        "numbers": {"type": "array"}
                    }
                }),
            ),
            Tool::new(
                "weather",
                "Gets weather information",
                json!({
                    "type": "object",
                    "properties": {
                        "location": {"type": "string"}
                    }
                }),
            ),
        ];

        let mut context = HashMap::new();
        context.insert("tools".to_string(), tools);

        let result = load_prompt(template, &context).unwrap();
        let expected = "### Tool Descriptions\n\ncalculator: Performs basic math operations\nweather: Gets weather information";
        assert_eq!(result, expected);
    }

    #[test]
    fn test_load_prompt_with_empty_tools() {
        let template = "### Tool Descriptions\n{% for tool in tools %}\n{{tool.name}}: {{tool.description}}{% endfor %}";

        let tools: Vec<Tool> = vec![];
        let mut context = HashMap::new();
        context.insert("tools".to_string(), tools);

        let result = load_prompt(template, &context).unwrap();
        let expected = "### Tool Descriptions";
        assert_eq!(result, expected);
    }
}
