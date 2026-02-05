//! Tool call prompt injection and response parsing.
//!
//! Since the custom LLM does not support native tool calling, this module:
//! 1. Injects tool definitions into the system prompt
//! 2. Instructs the LLM to use `<tool_call>` XML tags for calling tools
//! 3. Parses `<tool_call>` tags from the LLM response
//! 4. Converts parsed tool calls to OpenAI tool_calls format
//! 5. Re-serializes OpenAI tool_calls back to `<tool_call>` tags for multi-turn history

use regex::Regex;
use serde_json::Value;
use std::sync::LazyLock;
use uuid::Uuid;

static TOOL_CALL_PATTERN: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?s)<tool_call>\s*(.*?)\s*</tool_call>").expect("invalid regex")
});

const TOOL_USE_SYSTEM_PROMPT_EN: &str = r#"# Tool Use Instructions

You have access to the following tools.
You do NOT have access to real-time data, external systems, or the internet. Your training data may be outdated or incomplete.
If you are NOT 100% certain your answer is accurate, current, and complete, you MUST call the appropriate tool instead of answering directly.

## Available Tools

{tool_definitions}

## How to Call Tools

When you call a tool, use exactly this format. You may call multiple tools in a single response:

<tool_call>
{{"name": "function_name", "arguments": {{"arg1": "value1", "arg2": "value2"}}}}
</tool_call>

## Rules
- Weather, stock prices, current events, search results, database contents, file contents, system state — you NEVER know these. ALWAYS use a tool for these.
- If the user's request involves performing an action (reading files, running commands, fetching data, etc.), you MUST call the corresponding tool. Do NOT guess or make up the result.
- Each tool call must be in its own <tool_call> tag with valid JSON containing "name" (string) and "arguments" (object).
- Arguments must be valid JSON matching the tool's parameter schema.
- NEVER generate <tool_response> tags yourself. Only the system provides <tool_response> after you call a tool. You must wait for the system to return the result.
- When you receive a <tool_response> from the system, use the result to formulate your final answer.
- <tool_call> tags MUST appear in your main response content, NOT inside your internal reasoning/thinking. The system can only detect tool calls in your visible output.
- Only answer directly for simple greetings or questions where no tool is relevant and you are fully confident."#;

const TOOL_USE_SYSTEM_PROMPT_KO: &str = r#"# 도구 사용 지침

다음 도구들을 사용할 수 있습니다.
실시간 데이터, 외부 시스템, 인터넷에 접근할 수 없습니다. 학습 데이터가 오래되었거나 불완전할 수 있습니다.
답변이 정확하고, 최신이며, 완전하다고 100% 확신하지 못하면, 직접 답변하지 말고 반드시 적절한 도구를 호출하세요.

## 사용 가능한 도구

{tool_definitions}

## 도구 호출 방법

도구를 호출할 때 정확히 다음 형식을 사용하세요. 한 응답에서 여러 도구를 호출할 수 있습니다:

<tool_call>
{{"name": "함수이름", "arguments": {{"인자1": "값1", "인자2": "값2"}}}}
</tool_call>

## 규칙
- 날씨, 주가, 최신 뉴스, 검색 결과, 데이터베이스 내용, 파일 내용, 시스템 상태 — 이런 정보는 절대 알 수 없습니다. 반드시 도구를 사용하세요.
- 사용자의 요청이 작업 수행(파일 읽기, 명령 실행, 데이터 가져오기 등)을 포함하면, 반드시 해당 도구를 호출하세요. 결과를 추측하거나 지어내지 마세요.
- 각 도구 호출은 "name"(문자열)과 "arguments"(객체)를 포함하는 유효한 JSON이 담긴 개별 <tool_call> 태그에 있어야 합니다.
- arguments는 도구의 파라미터 스키마와 일치하는 유효한 JSON이어야 합니다.
- <tool_response> 태그를 직접 생성하지 마세요. 도구 호출 후 시스템만이 <tool_response>를 제공합니다. 시스템이 결과를 반환할 때까지 기다리세요.
- 시스템으로부터 <tool_response>를 받으면, 그 결과를 사용하여 최종 답변을 작성하세요.
- <tool_call> 태그는 내부 추론/사고가 아닌 주요 응답 내용에 나타나야 합니다. 시스템은 출력에서만 도구 호출을 감지합니다.
- 관련 도구가 없고 완전히 확신하는 간단한 인사나 질문에만 직접 답변하세요."#;

const TOOL_CHOICE_REQUIRED_ADDENDUM_EN: &str =
    "\n- You MUST call at least one tool. Do not respond with plain text only.";

const TOOL_CHOICE_REQUIRED_ADDENDUM_KO: &str =
    "\n- 반드시 최소한 하나의 도구를 호출하세요. 일반 텍스트만으로 응답하지 마세요.";

/// Supported prompt languages for tool injection.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum PromptLanguage {
    #[default]
    English,
    Korean,
}

impl PromptLanguage {
    /// Parse from environment variable value.
    pub fn from_env_value(value: &str) -> Self {
        match value.to_lowercase().as_str() {
            "ko" | "korean" | "한글" | "한국어" => PromptLanguage::Korean,
            _ => PromptLanguage::English,
        }
    }

    /// Get from CONNECTOR_PROMPT_LANG environment variable.
    pub fn from_env() -> Self {
        std::env::var("CONNECTOR_PROMPT_LANG")
            .map(|v| Self::from_env_value(&v))
            .unwrap_or_default()
    }
}

/// Format OpenAI tool definitions into human-readable text for prompt injection.
pub fn format_tool_definitions(tools: &[Value]) -> String {
    let mut parts = Vec::new();
    for tool in tools {
        let func = tool.get("function").unwrap_or(tool);
        let name = func
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let description = func
            .get("description")
            .and_then(|v| v.as_str())
            .unwrap_or("No description provided.");
        let params = func.get("parameters").cloned().unwrap_or(Value::Object(Default::default()));
        let params_text = serde_json::to_string_pretty(&params).unwrap_or_default();
        parts.push(format!(
            "### {}\nDescription: {}\nParameters:\n```json\n{}\n```",
            name, description, params_text
        ));
    }
    parts.join("\n\n")
}

/// Build the tool-use system prompt augmentation.
///
/// Returns `None` if no tools are provided.
/// Uses the language from CONNECTOR_PROMPT_LANG environment variable.
pub fn build_tool_use_prompt(tools: &[Value], tool_choice: Option<&Value>) -> Option<String> {
    build_tool_use_prompt_with_lang(tools, tool_choice, PromptLanguage::from_env())
}

/// Build the tool-use system prompt with specified language.
///
/// Returns `None` if no tools are provided.
pub fn build_tool_use_prompt_with_lang(
    tools: &[Value],
    tool_choice: Option<&Value>,
    lang: PromptLanguage,
) -> Option<String> {
    if tools.is_empty() {
        return None;
    }

    let tool_defs_text = format_tool_definitions(tools);

    let (base_prompt, required_addendum) = match lang {
        PromptLanguage::English => (TOOL_USE_SYSTEM_PROMPT_EN, TOOL_CHOICE_REQUIRED_ADDENDUM_EN),
        PromptLanguage::Korean => (TOOL_USE_SYSTEM_PROMPT_KO, TOOL_CHOICE_REQUIRED_ADDENDUM_KO),
    };

    let mut prompt = base_prompt.replace("{tool_definitions}", &tool_defs_text);

    if let Some(choice) = tool_choice {
        if choice.as_str() == Some("required") {
            prompt.push_str(required_addendum);
        }
    }

    Some(prompt)
}

/// Parse `<tool_call>` tags from LLM response content.
///
/// Returns `(tool_calls, remaining_content)`:
/// - `tool_calls`: List of OpenAI-format tool call values, or `None` if none found.
/// - `remaining_content`: Text with `<tool_call>` tags removed.
///
/// Also handles fallback parsing for raw JSON tool calls without `<tool_call>` tags,
/// which some LLMs may produce when they don't follow the instruction format exactly.
pub fn parse_tool_calls(content: &str) -> (Option<Vec<Value>>, String) {
    // First try to parse <tool_call> tags
    let matches: Vec<&str> = TOOL_CALL_PATTERN
        .captures_iter(content)
        .filter_map(|cap| cap.get(1).map(|m| m.as_str()))
        .collect();

    if !matches.is_empty() {
        let mut tool_calls = Vec::new();
        for m in &matches {
            if let Some(tc) = try_parse_tool_call_json(m) {
                tool_calls.push(tc);
            }
        }

        if !tool_calls.is_empty() {
            let remaining = TOOL_CALL_PATTERN.replace_all(content, "").trim().to_string();
            return (Some(tool_calls), remaining);
        }
    }

    // Fallback: Try to parse raw JSON object that looks like a tool call
    // This handles cases where the LLM outputs {"name": "...", "arguments": {...}} without tags
    let trimmed = content.trim();
    if trimmed.starts_with('{') && trimmed.ends_with('}') {
        if let Some(tc) = try_parse_tool_call_json(trimmed) {
            return (Some(vec![tc]), String::new());
        }
    }

    // Fallback: Try to find raw JSON tool calls in the content (may be preceded by text)
    if let Some(json_start) = trimmed.find(r#"{"name":"#).or_else(|| trimmed.find(r#"{ "name":"#)) {
        let json_part = &trimmed[json_start..];
        // Find matching closing brace
        if let Some(tc) = try_extract_json_object(json_part).and_then(|s| try_parse_tool_call_json(&s)) {
            let remaining = trimmed[..json_start].trim().to_string();
            return (Some(vec![tc]), remaining);
        }
    }

    (None, content.to_string())
}

/// Try to parse a JSON string as a tool call object.
/// Returns the OpenAI-format tool call if successful.
fn try_parse_tool_call_json(json_str: &str) -> Option<Value> {
    let parsed: Value = serde_json::from_str(json_str).ok()?;
    let name = parsed.get("name").and_then(|v| v.as_str()).unwrap_or_default();
    if name.is_empty() {
        return None;
    }
    let arguments = parsed.get("arguments").cloned().unwrap_or(Value::Object(Default::default()));
    let arguments_str = serde_json::to_string(&arguments).unwrap_or_else(|_| "{}".to_string());
    let call_id = format!("call_{}", Uuid::new_v4().as_simple());
    Some(serde_json::json!({
        "id": call_id,
        "type": "function",
        "function": {
            "name": name,
            "arguments": arguments_str,
        }
    }))
}

/// Try to extract a complete JSON object from a string that starts with '{'.
/// Returns the JSON string if a balanced object is found.
fn try_extract_json_object(s: &str) -> Option<String> {
    if !s.starts_with('{') {
        return None;
    }
    let mut depth = 0;
    let mut in_string = false;
    let mut escape_next = false;

    for (i, c) in s.char_indices() {
        if escape_next {
            escape_next = false;
            continue;
        }
        match c {
            '\\' if in_string => escape_next = true,
            '"' => in_string = !in_string,
            '{' if !in_string => depth += 1,
            '}' if !in_string => {
                depth -= 1;
                if depth == 0 {
                    return Some(s[..=i].to_string());
                }
            }
            _ => {}
        }
    }
    None
}

/// Convert OpenAI tool_calls back to `<tool_call>` XML tags.
///
/// Used when reconstructing assistant messages for multi-turn conversation history.
pub fn serialize_tool_calls_to_text(tool_calls: &[Value]) -> String {
    let mut parts = Vec::new();
    for tc in tool_calls {
        let func = tc.get("function").unwrap_or(tc);
        let name = func
            .get("name")
            .and_then(|v| v.as_str())
            .unwrap_or_default();
        let arguments_raw = func
            .get("arguments")
            .cloned()
            .unwrap_or(Value::String("{}".to_string()));

        // arguments can be a JSON string or an object
        let arguments: Value = if let Some(s) = arguments_raw.as_str() {
            serde_json::from_str(s).unwrap_or(Value::Object(Default::default()))
        } else {
            arguments_raw
        };

        let obj = serde_json::json!({
            "name": name,
            "arguments": arguments,
        });
        let pretty = serde_json::to_string_pretty(&obj).unwrap_or_default();
        parts.push(format!("<tool_call>\n{}\n</tool_call>", pretty));
    }
    parts.join("\n")
}

/// Format a tool result message as `<tool_response>` text.
///
/// Used when converting OpenAI "tool" role messages to the custom format.
pub fn format_tool_result(tool_call_id: &str, content: &str) -> String {
    format!(
        "<tool_response>\n{{\"tool_call_id\": \"{}\", \"result\": {}}}\n</tool_response>",
        tool_call_id, content
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_tool_definitions() {
        let tools = vec![serde_json::json!({
            "type": "function",
            "function": {
                "name": "read_file",
                "description": "Read contents of a file",
                "parameters": {
                    "type": "object",
                    "properties": {
                        "path": {"type": "string"}
                    }
                }
            }
        })];
        let result = format_tool_definitions(&tools);
        assert!(result.contains("### read_file"));
        assert!(result.contains("Read contents of a file"));
        assert!(result.contains("\"path\""));
    }

    #[test]
    fn test_build_tool_use_prompt_empty() {
        assert!(build_tool_use_prompt(&[], None).is_none());
    }

    #[test]
    fn test_build_tool_use_prompt_with_tools() {
        let tools = vec![serde_json::json!({
            "function": {
                "name": "test_tool",
                "description": "A test tool",
            }
        })];
        let prompt =
            build_tool_use_prompt_with_lang(&tools, None, PromptLanguage::English).unwrap();
        assert!(prompt.contains("# Tool Use Instructions"));
        assert!(prompt.contains("### test_tool"));
    }

    #[test]
    fn test_build_tool_use_prompt_required() {
        let tools = vec![serde_json::json!({
            "function": {
                "name": "test_tool",
                "description": "A test tool",
            }
        })];
        let choice = serde_json::json!("required");
        let prompt =
            build_tool_use_prompt_with_lang(&tools, Some(&choice), PromptLanguage::English)
                .unwrap();
        assert!(prompt.contains("You MUST call at least one tool"));
    }

    #[test]
    fn test_parse_tool_calls_single() {
        let text = r#"I'll read the file. <tool_call>{"name":"read_file","arguments":{"path":"/tmp/test.txt"}}</tool_call>"#;
        let (calls, remaining) = parse_tool_calls(text);
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["name"], "read_file");
        assert_eq!(remaining.trim(), "I'll read the file.");
    }

    #[test]
    fn test_parse_tool_calls_multiple() {
        let text = r#"<tool_call>{"name":"read_file","arguments":{"path":"a.txt"}}</tool_call> and <tool_call>{"name":"write_file","arguments":{"path":"b.txt","content":"hello"}}</tool_call>"#;
        let (calls, _remaining) = parse_tool_calls(text);
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0]["function"]["name"], "read_file");
        assert_eq!(calls[1]["function"]["name"], "write_file");
    }

    #[test]
    fn test_parse_tool_calls_none() {
        let text = "Just a plain response with no tool calls.";
        let (calls, remaining) = parse_tool_calls(text);
        assert!(calls.is_none());
        assert_eq!(remaining, text);
    }

    #[test]
    fn test_parse_tool_calls_malformed_json() {
        let text = "<tool_call>not valid json</tool_call>";
        let (calls, _remaining) = parse_tool_calls(text);
        assert!(calls.is_none());
    }

    #[test]
    fn test_parse_tool_calls_missing_name() {
        let text = r#"<tool_call>{"arguments":{"path":"a.txt"}}</tool_call>"#;
        let (calls, _remaining) = parse_tool_calls(text);
        assert!(calls.is_none());
    }

    #[test]
    fn test_serialize_tool_calls_to_text() {
        let tool_calls = vec![serde_json::json!({
            "id": "call_abc123",
            "type": "function",
            "function": {
                "name": "read_file",
                "arguments": "{\"path\":\"/tmp/test.txt\"}"
            }
        })];
        let text = serialize_tool_calls_to_text(&tool_calls);
        assert!(text.contains("<tool_call>"));
        assert!(text.contains("</tool_call>"));
        assert!(text.contains("read_file"));
        assert!(text.contains("/tmp/test.txt"));
    }

    #[test]
    fn test_format_tool_result() {
        let result = format_tool_result("call_abc123", "\"file contents here\"");
        assert!(result.contains("<tool_response>"));
        assert!(result.contains("</tool_response>"));
        assert!(result.contains("call_abc123"));
        assert!(result.contains("file contents here"));
    }

    #[test]
    fn test_parse_tool_calls_with_whitespace() {
        let text = "<tool_call>\n  {\"name\": \"test\", \"arguments\": {}}\n</tool_call>";
        let (calls, _remaining) = parse_tool_calls(text);
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["name"], "test");
    }

    #[test]
    fn test_parse_tool_calls_unicode() {
        let text = r#"날씨를 확인하겠습니다. <tool_call>{"name":"get_weather","arguments":{"city":"서울"}}</tool_call>"#;
        let (calls, remaining) = parse_tool_calls(text);
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["name"], "get_weather");
        assert!(remaining.contains("날씨를 확인하겠습니다."));
    }

    #[test]
    fn test_round_trip_tool_calls() {
        // Parse -> serialize -> parse should preserve tool call semantics
        let original = r#"Let me help. <tool_call>{"name":"read_file","arguments":{"path":"/tmp/test.txt"}}</tool_call>"#;
        let (calls, _) = parse_tool_calls(original);
        let calls = calls.unwrap();

        let serialized = serialize_tool_calls_to_text(&calls);
        let (reparsed, _) = parse_tool_calls(&serialized);
        let reparsed = reparsed.unwrap();

        assert_eq!(reparsed.len(), 1);
        assert_eq!(reparsed[0]["function"]["name"], "read_file");
    }

    #[test]
    fn test_build_tool_use_prompt_korean() {
        let tools = vec![serde_json::json!({
            "function": {
                "name": "test_tool",
                "description": "A test tool",
            }
        })];
        let prompt = build_tool_use_prompt_with_lang(&tools, None, PromptLanguage::Korean).unwrap();
        assert!(prompt.contains("# 도구 사용 지침"));
        assert!(prompt.contains("### test_tool"));
    }

    #[test]
    fn test_build_tool_use_prompt_korean_required() {
        let tools = vec![serde_json::json!({
            "function": {
                "name": "test_tool",
                "description": "A test tool",
            }
        })];
        let choice = serde_json::json!("required");
        let prompt =
            build_tool_use_prompt_with_lang(&tools, Some(&choice), PromptLanguage::Korean).unwrap();
        assert!(prompt.contains("반드시 최소한 하나의 도구를 호출하세요"));
    }

    #[test]
    fn test_prompt_language_from_env_value() {
        assert_eq!(
            PromptLanguage::from_env_value("ko"),
            PromptLanguage::Korean
        );
        assert_eq!(
            PromptLanguage::from_env_value("korean"),
            PromptLanguage::Korean
        );
        assert_eq!(
            PromptLanguage::from_env_value("한글"),
            PromptLanguage::Korean
        );
        assert_eq!(
            PromptLanguage::from_env_value("한국어"),
            PromptLanguage::Korean
        );
        assert_eq!(
            PromptLanguage::from_env_value("en"),
            PromptLanguage::English
        );
        assert_eq!(
            PromptLanguage::from_env_value("anything"),
            PromptLanguage::English
        );
    }

    #[test]
    fn test_parse_tool_calls_raw_json() {
        // Test fallback parsing for raw JSON without <tool_call> tags
        let text = r#"{"name": "developer__shell", "arguments": {"command": "pwd"}}"#;
        let (calls, remaining) = parse_tool_calls(text);
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["name"], "developer__shell");
        assert!(remaining.is_empty());
    }

    #[test]
    fn test_parse_tool_calls_raw_json_with_preceding_text() {
        // Test fallback parsing for raw JSON preceded by text
        let text = r#"Let me run that command. {"name": "developer__shell", "arguments": {"command": "pwd"}}"#;
        let (calls, remaining) = parse_tool_calls(text);
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["name"], "developer__shell");
        assert_eq!(remaining, "Let me run that command.");
    }

    #[test]
    fn test_parse_tool_calls_prefers_tool_call_tags() {
        // When both tags and raw JSON exist, tags should be preferred
        let text = r#"<tool_call>{"name": "get_weather", "arguments": {"city": "Seoul"}}</tool_call>"#;
        let (calls, remaining) = parse_tool_calls(text);
        let calls = calls.unwrap();
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0]["function"]["name"], "get_weather");
        assert!(remaining.trim().is_empty());
    }

    #[test]
    fn test_try_extract_json_object() {
        let s = r#"{"name": "test", "arguments": {"key": "value"}} extra text"#;
        let extracted = try_extract_json_object(s).unwrap();
        assert_eq!(extracted, r#"{"name": "test", "arguments": {"key": "value"}}"#);

        // Test nested braces
        let s2 = r#"{"name": "test", "arguments": {"nested": {"deep": "value"}}}"#;
        let extracted2 = try_extract_json_object(s2).unwrap();
        assert_eq!(extracted2, s2);

        // Test escaped quotes
        let s3 = r#"{"name": "test", "arguments": {"path": "/tmp/\"file\""}}"#;
        let extracted3 = try_extract_json_object(s3).unwrap();
        assert_eq!(extracted3, s3);
    }
}
