use super::tools;
use super::tree_gen;
use crate::config::Config;

use std::path::Path;
use std::sync::LazyLock;
use std::time::Duration;

use regex::Regex;
use serde::{Deserialize, Serialize};

const API_ENDPOINT: &str = "https://api.morphllm.com/v1/chat/completions";
const MODEL: &str = "morph-warp-grep-v2";
const MAX_TURNS: usize = 4;
const REQUEST_TIMEOUT_SECS: u64 = 30;
const MAX_TOKENS: u32 = 1024;
const MAX_CONTEXT_CHARS: usize = 540_000;

// ---------------------------------------------------------------------------
// Compiled regexes (shared across calls)
// ---------------------------------------------------------------------------

static THINK_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?si)<think>.*?</think>").unwrap());

static TOOL_CALL_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"(?si)<tool_call>\s*<function=([a-z_][a-z0-9_]*)>(.*?)</function>\s*</tool_call>")
        .unwrap()
});

static PARAM_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?si)<parameter=([a-z_][a-z0-9_]*)>(.*?)</parameter>").unwrap());

// ---------------------------------------------------------------------------
// Wire types — minimal, matching what the Morph API actually expects/returns
// ---------------------------------------------------------------------------

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    temperature: f64,
    max_tokens: u32,
    stream: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct Message {
    role: String,
    content: String,
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
}

// ---------------------------------------------------------------------------
// Parsed tool call from XML in assistant content
// ---------------------------------------------------------------------------

struct ParsedToolCall {
    name: String,
    params: Vec<(String, String)>,
}

impl ParsedToolCall {
    fn param(&self, key: &str) -> Option<&str> {
        self.params
            .iter()
            .find(|(k, _)| k == key)
            .map(|(_, v)| v.as_str())
    }
}

// ---------------------------------------------------------------------------
// API key resolution
// ---------------------------------------------------------------------------

pub fn get_api_key() -> Result<String, String> {
    if let Ok(config_key) = Config::global().get_secret::<String>("MORPH_API_KEY") {
        return Ok(config_key);
    }
    std::env::var("MORPH_API_KEY").map_err(|_| {
        "Missing API key. Set MORPH_API_KEY in goose config or as an environment variable."
            .to_string()
    })
}

// ---------------------------------------------------------------------------
// XML tool-call parser (Qwen3-Coder-Next format)
// ---------------------------------------------------------------------------

fn parse_tool_calls(text: &str) -> Vec<ParsedToolCall> {
    let cleaned = THINK_RE.replace_all(text, "");

    let mut calls = Vec::new();
    for cap in TOOL_CALL_RE.captures_iter(&cleaned) {
        let name = cap[1].to_lowercase();
        let body = &cap[2];

        if !matches!(
            name.as_str(),
            "ripgrep" | "read" | "list_directory" | "finish"
        ) {
            continue;
        }

        let mut params = Vec::new();
        for pcap in PARAM_RE.captures_iter(body) {
            params.push((pcap[1].to_lowercase(), pcap[2].trim().to_string()));
        }
        calls.push(ParsedToolCall { name, params });
    }
    calls
}

// ---------------------------------------------------------------------------
// Parse finish file specs from the model's "files" parameter
// ---------------------------------------------------------------------------

struct FileSpec {
    path: String,
    ranges: Vec<(usize, usize)>, // empty = whole file
}

fn parse_finish_files(files_str: &str) -> Vec<FileSpec> {
    let mut specs = Vec::new();
    for line in files_str.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        match line.split_once(':') {
            Some((path, ranges_part)) => {
                let mut ranges = Vec::new();
                for r in ranges_part.split(',') {
                    let r = r.trim();
                    if r.is_empty() || r == "*" {
                        ranges.clear();
                        break;
                    }
                    if let Some((s, e)) = r.split_once('-') {
                        if let (Ok(start), Ok(end)) =
                            (s.trim().parse::<usize>(), e.trim().parse::<usize>())
                        {
                            if start > 0 && end >= start {
                                ranges.push((start, end));
                            }
                        }
                    }
                }
                specs.push(FileSpec {
                    path: path.trim().to_string(),
                    ranges,
                });
            }
            None => {
                specs.push(FileSpec {
                    path: line.to_string(),
                    ranges: Vec::new(),
                });
            }
        }
    }
    specs
}

/// Resolve finish file specs into actual file contents, matching the SDK's
/// output format with `// ... existing code ...` gap markers.
fn resolve_finish(specs: &[FileSpec], working_dir: &Path) -> String {
    let mut output = String::new();
    for spec in specs {
        let resolved = tools::resolve_path(&spec.path, working_dir);

        let content = match std::fs::read_to_string(&resolved) {
            Ok(text) => text,
            Err(err) => {
                output.push_str(&format!("--- {} (error: {err}) ---\n", spec.path));
                continue;
            }
        };

        let all_lines: Vec<&str> = content.lines().collect();
        output.push_str(&format!("--- {} ---\n", spec.path));

        if spec.ranges.is_empty() {
            for (i, line) in all_lines.iter().enumerate() {
                output.push_str(&format!("{:>4} | {}\n", i + 1, line));
            }
        } else {
            let mut ranges = spec.ranges.clone();
            ranges.sort_by_key(|r| r.0);
            let mut merged: Vec<(usize, usize)> = Vec::new();
            for (s, e) in &ranges {
                if let Some(last) = merged.last_mut() {
                    if *s <= last.1 + 2 {
                        last.1 = last.1.max(*e);
                        continue;
                    }
                }
                merged.push((*s, *e));
            }

            for (i, (start, end)) in merged.iter().enumerate() {
                if (i == 0 && *start > 1) || i > 0 {
                    output.push_str(&format!(
                        "// ... existing code, block starting at line {} ...\n",
                        start
                    ));
                }
                let s = start.saturating_sub(1);
                let e = (*end).min(all_lines.len());
                for (j, line) in all_lines[s..e].iter().enumerate() {
                    output.push_str(&format!("{:>4} | {}\n", s + j + 1, line));
                }
            }
        }
    }
    output
}

// ---------------------------------------------------------------------------
// Context helpers (matching SDK behavior)
// ---------------------------------------------------------------------------

fn total_chars(messages: &[Message]) -> usize {
    messages.iter().map(|m| m.content.len()).sum()
}

fn context_budget_tag(messages: &[Message]) -> String {
    let used = total_chars(messages);
    let pct = (used as f64 / MAX_CONTEXT_CHARS as f64 * 100.0).round() as usize;
    let used_k = used / 1000;
    let max_k = MAX_CONTEXT_CHARS / 1000;
    format!("<context_budget>{pct}% ({used_k}K/{max_k}K chars used)</context_budget>")
}

fn enforce_context_limit(messages: &mut [Message]) {
    if total_chars(messages) <= MAX_CONTEXT_CHARS {
        return;
    }
    let mut first_user_skipped = false;
    let indices: Vec<usize> = (0..messages.len())
        .filter(|&i| {
            if messages[i].role == "user" {
                if !first_user_skipped {
                    first_user_skipped = true;
                    return false;
                }
                return true;
            }
            false
        })
        .collect();

    for idx in indices {
        if total_chars(messages) <= MAX_CONTEXT_CHARS {
            break;
        }
        messages[idx].content = "[truncated for context limit]".to_string();
    }
}

fn turn_message(turn: usize, max_turns: usize) -> String {
    let remaining = max_turns - turn;
    if remaining == 1 {
        format!(
            "\nYou have used {turn} turns, you only have 1 turn remaining. \
             You have run out of turns to explore the code base and MUST call the finish tool now"
        )
    } else {
        let s = if turn == 1 { "" } else { "s" };
        format!("\nYou have used {turn} turn{s} and have {remaining} remaining")
    }
}

// ---------------------------------------------------------------------------
// Main search loop
// ---------------------------------------------------------------------------

/// Orchestrate a multi-turn WarpGrep search against the Morph API.
///
/// The model uses Qwen3-Coder-Next XML tool calls in its `content` field,
/// NOT OpenAI-style structured `tool_calls`. We parse the XML, execute tools
/// locally, and wrap results in `<tool_response>` tags sent as user messages.
pub async fn search(query: &str, working_dir: &Path, api_key: &str) -> Result<String, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(REQUEST_TIMEOUT_SECS))
        .build()
        .map_err(|err| format!("Failed to create HTTP client: {err}"))?;

    let repo_structure = tree_gen::generate_repo_structure(working_dir);
    let budget = context_budget_tag(&[]);

    let initial_content = format!(
        "<repo_structure>\n{repo_structure}\n</repo_structure>\n\n\
         <search_string>\n{query}\n</search_string>\n\
         {budget}\nTurn 0/{MAX_TURNS}"
    );

    let mut messages = vec![Message {
        role: "user".to_string(),
        content: initial_content,
    }];

    for turn in 1..=MAX_TURNS {
        enforce_context_limit(&mut messages);

        let request = ChatRequest {
            model: MODEL.to_string(),
            messages: messages.clone(),
            temperature: 0.0,
            max_tokens: MAX_TOKENS,
            stream: false,
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

        let assistant_content = chat_response
            .choices
            .into_iter()
            .next()
            .and_then(|c| c.message.content)
            .unwrap_or_default();

        if assistant_content.is_empty() {
            break;
        }

        messages.push(Message {
            role: "assistant".to_string(),
            content: assistant_content.clone(),
        });

        // Parse XML tool calls from content
        let tool_calls = parse_tool_calls(&assistant_content);
        if tool_calls.is_empty() {
            // No tool calls — model returned a text-only response
            let cleaned = THINK_RE
                .replace_all(&assistant_content, "")
                .trim()
                .to_string();
            if cleaned.is_empty() {
                break;
            }
            return Ok(cleaned);
        }

        // Check for finish tool first
        for tc in &tool_calls {
            if tc.name == "finish" {
                let files_str = tc.param("files").unwrap_or("");
                let text_result = tc.param("result");

                if files_str.is_empty() {
                    return Ok(text_result
                        .unwrap_or("No relevant code found.")
                        .to_string());
                }

                let specs = parse_finish_files(files_str);
                if specs.is_empty() {
                    return Ok(text_result
                        .unwrap_or("No relevant code found.")
                        .to_string());
                }

                return Ok(resolve_finish(&specs, working_dir));
            }
        }

        // Execute non-finish tools and collect results
        let mut formatted_results: Vec<String> = Vec::new();
        for tc in &tool_calls {
            let result = match tc.name.as_str() {
                "ripgrep" => {
                    let pattern = tc.param("pattern").unwrap_or("");
                    let path = tc.param("path").unwrap_or(".");
                    let glob = tc.param("glob");
                    let args = match glob {
                        Some(g) => serde_json::json!({"pattern": pattern, "path": path, "glob": g}),
                        None => serde_json::json!({"pattern": pattern, "path": path}),
                    };
                    tools::execute_tool("ripgrep", &args, working_dir).await
                }
                "read" => {
                    let path = tc.param("path").unwrap_or("");
                    let lines = tc.param("lines");
                    let args = match lines {
                        Some(l) => serde_json::json!({"path": path, "lines": l}),
                        None => serde_json::json!({"path": path}),
                    };
                    tools::execute_tool("read", &args, working_dir).await
                }
                "list_directory" => {
                    let path = tc.param("path").or_else(|| tc.param("command")).unwrap_or(".");
                    let args = serde_json::json!({"path": path});
                    tools::execute_tool("list_directory", &args, working_dir).await
                }
                _ => continue,
            };

            let trimmed = result.trim();
            if !trimmed.is_empty() {
                formatted_results.push(format!("<tool_response>\n{trimmed}\n</tool_response>"));
            }
        }

        if !formatted_results.is_empty() {
            let turn_msg = turn_message(turn, MAX_TURNS);
            let budget = context_budget_tag(&messages);
            let combined = format!("{}{turn_msg}\n{budget}", formatted_results.join("\n"));
            messages.push(Message {
                role: "user".to_string(),
                content: combined,
            });
        }
    }

    Ok("Search completed but did not converge within the turn limit.".to_string())
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_xml_tool_calls() {
        let text = r#"
<tool_call>
<function=ripgrep>
<parameter=pattern>macos|darwin</parameter>
<parameter=path>crates/</parameter>
<parameter=glob>*.rs</parameter>
</function>
</tool_call>
<tool_call>
<function=read>
<parameter=path>src/main.rs</parameter>
<parameter=lines>1-50</parameter>
</function>
</tool_call>
"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 2);
        assert_eq!(calls[0].name, "ripgrep");
        assert_eq!(calls[0].param("pattern"), Some("macos|darwin"));
        assert_eq!(calls[0].param("path"), Some("crates/"));
        assert_eq!(calls[0].param("glob"), Some("*.rs"));
        assert_eq!(calls[1].name, "read");
        assert_eq!(calls[1].param("path"), Some("src/main.rs"));
        assert_eq!(calls[1].param("lines"), Some("1-50"));
    }

    #[test]
    fn parse_xml_with_think_block() {
        let text = r#"<think>I should search for macos</think>
<tool_call>
<function=ripgrep>
<parameter=pattern>macos</parameter>
<parameter=path>.</parameter>
</function>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].name, "ripgrep");
    }

    #[test]
    fn parse_xml_ignores_invalid_tools() {
        let text = r#"
<tool_call>
<function=evil_tool>
<parameter=path>.</parameter>
</function>
</tool_call>"#;
        let calls = parse_tool_calls(text);
        assert!(calls.is_empty());
    }

    #[test]
    fn parse_finish_with_ranges() {
        let specs = parse_finish_files("src/main.rs:1-50,100-120\nlib.rs");
        assert_eq!(specs.len(), 2);
        assert_eq!(specs[0].path, "src/main.rs");
        assert_eq!(specs[0].ranges, vec![(1, 50), (100, 120)]);
        assert_eq!(specs[1].path, "lib.rs");
        assert!(specs[1].ranges.is_empty());
    }

    #[test]
    fn parse_finish_empty_and_star() {
        let specs = parse_finish_files("config.rs:*");
        assert_eq!(specs.len(), 1);
        assert!(specs[0].ranges.is_empty());
    }

    #[test]
    fn resolve_finish_with_temp_file() {
        let dir = tempfile::tempdir().unwrap();
        let content: String = (1..=20).map(|i| format!("line {i}\n")).collect();
        std::fs::write(dir.path().join("test.rs"), &content).unwrap();

        let specs = vec![FileSpec {
            path: "test.rs".to_string(),
            ranges: vec![(3, 5), (10, 12)],
        }];
        let result = resolve_finish(&specs, dir.path());
        assert!(result.contains("--- test.rs ---"));
        assert!(result.contains("3 | line 3"));
        assert!(result.contains("5 | line 5"));
        assert!(result.contains("10 | line 10"));
        assert!(result.contains("// ... existing code"));
        assert!(!result.contains("6 | line 6"));
    }

    #[test]
    fn resolve_finish_missing_file() {
        let dir = tempfile::tempdir().unwrap();
        let specs = vec![FileSpec {
            path: "nonexistent.rs".to_string(),
            ranges: vec![(1, 5)],
        }];
        let result = resolve_finish(&specs, dir.path());
        assert!(result.contains("error"));
    }

    #[test]
    fn context_budget_format() {
        let msgs = vec![Message {
            role: "user".to_string(),
            content: "x".repeat(54_000),
        }];
        let tag = context_budget_tag(&msgs);
        assert!(tag.contains("10%"));
        assert!(tag.contains("54K/540K"));
    }

    #[test]
    fn enforce_context_limit_truncates_old_tool_results() {
        let mut messages = vec![
            Message {
                role: "user".to_string(),
                content: "initial query".to_string(),
            },
            Message {
                role: "assistant".to_string(),
                content: "response".to_string(),
            },
            Message {
                role: "user".to_string(),
                content: "x".repeat(MAX_CONTEXT_CHARS),
            },
        ];
        enforce_context_limit(&mut messages);
        // First user message preserved, second user message truncated
        assert_eq!(messages[0].content, "initial query");
        assert_eq!(messages[2].content, "[truncated for context limit]");
    }

    #[test]
    fn turn_message_format() {
        assert!(turn_message(1, 4).contains("3 remaining"));
        assert!(turn_message(3, 4).contains("MUST call the finish tool"));
    }
}
