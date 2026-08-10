//! Shared text-based tool call emulation for local inference backends.
//!
//! Models that do not have native tool-calling support are prompted to emit shell commands
//! as `$ command` on a new line and code blocks as ```execute_typescript fenced blocks.
//! The parser converts those patterns into Goose tool-call messages.

#[cfg(feature = "mlx")]
use goose_provider_types::conversation::message::{Message, MessageContent};
#[cfg(feature = "mlx")]
use rmcp::model::{CallToolRequestParams, Tool};
#[cfg(feature = "mlx")]
use serde_json::json;
#[cfg(feature = "mlx")]
use std::borrow::Cow;
#[cfg(feature = "mlx")]
use uuid::Uuid;

#[cfg(feature = "mlx")]
pub(crate) const SHELL_TOOL: &str = "developer__shell";
#[cfg(feature = "mlx")]
pub(crate) const CODE_EXECUTION_TOOL: &str = "code_execution__execute_typescript";

#[cfg(feature = "mlx")]
pub(crate) fn load_tiny_model_prompt() -> String {
    use std::env;

    let os = if cfg!(target_os = "macos") {
        "macos"
    } else if cfg!(target_os = "linux") {
        "linux"
    } else if cfg!(target_os = "windows") {
        "windows"
    } else {
        "unknown"
    };

    let working_directory = env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "unknown".to_string());

    let shell = env::var("SHELL").unwrap_or_else(|_| "/bin/sh".to_string());

    let context = json!({
        "os": os,
        "working_directory": working_directory,
        "shell": shell,
    });

    crate::prompt_template::render_template("tiny_model_system.md", &context).unwrap_or_else(|e| {
        tracing::warn!("Failed to load tiny_model_system.md: {:?}", e);
        "You are Goose, an AI assistant. You can execute shell commands by starting lines with $."
            .to_string()
    })
}

#[cfg(feature = "mlx")]
pub(crate) fn build_emulator_tool_description(tools: &[Tool], code_mode_enabled: bool) -> String {
    let mut tool_desc = String::new();

    if code_mode_enabled {
        tool_desc.push_str("\n\n# Running Code\n\n");
        tool_desc.push_str(
            "You can call tools by writing code in a ```execute_typescript block. \
             The code runs immediately — do not explain it, just run it.\n\n",
        );
        tool_desc.push_str("Example — counting files in /tmp:\n\n");
        tool_desc.push_str("```execute_typescript\nasync function run() {\n");
        tool_desc.push_str(
            "  const result = await Developer.shell({ command: \"ls -1 /tmp | wc -l\" });\n",
        );
        tool_desc.push_str("  return result;\n}\n```\n\n");
        tool_desc.push_str("Rules:\n");
        tool_desc.push_str("- Code MUST define async function run() and return a result\n");
        tool_desc.push_str("- All function calls are async — use await\n");
        tool_desc.push_str(
            "- Use ```execute_typescript for tool calls, $ for simple shell one-liners\n\n",
        );
        tool_desc.push_str("Available functions:\n\n");

        for tool in tools {
            if tool.name.starts_with("code_execution__") {
                continue;
            }
            let parts: Vec<&str> = tool.name.splitn(2, "__").collect();
            if parts.len() == 2 {
                let namespace = {
                    let mut c = parts[0].chars();
                    match c.next() {
                        None => String::new(),
                        Some(first) => first.to_uppercase().chain(c).collect::<String>(),
                    }
                };
                let camel_name: String = parts[1]
                    .split('_')
                    .enumerate()
                    .map(|(i, part)| {
                        if i == 0 {
                            part.to_string()
                        } else {
                            let mut c = part.chars();
                            match c.next() {
                                None => String::new(),
                                Some(first) => first.to_uppercase().chain(c).collect(),
                            }
                        }
                    })
                    .collect();
                let desc = tool.description.as_ref().map(|d| d.as_ref()).unwrap_or("");
                tool_desc.push_str(&format!("- {namespace}.{camel_name}(): {desc}\n"));
            }
        }
    } else {
        tool_desc.push_str("\n\n# Tools\n\nYou have access to the following tools:\n\n");
        for tool in tools {
            let desc = tool
                .description
                .as_ref()
                .map(|d| d.as_ref())
                .unwrap_or("No description");
            tool_desc.push_str(&format!("- {}: {}\n", tool.name, desc));
        }
    }

    tool_desc
}

pub(crate) enum EmulatorAction {
    Text(String),
    ShellCommand(String),
    ExecuteCode(String),
}

#[derive(Clone, Copy)]
enum ParserState {
    Normal,
    InCommand,
    InExecuteBlock { fence_len: usize },
    InMarkdownFence { marker: char, fence_len: usize },
}

pub(crate) struct StreamingEmulatorParser {
    buffer: String,
    state: ParserState,
    code_mode_enabled: bool,
    at_line_start: bool,
}

struct Fence<'a> {
    marker: char,
    len: usize,
    info: &'a str,
}

#[allow(clippy::string_slice)]
fn parse_fence(line: &str) -> Option<Fence<'_>> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return None;
    }

    let rest = &line[indent..];
    let marker = rest.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }

    let len = rest.chars().take_while(|ch| *ch == marker).count();
    if len < 3 {
        return None;
    }

    Some(Fence {
        marker,
        len,
        info: rest[len..].trim(),
    })
}

fn is_closing_fence(line: &str, marker: char, minimum_len: usize) -> bool {
    parse_fence(line).is_some_and(|fence| {
        fence.marker == marker && fence.len >= minimum_len && fence.info.is_empty()
    })
}

#[allow(clippy::string_slice)]
fn could_be_control_line(line: &str, marker: Option<char>) -> bool {
    let indent = line.bytes().take_while(|byte| *byte == b' ').count();
    if indent > 3 {
        return false;
    }

    let rest = &line[indent..];
    match marker {
        Some(marker) => {
            let marker_len = rest.chars().take_while(|ch| *ch == marker).count();
            marker_len == rest.chars().count()
                || (marker_len >= 3
                    && rest[marker_len..]
                        .chars()
                        .all(|ch| ch == ' ' || ch == '\t' || ch == '\r'))
        }
        None => {
            rest.is_empty()
                || "$".starts_with(rest)
                || rest.starts_with('`')
                || rest.starts_with('~')
        }
    }
}

#[allow(clippy::string_slice)]
fn closing_fence_range(input: &str, marker: char, minimum_len: usize) -> Option<(usize, usize)> {
    let mut line_start = 0;
    loop {
        let remaining = &input[line_start..];
        let line_end = remaining
            .find('\n')
            .map(|offset| line_start + offset)
            .unwrap_or(input.len());
        if is_closing_fence(&input[line_start..line_end], marker, minimum_len) {
            let consumed = if line_end < input.len() {
                line_end + 1
            } else {
                line_end
            };
            return Some((line_start, consumed));
        }
        if line_end == input.len() {
            return None;
        }
        line_start = line_end + 1;
    }
}

impl StreamingEmulatorParser {
    pub(crate) fn new(code_mode_enabled: bool) -> Self {
        Self {
            buffer: String::new(),
            state: ParserState::Normal,
            code_mode_enabled,
            at_line_start: true,
        }
    }

    #[allow(clippy::string_slice)]
    pub(crate) fn process_chunk(&mut self, chunk: &str) -> Vec<EmulatorAction> {
        self.buffer.push_str(chunk);
        let mut results = Vec::new();

        loop {
            match self.state {
                ParserState::InCommand => {
                    if let Some((command_line, rest)) = self.buffer.split_once('\n') {
                        if let Some(command) = command_line.strip_prefix('$') {
                            let command = command.trim();
                            if !command.is_empty() {
                                results.push(EmulatorAction::ShellCommand(command.to_string()));
                            }
                        }
                        self.buffer = rest.to_string();
                        self.state = ParserState::Normal;
                    } else {
                        break;
                    }
                }
                ParserState::InExecuteBlock { fence_len } => {
                    if let Some((closing_start, consumed)) =
                        closing_fence_range(&self.buffer, '`', fence_len)
                    {
                        let code_end = closing_start
                            .checked_sub(1)
                            .filter(|index| self.buffer.as_bytes()[*index] == b'\n')
                            .unwrap_or(closing_start);
                        let code = self.buffer[..code_end]
                            .strip_suffix('\r')
                            .unwrap_or(&self.buffer[..code_end])
                            .to_string();
                        self.buffer = self.buffer[consumed..].to_string();
                        self.state = ParserState::Normal;
                        self.at_line_start = true;
                        if !code.trim().is_empty() {
                            results.push(EmulatorAction::ExecuteCode(code));
                        }
                    } else {
                        break;
                    }
                }
                ParserState::InMarkdownFence { marker, fence_len } => {
                    if !self.at_line_start {
                        if let Some(end_idx) = self.buffer.find('\n') {
                            results.push(EmulatorAction::Text(self.buffer[..=end_idx].to_string()));
                            self.buffer = self.buffer[end_idx + 1..].to_string();
                            self.at_line_start = true;
                            continue;
                        }
                        if !self.buffer.is_empty() {
                            results.push(EmulatorAction::Text(std::mem::take(&mut self.buffer)));
                        }
                        break;
                    }

                    if let Some(end_idx) = self.buffer.find('\n') {
                        let line = self.buffer[..end_idx].to_string();
                        let closes = is_closing_fence(&line, marker, fence_len);
                        results.push(EmulatorAction::Text(self.buffer[..=end_idx].to_string()));
                        self.buffer = self.buffer[end_idx + 1..].to_string();
                        if closes {
                            self.state = ParserState::Normal;
                        }
                        self.at_line_start = true;
                        continue;
                    }

                    if could_be_control_line(&self.buffer, Some(marker)) {
                        break;
                    }
                    if !self.buffer.is_empty() {
                        results.push(EmulatorAction::Text(std::mem::take(&mut self.buffer)));
                        self.at_line_start = false;
                    }
                    break;
                }
                ParserState::Normal => {
                    if !self.at_line_start {
                        if let Some(end_idx) = self.buffer.find('\n') {
                            results.push(EmulatorAction::Text(self.buffer[..=end_idx].to_string()));
                            self.buffer = self.buffer[end_idx + 1..].to_string();
                            self.at_line_start = true;
                            continue;
                        }
                        if !self.buffer.is_empty() {
                            results.push(EmulatorAction::Text(std::mem::take(&mut self.buffer)));
                        }
                        break;
                    }

                    if let Some(end_idx) = self.buffer.find('\n') {
                        let line = self.buffer[..end_idx].to_string();
                        let line_with_newline = self.buffer[..=end_idx].to_string();
                        self.buffer = self.buffer[end_idx + 1..].to_string();

                        if self.code_mode_enabled {
                            if let Some(fence) = parse_fence(&line) {
                                if fence.marker == '`' && fence.info == "execute_typescript" {
                                    self.state = ParserState::InExecuteBlock {
                                        fence_len: fence.len,
                                    };
                                    self.at_line_start = true;
                                    continue;
                                }
                                results.push(EmulatorAction::Text(line_with_newline));
                                self.state = ParserState::InMarkdownFence {
                                    marker: fence.marker,
                                    fence_len: fence.len,
                                };
                                self.at_line_start = true;
                                continue;
                            }
                        }

                        if let Some(command) = line.strip_prefix('$') {
                            let command = command.trim();
                            if !command.is_empty() {
                                results.push(EmulatorAction::ShellCommand(command.to_string()));
                            }
                        } else {
                            results.push(EmulatorAction::Text(line_with_newline));
                        }
                        self.at_line_start = true;
                        continue;
                    }

                    if self.buffer.starts_with('$') {
                        self.state = ParserState::InCommand;
                        continue;
                    }
                    if could_be_control_line(&self.buffer, None) {
                        break;
                    }
                    if !self.buffer.is_empty() {
                        results.push(EmulatorAction::Text(std::mem::take(&mut self.buffer)));
                        self.at_line_start = false;
                    }
                    break;
                }
            }
        }

        results
    }

    pub(crate) fn flush(&mut self) -> Vec<EmulatorAction> {
        let mut results = Vec::new();

        if !self.buffer.is_empty() {
            match self.state {
                ParserState::InCommand => {
                    let command_line = self.buffer.trim();
                    if let Some(command) = command_line.strip_prefix('$') {
                        let command = command.trim();
                        if !command.is_empty() {
                            results.push(EmulatorAction::ShellCommand(command.to_string()));
                        }
                    } else if !command_line.is_empty() {
                        results.push(EmulatorAction::Text(self.buffer.clone()));
                    }
                }
                ParserState::InExecuteBlock { .. } => {
                    let code = self.buffer.trim();
                    if !code.is_empty() {
                        results.push(EmulatorAction::ExecuteCode(code.to_string()));
                    }
                }
                ParserState::Normal | ParserState::InMarkdownFence { .. } => {
                    results.push(EmulatorAction::Text(self.buffer.clone()));
                }
            }
            self.buffer.clear();
            self.state = ParserState::Normal;
            self.at_line_start = true;
        }

        results
    }
}

#[cfg(feature = "mlx")]
pub(crate) fn message_for_emulator_action(
    action: &EmulatorAction,
    message_id: &str,
) -> (Message, bool) {
    match action {
        EmulatorAction::Text(text) => {
            let mut message = Message::assistant().with_text(text);
            message.id = Some(message_id.to_string());
            (message, false)
        }
        EmulatorAction::ShellCommand(command) => {
            let tool_id = Uuid::new_v4().to_string();
            let mut args = serde_json::Map::new();
            args.insert("command".to_string(), json!(command));
            let tool_call =
                CallToolRequestParams::new(Cow::Borrowed(SHELL_TOOL)).with_arguments(args);
            let mut message = Message::assistant();
            message
                .content
                .push(MessageContent::tool_request(tool_id, Ok(tool_call)));
            message.id = Some(message_id.to_string());
            (message, true)
        }
        EmulatorAction::ExecuteCode(code) => {
            let tool_id = Uuid::new_v4().to_string();
            let wrapped = if code.contains("async function run()") {
                code.clone()
            } else {
                format!("async function run() {{\n{}\n}}", code)
            };
            let mut args = serde_json::Map::new();
            args.insert("code".to_string(), json!(wrapped));
            let tool_call =
                CallToolRequestParams::new(Cow::Borrowed(CODE_EXECUTION_TOOL)).with_arguments(args);
            let mut message = Message::assistant();
            message
                .content
                .push(MessageContent::tool_request(tool_id, Ok(tool_call)));
            message.id = Some(message_id.to_string());
            (message, true)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_chunks(chunks: &[&str], code_mode: bool) -> Vec<EmulatorAction> {
        let mut parser = StreamingEmulatorParser::new(code_mode);
        let mut actions = Vec::new();
        for chunk in chunks {
            actions.extend(parser.process_chunk(chunk));
        }
        actions.extend(parser.flush());
        actions
    }

    fn parse_all(input: &str, code_mode: bool) -> Vec<EmulatorAction> {
        parse_chunks(&[input], code_mode)
    }

    fn assert_text(action: &EmulatorAction, expected: &str) {
        match action {
            EmulatorAction::Text(t) => assert_eq!(t.trim(), expected.trim(), "text mismatch"),
            other => panic!("expected Text, got {:?}", action_label(other)),
        }
    }

    fn assert_shell(action: &EmulatorAction, expected: &str) {
        match action {
            EmulatorAction::ShellCommand(cmd) => {
                assert_eq!(cmd, expected, "shell command mismatch")
            }
            other => panic!("expected ShellCommand, got {:?}", action_label(other)),
        }
    }

    fn assert_execute(action: &EmulatorAction, expected: &str) {
        match action {
            EmulatorAction::ExecuteCode(code) => {
                assert_eq!(code.trim(), expected.trim(), "execute code mismatch")
            }
            other => panic!("expected ExecuteCode, got {:?}", action_label(other)),
        }
    }

    fn action_label(a: &EmulatorAction) -> &'static str {
        match a {
            EmulatorAction::Text(_) => "Text",
            EmulatorAction::ShellCommand(_) => "ShellCommand",
            EmulatorAction::ExecuteCode(_) => "ExecuteCode",
        }
    }

    #[test]
    fn plain_text_no_tools() {
        let actions = parse_all("Hello, world!", false);
        let all_text: String = actions
            .iter()
            .map(|a| match a {
                EmulatorAction::Text(t) => t.as_str(),
                _ => panic!("expected only Text actions"),
            })
            .collect();
        assert_eq!(all_text.trim(), "Hello, world!");
    }

    #[test]
    fn single_shell_command() {
        let actions = parse_all("$ ls -la\n", false);
        assert_eq!(actions.len(), 1);
        assert_shell(&actions[0], "ls -la");
    }

    #[test]
    fn text_then_shell_command() {
        let actions = parse_all("Let me check:\n$ ls -la\n", false);
        assert!(actions.len() >= 2);
        assert_text(&actions[0], "Let me check:");
        assert_shell(&actions[actions.len() - 1], "ls -la");
    }

    #[test]
    fn shell_command_at_start_of_output() {
        let actions = parse_all("$ whoami\n", false);
        assert_eq!(actions.len(), 1);
        assert_shell(&actions[0], "whoami");
    }

    #[test]
    fn shell_command_without_trailing_newline() {
        let actions = parse_all("$ whoami", false);
        assert_eq!(actions.len(), 1);
        assert_shell(&actions[0], "whoami");
    }

    #[test]
    fn dollar_sign_mid_sentence_is_not_command() {
        let actions = parse_all("It costs $50 per month", false);
        for action in &actions {
            assert!(matches!(action, EmulatorAction::Text(_)));
        }
        let all_text: String = actions
            .iter()
            .filter_map(|a| match a {
                EmulatorAction::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(all_text.trim(), "It costs $50 per month");
    }

    #[test]
    fn execute_block() {
        let input = "Here's the code:\n```execute_typescript\nconsole.log('hi');\n```\n";
        let actions = parse_all(input, true);
        assert!(actions.len() >= 2);
        assert_text(&actions[0], "Here's the code:");
        assert_execute(&actions[actions.len() - 1], "console.log('hi');");
    }

    #[test]
    #[cfg(feature = "mlx")]
    fn tool_description_uses_parser_execute_fence() {
        let description = build_emulator_tool_description(&[], true);

        assert!(description.contains("```execute_typescript"));
        assert!(!description.contains("```execute block"));
        assert!(!description.contains("Use ```execute for tool calls"));
    }

    #[test]
    fn execute_block_not_detected_without_code_mode() {
        let input = "```execute_typescript\nconsole.log('hi');\n```\n";
        let actions = parse_all(input, false);
        for action in &actions {
            assert!(matches!(action, EmulatorAction::Text(_)));
        }
    }

    #[test]
    fn dollar_split_across_chunks() {
        let actions = parse_chunks(&["Let me check\n", "$ ls -la\n"], false);
        let shells: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, EmulatorAction::ShellCommand(_)))
            .collect();
        assert_eq!(shells.len(), 1);
        assert_shell(shells[0], "ls -la");
    }

    #[test]
    fn execute_fence_split_across_chunks() {
        let actions = parse_chunks(
            &["Here:\n```ex", "ecute_typescript\nlet x = 1;\n", "```\n"],
            true,
        );
        let executes: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, EmulatorAction::ExecuteCode(_)))
            .collect();
        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let x = 1;");
    }

    #[test]
    fn multiple_commands_on_separate_lines() {
        let actions = parse_chunks(&["Here:\n$ cd /tmp\n", "Done.\n$ ls\n"], false);
        let shells: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, EmulatorAction::ShellCommand(_)))
            .collect();
        assert_eq!(shells.len(), 2);
        assert_shell(shells[0], "cd /tmp");
        assert_shell(shells[1], "ls");
    }

    #[test]
    fn regular_code_fence_not_treated_as_execute() {
        let input = "```python\nprint('hi')\n```\n";
        let actions = parse_all(input, true);
        for action in &actions {
            assert!(matches!(action, EmulatorAction::Text(_)));
        }
    }

    #[test]
    fn execute_fence_nested_in_longer_markdown_fence_remains_text() {
        let input = "````markdown\nquoted output:\n```execute_typescript\nawait Developer.shell({ command: \"id\" });\n```\n````\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);

        assert!(actions
            .iter()
            .all(|action| matches!(action, EmulatorAction::Text(_))));
        let text: String = actions
            .iter()
            .filter_map(|action| match action {
                EmulatorAction::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(text, input);
    }

    #[test]
    fn execute_fence_nested_in_tilde_fence_remains_text() {
        let input = "~~~markdown\n```execute_typescript\nawait Developer.shell({ command: \"id\" });\n```\n~~~\n";
        let actions = parse_all(input, true);

        assert!(actions
            .iter()
            .all(|action| matches!(action, EmulatorAction::Text(_))));
    }

    #[test]
    fn longer_top_level_execute_fence_is_recognized() {
        let input = "````execute_typescript\nlet x = 1;\n````\n";
        let actions = parse_all(input, true);
        let executes: Vec<_> = actions
            .iter()
            .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
            .collect();

        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let x = 1;");
    }

    #[test]
    fn closing_fence_with_split_trailing_spaces_is_recognized() {
        let actions = parse_chunks(
            &["```execute_typescript\nlet x = 1;\n```", "  ", "\n"],
            true,
        );
        let executes: Vec<_> = actions
            .iter()
            .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
            .collect();

        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let x = 1;");
    }

    #[test]
    fn empty_command_ignored() {
        let actions = parse_all("$\n", false);
        let shells: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, EmulatorAction::ShellCommand(_)))
            .collect();
        assert_eq!(shells.len(), 0);
    }

    #[test]
    fn token_by_token_streaming() {
        let input = "$ echo hello\n";
        let chars: Vec<String> = input.chars().map(|c| c.to_string()).collect();
        let chunks: Vec<&str> = chars.iter().map(|s| s.as_str()).collect();
        let actions = parse_chunks(&chunks, false);
        let shells: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, EmulatorAction::ShellCommand(_)))
            .collect();
        assert_eq!(shells.len(), 1);
        assert_shell(shells[0], "echo hello");
    }

    #[test]
    fn execute_block_with_multiline_code() {
        let input = "```execute_typescript\nasync function run() {\n  const r = await Developer.shell({ command: \"ls\" });\n  return r;\n}\n```\n";
        let actions = parse_all(input, true);
        let executes: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, EmulatorAction::ExecuteCode(_)))
            .collect();
        assert_eq!(executes.len(), 1);
        match executes[0] {
            EmulatorAction::ExecuteCode(code) => {
                assert!(code.contains("async function run()"));
                assert!(code.contains("Developer.shell"));
            }
            _ => unreachable!(),
        }
    }

    #[test]
    fn unclosed_execute_block_flushed() {
        let input = "```execute_typescript\nlet x = 1;";
        let actions = parse_all(input, true);
        let executes: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, EmulatorAction::ExecuteCode(_)))
            .collect();
        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let x = 1;");
    }
}
