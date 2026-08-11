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
    InExecuteBlock {
        fence_len: usize,
    },
    InMarkdownFence {
        marker: char,
        fence_len: usize,
        container_indent: Option<usize>,
    },
}

pub(crate) struct StreamingEmulatorParser {
    buffer: String,
    state: ParserState,
    code_mode_enabled: bool,
    at_line_start: bool,
    list_container_indent: Option<usize>,
    empty_list_item_indent: Option<usize>,
}

struct Fence<'a> {
    marker: char,
    len: usize,
    info: &'a str,
    container_indent: Option<usize>,
}

fn leading_indent(line: &str) -> (usize, usize) {
    let mut bytes = 0;
    let mut columns = 0;
    for byte in line.bytes() {
        match byte {
            b' ' => columns += 1,
            b'\t' => columns += 4 - (columns % 4),
            _ => break,
        }
        bytes += 1;
    }
    (bytes, columns)
}

#[allow(clippy::string_slice)]
fn parse_fence_marker(rest: &str, container_indent: Option<usize>) -> Option<Fence<'_>> {
    let marker = rest.chars().next()?;
    if marker != '`' && marker != '~' {
        return None;
    }

    let len = rest.chars().take_while(|ch| *ch == marker).count();
    if len < 3 {
        return None;
    }

    let info = rest[len..].trim_matches([' ', '\t', '\r']);
    if marker == '`' && info.contains('`') {
        return None;
    }

    Some(Fence {
        marker,
        len,
        info,
        container_indent,
    })
}

#[allow(clippy::string_slice)]
fn parse_direct_fence(
    line: &str,
    minimum_indent: usize,
    maximum_indent: usize,
    container_indent: Option<usize>,
) -> Option<Fence<'_>> {
    let (indent_bytes, indent_columns) = leading_indent(line);
    if indent_columns < minimum_indent || indent_columns > maximum_indent {
        return None;
    }

    parse_fence_marker(&line[indent_bytes..], container_indent)
}

fn list_marker_width(line: &str) -> Option<usize> {
    let bytes = line.as_bytes();
    let marker_end = match bytes.first()? {
        b'-' | b'+' | b'*' => 1,
        byte if byte.is_ascii_digit() => {
            let digits = bytes
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            if digits > 9 || !matches!(bytes.get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
        _ => return None,
    };

    let whitespace = bytes[marker_end..]
        .iter()
        .take_while(|byte| **byte == b' ')
        .count();
    (1..=4)
        .contains(&whitespace)
        .then_some(marker_end + whitespace)
}

#[allow(clippy::string_slice)]
fn empty_list_item_indents(line: &str) -> Option<(usize, usize)> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let (indent_bytes, marker_indent) = leading_indent(line);
    if marker_indent > 3 {
        return None;
    }
    let rest = &line[indent_bytes..];
    let bytes = rest.as_bytes();
    let marker_end = match bytes.first()? {
        b'-' | b'+' | b'*' => 1,
        byte if byte.is_ascii_digit() => {
            let digits = bytes
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            if digits > 9 || !matches!(bytes.get(digits), Some(b'.' | b')')) {
                return None;
            }
            digits + 1
        }
        _ => return None,
    };
    rest[marker_end..]
        .chars()
        .all(|ch| ch == ' ' || ch == '\t')
        .then_some((marker_indent, marker_indent + marker_end + 1))
}

#[allow(clippy::string_slice)]
fn is_list_thematic_break_candidate(line: &str) -> bool {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let (indent_bytes, marker_indent) = leading_indent(line);
    if marker_indent > 3 {
        return false;
    }
    let rest = &line[indent_bytes..];
    let Some(marker @ ('*' | '-')) = rest.chars().next() else {
        return false;
    };
    if list_marker_width(rest).is_none() {
        return false;
    }
    rest.chars()
        .all(|ch| ch == marker || ch == ' ' || ch == '\t' || ch == '\r')
}

fn is_list_thematic_break(line: &str) -> bool {
    is_list_thematic_break_candidate(line)
        && line.chars().filter(|ch| matches!(ch, '*' | '-')).count() >= 3
}

#[allow(clippy::string_slice)]
fn list_item_indents(line: &str) -> Option<(usize, usize)> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    if is_list_thematic_break(line) {
        return None;
    }
    let (indent_bytes, marker_indent) = leading_indent(line);
    if marker_indent > 3 {
        return None;
    }
    let marker_width = list_marker_width(&line[indent_bytes..])?;
    Some((marker_indent, marker_indent + marker_width))
}

fn parse_fence_in_container(line: &str, container_indent: Option<usize>) -> Option<Fence<'_>> {
    if let Some(container_indent) = container_indent {
        if let Some(fence) = parse_direct_fence(
            line,
            container_indent,
            container_indent + 3,
            Some(container_indent),
        ) {
            return Some(fence);
        }
    }
    parse_fence(line)
}

#[allow(clippy::string_slice)]
fn parse_fence(line: &str) -> Option<Fence<'_>> {
    let line = line.strip_suffix('\r').unwrap_or(line);
    if let Some(fence) = parse_direct_fence(line, 0, 3, None) {
        return Some(fence);
    }

    let (indent_bytes, indent_columns) = leading_indent(line);
    if indent_columns > 3 {
        return None;
    }
    let rest = &line[indent_bytes..];
    let list_width = list_marker_width(rest)?;
    let container_indent = indent_columns + list_width;
    parse_fence_marker(&rest[list_width..], Some(container_indent))
}

fn is_closing_fence(
    line: &str,
    marker: char,
    minimum_len: usize,
    container_indent: Option<usize>,
) -> bool {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let minimum_indent = container_indent.unwrap_or(0);
    parse_direct_fence(line, minimum_indent, minimum_indent + 3, container_indent).is_some_and(
        |fence| fence.marker == marker && fence.len >= minimum_len && fence.info.is_empty(),
    )
}

fn could_be_fence_prefix(rest: &str) -> bool {
    let Some(marker @ ('`' | '~')) = rest.chars().next() else {
        return false;
    };
    let marker_len = rest.chars().take_while(|ch| *ch == marker).count();
    if marker_len < 3 {
        return marker_len == rest.chars().count();
    }

    marker != '`'
        || !rest
            .get(marker_len..)
            .is_some_and(|info| info.contains('`'))
}

#[allow(clippy::string_slice)]
fn could_be_list_fence(line: &str) -> bool {
    let bytes = line.as_bytes();
    let marker_end = match bytes.first() {
        Some(b'-' | b'+' | b'*') => 1,
        Some(byte) if byte.is_ascii_digit() => {
            let digits = bytes
                .iter()
                .take_while(|byte| byte.is_ascii_digit())
                .count();
            if digits == bytes.len() {
                return digits <= 9;
            }
            if digits > 9 || !matches!(bytes.get(digits), Some(b'.' | b')')) {
                return false;
            }
            digits + 1
        }
        _ => return false,
    };

    if marker_end == bytes.len() {
        return true;
    }
    let whitespace = bytes[marker_end..]
        .iter()
        .take_while(|byte| **byte == b' ')
        .count();
    if !(1..=4).contains(&whitespace) {
        return false;
    }
    let rest = &line[marker_end + whitespace..];
    rest.is_empty() || could_be_fence_prefix(rest)
}

#[allow(clippy::string_slice)]
fn could_be_control_line(
    line: &str,
    marker: Option<char>,
    container_indent: Option<usize>,
) -> bool {
    let (indent_bytes, indent_columns) = leading_indent(line);
    let minimum_indent = container_indent.unwrap_or(0);
    if indent_columns > minimum_indent + 3 {
        return false;
    }
    if indent_columns < minimum_indent {
        return indent_bytes == line.len();
    }

    let rest = &line[indent_bytes..];
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
                || could_be_fence_prefix(rest)
                || empty_list_item_indents(line).is_some()
                || is_list_thematic_break_candidate(line)
                || could_be_list_fence(rest)
        }
    }
}

#[allow(clippy::string_slice)]
fn closing_fence_range(
    input: &str,
    marker: char,
    minimum_len: usize,
    container_indent: Option<usize>,
    allow_end_of_stream: bool,
) -> Option<(usize, usize)> {
    let mut line_start = 0;
    loop {
        let remaining = &input[line_start..];
        let newline_offset = remaining.find('\n');
        if newline_offset.is_none() && !allow_end_of_stream {
            return None;
        }
        let line_end = newline_offset
            .map(|offset| line_start + offset)
            .unwrap_or(input.len());
        if is_closing_fence(
            &input[line_start..line_end],
            marker,
            minimum_len,
            container_indent,
        ) {
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
            list_container_indent: None,
            empty_list_item_indent: None,
        }
    }

    fn update_list_container(&mut self, line: &str, complete_line: bool) {
        if line.trim().is_empty() {
            if complete_line {
                if self.empty_list_item_indent == self.list_container_indent {
                    self.list_container_indent = None;
                }
                self.empty_list_item_indent = None;
            }
            return;
        }

        if complete_line {
            if let Some((marker_indent, content_indent)) = empty_list_item_indents(line) {
                self.list_container_indent = match self.list_container_indent {
                    Some(outer_indent) if marker_indent >= outer_indent => Some(outer_indent),
                    _ => Some(content_indent),
                };
                self.empty_list_item_indent = Some(content_indent);
                return;
            }
        }

        if let Some((marker_indent, content_indent)) = list_item_indents(line) {
            if complete_line || line.len() > leading_indent(line).0 + content_indent - marker_indent
            {
                self.list_container_indent = match self.list_container_indent {
                    Some(outer_indent) if marker_indent >= outer_indent => Some(outer_indent),
                    _ => Some(content_indent),
                };
                self.empty_list_item_indent = None;
                return;
            }
        }

        if let Some(empty_indent) = self.empty_list_item_indent.take() {
            let line_indent = leading_indent(line).1;
            if line_indent < empty_indent && self.list_container_indent == Some(empty_indent) {
                self.list_container_indent = None;
            }
        }

        if self
            .list_container_indent
            .is_some_and(|container_indent| leading_indent(line).1 < container_indent)
        {
            self.list_container_indent = None;
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
                        closing_fence_range(&self.buffer, '`', fence_len, None, false)
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
                ParserState::InMarkdownFence {
                    marker,
                    fence_len,
                    container_indent,
                } => {
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
                        if container_indent.is_some_and(|minimum_indent| {
                            let (_, indent_columns) = leading_indent(&line);
                            !line.trim().is_empty() && indent_columns < minimum_indent
                        }) {
                            self.state = ParserState::Normal;
                            continue;
                        }
                        let closes = is_closing_fence(&line, marker, fence_len, container_indent);
                        results.push(EmulatorAction::Text(self.buffer[..=end_idx].to_string()));
                        self.buffer = self.buffer[end_idx + 1..].to_string();
                        if closes {
                            self.state = ParserState::Normal;
                        }
                        self.at_line_start = true;
                        continue;
                    }

                    if container_indent.is_some_and(|minimum_indent| {
                        let (indent_bytes, indent_columns) = leading_indent(&self.buffer);
                        indent_columns < minimum_indent && indent_bytes < self.buffer.len()
                    }) {
                        self.state = ParserState::Normal;
                        continue;
                    }
                    if could_be_control_line(&self.buffer, Some(marker), container_indent) {
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
                            self.update_list_container(&line, true);
                            if let Some(fence) =
                                parse_fence_in_container(&line, self.list_container_indent)
                            {
                                if fence.container_indent.is_none()
                                    && fence.marker == '`'
                                    && fence.info == "execute_typescript"
                                {
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
                                    container_indent: fence.container_indent,
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

                    if self.code_mode_enabled {
                        let line = self.buffer.clone();
                        self.update_list_container(&line, false);
                    }
                    if self.buffer.starts_with('$') {
                        self.state = ParserState::InCommand;
                        continue;
                    }
                    if could_be_control_line(&self.buffer, None, None) {
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
                ParserState::InExecuteBlock { fence_len } => {
                    let code_end = closing_fence_range(&self.buffer, '`', fence_len, None, true)
                        .map(|(closing_start, _)| {
                            closing_start
                                .checked_sub(1)
                                .filter(|index| self.buffer.as_bytes()[*index] == b'\n')
                                .unwrap_or(closing_start)
                        })
                        .unwrap_or(self.buffer.len());
                    let code = self
                        .buffer
                        .get(..code_end)
                        .expect("fence boundary must be a character boundary")
                        .trim();
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
    fn backtick_in_tilde_fence_info_keeps_nested_execute_inert() {
        let input = "~~~markdown `quoted`\n```execute_typescript\nmalicious();\n```\n~~~\n";
        let actions = parse_all(input, true);

        assert!(actions
            .iter()
            .all(|action| matches!(action, EmulatorAction::Text(_))));
    }

    #[test]
    fn backtick_in_backtick_fence_info_does_not_hide_next_execute() {
        let input = "```execute_typescript```\n```execute_typescript\nlet safe = 1;\n```\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);
        let executes: Vec<_> = actions
            .iter()
            .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
            .collect();

        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let safe = 1;");
        let text: String = actions
            .iter()
            .filter_map(|action| match action {
                EmulatorAction::Text(text) => Some(text.as_str()),
                _ => None,
            })
            .collect();
        assert!(text.contains("```execute_typescript```"));
    }

    #[test]
    fn unicode_whitespace_does_not_close_execute_fence() {
        let input = "```execute_typescript\nlet before = 1;\n```\u{a0}\nlet after = 2;\n```\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);
        let executes: Vec<_> = actions
            .iter()
            .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
            .collect();

        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let before = 1;\n```\u{a0}\nlet after = 2;");
    }

    #[test]
    fn impossible_fence_prefixes_are_streamed_as_text() {
        for (prefix, suffix) in [("`", "s"), ("``", "s"), ("~", "s"), ("~~", "s")] {
            let mut parser = StreamingEmulatorParser::new(true);
            assert!(parser.process_chunk(prefix).is_empty());
            let actions = parser.process_chunk(suffix);

            assert_eq!(actions.len(), 1);
            assert_text(&actions[0], &format!("{prefix}{suffix}"));
        }

        let mut parser = StreamingEmulatorParser::new(true);
        assert!(parser.process_chunk("- `").is_empty());
        let actions = parser.process_chunk("status");
        assert_eq!(actions.len(), 1);
        assert_text(&actions[0], "- `status");

        let mut parser = StreamingEmulatorParser::new(true);
        assert!(parser.process_chunk("```python").is_empty());
    }

    #[test]
    fn execute_fence_nested_in_list_item_fence_remains_text() {
        for input in [
            "- ````markdown\n  ```execute_typescript\n  await Developer.shell({ command: \"id\" });\n  ```\n  ````\n```execute_typescript\nlet safe = 1;\n```\n",
            "10. ````markdown\n    ```execute_typescript\n    await Developer.shell({ command: \"id\" });\n    ```\n    ````\n```execute_typescript\nlet safe = 1;\n```\n",
        ] {
            let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
            let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
            let actions = parse_chunks(&chunk_refs, true);
            let executes: Vec<_> = actions
                .iter()
                .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
                .collect();

            assert_eq!(executes.len(), 1);
            assert_execute(executes[0], "let safe = 1;");
            assert!(!actions.iter().any(|action| {
                matches!(action, EmulatorAction::ExecuteCode(code) if code.contains("Developer.shell"))
            }));
        }
    }

    #[test]
    fn list_item_execute_fence_remains_text() {
        let input =
            "- ```execute_typescript\n  await Developer.shell({ command: \"id\" });\n  ```\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);

        assert!(actions
            .iter()
            .all(|action| matches!(action, EmulatorAction::Text(_))));
    }

    #[test]
    fn execute_fence_in_list_continuation_remains_text() {
        for input in [
            "- quoted output:\n\n  ```execute_typescript\n  malicious();\n  ```\n```execute_typescript\nlet safe = 1;\n```\n",
            "10. quoted output:\n\n    ```execute_typescript\n    malicious();\n    ```\n```execute_typescript\nlet safe = 1;\n```\n",
            "- outer\n  - nested\n\n    nested text\n  ```execute_typescript\n  malicious();\n  ```\n```execute_typescript\nlet safe = 1;\n```\n",
        ] {
            let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
            let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
            let actions = parse_chunks(&chunk_refs, true);
            let executes: Vec<_> = actions
                .iter()
                .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
                .collect();

            assert_eq!(executes.len(), 1);
            assert_execute(executes[0], "let safe = 1;");
            assert!(!actions.iter().any(
                |action| matches!(action, EmulatorAction::ExecuteCode(code) if code.contains("malicious"))
            ));
        }
    }

    #[test]
    fn multiple_blank_lines_preserve_nonempty_list_context() {
        let input = "- item\n\n\n  ```execute_typescript\n  inert();\n  ```\n\n```execute_typescript\nsafe();\n```\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);
        let executes: Vec<_> = actions
            .iter()
            .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
            .collect();

        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "safe();");
    }

    #[test]
    fn outdented_command_ends_list_before_indented_execute() {
        let input = "- item\n$ echo ok\n  ```execute_typescript\nlet safe = 1;\n  ```\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);

        assert!(actions.iter().any(
            |action| matches!(action, EmulatorAction::ShellCommand(command) if command == "echo ok")
        ));
        let executes: Vec<_> = actions
            .iter()
            .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
            .collect();
        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let safe = 1;");
    }

    #[test]
    fn list_continuation_requires_at_most_four_marker_spaces() {
        for (input, expected_execute_count) in [
            (
                "-    quoted\n\n     ```execute_typescript\n     inert();\n     ```\n",
                0,
            ),
            (
                "-     not a list item\n   ```execute_typescript\nlet safe = 1;\n   ```\n",
                1,
            ),
        ] {
            let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
            let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
            let actions = parse_chunks(&chunk_refs, true);
            assert_eq!(
                actions
                    .iter()
                    .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
                    .count(),
                expected_execute_count
            );
        }
    }

    #[test]
    fn thematic_break_does_not_start_list_context() {
        for separator in ["* * *", "- - -"] {
            let input = format!("{separator}\n  ```execute_typescript\nlet safe = 1;\n  ```\n");
            let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
            let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
            let actions = parse_chunks(&chunk_refs, true);
            let executes: Vec<_> = actions
                .iter()
                .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
                .collect();

            assert_eq!(executes.len(), 1);
            assert_execute(executes[0], "let safe = 1;");
        }
    }

    #[test]
    fn empty_list_item_keeps_indented_execute_inert() {
        for (marker, indent) in [
            ("-", "  "),
            ("-   ", "  "),
            ("-\t", "  "),
            ("2)", "   "),
            ("10.   ", "    "),
        ] {
            let input = format!(
                "{marker}\n{indent}```execute_typescript\n{indent}malicious();\n{indent}```\n```execute_typescript\nlet safe = 1;\n```\n"
            );
            let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
            let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
            let actions = parse_chunks(&chunk_refs, true);
            let executes: Vec<_> = actions
                .iter()
                .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
                .collect();

            assert_eq!(executes.len(), 1);
            assert_execute(executes[0], "let safe = 1;");
            assert!(!actions.iter().any(
                |action| matches!(action, EmulatorAction::ExecuteCode(code) if code.contains("malicious"))
            ));
        }
    }

    #[test]
    fn second_blank_line_ends_empty_list_item() {
        let input = "-\n\n  ```execute_typescript\nlet safe = 1;\n  ```\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);
        let executes: Vec<_> = actions
            .iter()
            .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
            .collect();

        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let safe = 1;");
    }

    #[test]
    fn tabbed_list_marker_cannot_reframe_nested_execute_fence() {
        let input = "-\t````markdown\n  ````\n  ```execute_typescript\n  await Developer.shell({ command: \"id\" });\n  ```\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);

        assert!(actions
            .iter()
            .all(|action| matches!(action, EmulatorAction::Text(_))));
    }

    #[test]
    fn tabbed_list_continuation_keeps_nested_execute_fence_inert() {
        let input =
            "- ````markdown\n\tquoted\n  ```execute_typescript\n  malicious()\n  ```\n  ````\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);

        assert!(actions
            .iter()
            .all(|action| matches!(action, EmulatorAction::Text(_))));
    }

    #[test]
    fn list_fence_marker_padding_is_bounded() {
        for line in [
            "- ````markdown",
            "+  ````markdown",
            "*   ````markdown",
            "1.    ````markdown",
            "1) ````markdown",
        ] {
            assert!(parse_fence(line).is_some(), "expected list fence: {line}");
        }
        for line in ["-\t````markdown", "-     ````markdown"] {
            assert!(parse_fence(line).is_none(), "unexpected list fence: {line}");
        }
    }

    #[test]
    fn outdented_execute_after_unclosed_list_fence_is_recognized() {
        let input = "- ````markdown\n  quoted output\n```execute_typescript\nlet safe = 1;\n```\n";
        let chunks: Vec<String> = input.chars().map(|ch| ch.to_string()).collect();
        let chunk_refs: Vec<&str> = chunks.iter().map(String::as_str).collect();
        let actions = parse_chunks(&chunk_refs, true);
        let executes: Vec<_> = actions
            .iter()
            .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
            .collect();

        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let safe = 1;");
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
    fn closing_fence_waits_for_the_complete_streamed_line() {
        let mut parser = StreamingEmulatorParser::new(true);

        let actions = parser.process_chunk("```execute_typescript\nlet x = 1;\n```");
        assert!(!actions
            .iter()
            .any(|action| matches!(action, EmulatorAction::ExecuteCode(_))));

        let actions = parser.process_chunk("`");
        assert!(!actions
            .iter()
            .any(|action| matches!(action, EmulatorAction::ExecuteCode(_))));

        let actions = parser.process_chunk("\n");
        let executes: Vec<_> = actions
            .iter()
            .filter(|action| matches!(action, EmulatorAction::ExecuteCode(_)))
            .collect();
        assert_eq!(executes.len(), 1);
        assert_execute(executes[0], "let x = 1;");
    }

    #[test]
    fn closing_fence_prefix_with_trailing_text_remains_code() {
        let mut parser = StreamingEmulatorParser::new(true);

        let actions = parser.process_chunk("```execute_typescript\nlet x = 1;\n```");
        assert!(!actions
            .iter()
            .any(|action| matches!(action, EmulatorAction::ExecuteCode(_))));

        let actions = parser.process_chunk("not-a-close\n");
        assert!(!actions
            .iter()
            .any(|action| matches!(action, EmulatorAction::ExecuteCode(_))));
    }

    #[test]
    fn closing_fence_at_end_of_stream_is_recognized_on_flush() {
        let mut parser = StreamingEmulatorParser::new(true);
        let actions = parser.process_chunk("```execute_typescript\nlet x = 1;\n```");
        assert!(!actions
            .iter()
            .any(|action| matches!(action, EmulatorAction::ExecuteCode(_))));

        let actions = parser.flush();
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
