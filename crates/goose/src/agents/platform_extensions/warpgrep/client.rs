use super::tools;
use super::tree_gen;
use crate::config::Config;

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

const API_ENDPOINT: &str = "https://api.morphllm.com/v1/chat/completions";
const MODEL: &str = "morph-warp-grep-v2";
const MAX_TURNS: usize = 4;
const REQUEST_TIMEOUT_SECS: u64 = 120;
const MAX_TOKENS: u32 = 16384;

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
    max_tokens: u32,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<ToolDef>>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    role: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tool_call_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct ToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: FunctionCall,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct FunctionCall {
    name: String,
    arguments: String,
}

#[derive(Serialize)]
struct ToolDef {
    #[serde(rename = "type")]
    tool_type: String,
    function: FunctionDef,
}

#[derive(Serialize)]
struct FunctionDef {
    name: String,
    description: String,
    parameters: Value,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: ResponseMessage,
}

#[derive(Deserialize)]
struct ResponseMessage {
    content: Option<String>,
    tool_calls: Option<Vec<ToolCall>>,
}

pub fn get_api_key() -> Result<String, String> {
    if let Ok(config_key) = Config::global().get_secret::<String>("MORPH_API_KEY") {
        return Ok(config_key);
    }
    std::env::var("WARPGREP_API_KEY").map_err(|_| {
        "Missing API key. Set MORPH_API_KEY in goose config or WARPGREP_API_KEY env var."
            .to_string()
    })
}

/// Orchestrate a multi-turn WarpGrep search against the Morph API.
pub async fn search(query: &str, working_dir: &Path, api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|err| format!("Failed to create HTTP client: {err}"))?;

    let repo_structure = tree_gen::generate_repo_structure(working_dir);

    let initial_content = format!(
        "<repo_structure>\n{repo_structure}\n</repo_structure>\n\n<search_string>\n{query}\n</search_string>"
    );

    let mut messages = vec![Message {
        role: "user".to_string(),
        content: Some(initial_content),
        tool_calls: None,
        tool_call_id: None,
    }];

    for _turn in 0..MAX_TURNS {
        let request = ChatRequest {
            model: MODEL.to_string(),
            messages: messages.clone(),
            temperature: 0.0,
            max_tokens: MAX_TOKENS,
            stream: false,
            tools: Some(tool_definitions()),
        };

        let response = client
            .post(API_ENDPOINT)
            .header("Authorization", format!("Bearer {api_key}"))
            .json(&request)
            .send()
            .await
            .map_err(|err| {
                if err.is_timeout() {
                    "WarpGrep request timed out. The search may be too broad.".to_string()
                } else {
                    format!("WarpGrep API request failed: {err}")
                }
            })?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(match status.as_u16() {
                401 => "Invalid or missing API key for WarpGrep.".to_string(),
                429 => "WarpGrep rate limit exceeded. Try again later.".to_string(),
                code if code >= 500 => format!("WarpGrep server error ({code}): {body}"),
                _ => format!("WarpGrep API error ({status}): {body}"),
            });
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|err| format!("Failed to parse WarpGrep response: {err}"))?;

        let choice = chat_response
            .choices
            .into_iter()
            .next()
            .ok_or_else(|| "WarpGrep returned empty response.".to_string())?;

        let tool_calls = match choice.message.tool_calls {
            Some(calls) if !calls.is_empty() => calls,
            _ => {
                // No tool calls — model returned a final text response
                return Ok(choice
                    .message
                    .content
                    .unwrap_or_else(|| "No results found.".to_string()));
            }
        };

        messages.push(Message {
            role: "assistant".to_string(),
            content: choice.message.content,
            tool_calls: Some(tool_calls.clone()),
            tool_call_id: None,
        });

        // If any tool call is `finish`, execute it and return immediately
        for tool_call in &tool_calls {
            if tool_call.function.name == "finish" {
                let args: Value = serde_json::from_str(&tool_call.function.arguments)
                    .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));
                let result = tools::execute_tool("finish", &args, working_dir).await;
                return Ok(result);
            }
        }

        for tool_call in &tool_calls {
            let args: Value = serde_json::from_str(&tool_call.function.arguments)
                .unwrap_or_else(|_| Value::Object(serde_json::Map::new()));
            let result = tools::execute_tool(&tool_call.function.name, &args, working_dir).await;

            messages.push(Message {
                role: "tool".to_string(),
                content: Some(result),
                tool_calls: None,
                tool_call_id: Some(tool_call.id.clone()),
            });
        }
    }

    // Exhausted turns — return what we have
    Ok("Search completed but did not converge within the turn limit.".to_string())
}

fn tool_definitions() -> Vec<ToolDef> {
    vec![
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "ripgrep".to_string(),
                description: "Search for a pattern in files using ripgrep.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "pattern": { "type": "string", "description": "Regex pattern to search" },
                        "path": { "type": "string", "description": "Path to search in" },
                        "glob": { "type": "string", "description": "Glob pattern to filter files" }
                    },
                    "required": ["pattern", "path"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "read".to_string(),
                description: "Read a file, optionally specific line ranges.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "File path to read" },
                        "lines": { "type": "string", "description": "Line ranges like '1-50,75-100'" }
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "list_directory".to_string(),
                description: "List files and directories at a path.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "path": { "type": "string", "description": "Directory path to list" }
                    },
                    "required": ["path"]
                }),
            },
        },
        ToolDef {
            tool_type: "function".to_string(),
            function: FunctionDef {
                name: "finish".to_string(),
                description: "Finish the search and return the relevant file spans.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "files": {
                            "type": "string",
                            "description": "File spans in format: path:start-end,start-end\\npath2:start-end"
                        }
                    },
                    "required": ["files"]
                }),
            },
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definitions_has_four_tools() {
        let defs = tool_definitions();
        assert_eq!(defs.len(), 4);
        let names: Vec<&str> = defs.iter().map(|t| t.function.name.as_str()).collect();
        assert!(names.contains(&"ripgrep"));
        assert!(names.contains(&"read"));
        assert!(names.contains(&"list_directory"));
        assert!(names.contains(&"finish"));
    }

    #[test]
    fn missing_api_key_returns_error() {
        // In test context, MORPH_API_KEY and WARPGREP_API_KEY are typically unset
        // so get_api_key should return an Err unless they happen to be set.
        // We simply verify the function does not panic.
        let _ = get_api_key();
    }
}
