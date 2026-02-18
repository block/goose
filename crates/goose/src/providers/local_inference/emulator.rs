use crate::conversation::message::{Message, MessageContent};
use crate::providers::errors::ProviderError;
use crate::providers::utils::RequestLog;
use llama_cpp_2::model::{AddBos, LlamaChatMessage};
use rmcp::model::{CallToolRequestParams, Tool};
use serde_json::json;
use std::borrow::Cow;
use uuid::Uuid;

use super::inference_context::{
    create_and_prefill_context, generation_loop, validate_and_compute_context, LoadedModel,
    TokenAction,
};
use super::{finalize_usage, InferenceRuntime, StreamSender, CODE_EXECUTION_TOOL, SHELL_TOOL};

pub(super) fn load_tiny_model_prompt() -> String {
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
        eprintln!("WARNING: Failed to load tiny_model_system.md: {:?}", e);
        "You are Goose, an AI assistant. You can execute shell commands by starting lines with $."
            .to_string()
    })
}

pub(super) fn build_emulator_tool_description(tools: &[Tool], code_mode_enabled: bool) -> String {
    let mut tool_desc = String::new();

    if code_mode_enabled {
        tool_desc.push_str("\n\n# Running Code\n\n");
        tool_desc.push_str(
            "You can call tools by writing code in a ```execute block. \
             The code runs immediately — do not explain it, just run it.\n\n",
        );
        tool_desc.push_str("Example — counting files in /tmp:\n\n");
        tool_desc.push_str("```execute\nasync function run() {\n");
        tool_desc.push_str(
            "  const result = await Developer.shell({ command: \"ls -1 /tmp | wc -l\" });\n",
        );
        tool_desc.push_str("  return result;\n}\n```\n\n");
        tool_desc.push_str("Rules:\n");
        tool_desc.push_str("- Code MUST define async function run() and return a result\n");
        tool_desc.push_str("- All function calls are async — use await\n");
        tool_desc.push_str("- Use ```execute for tool calls, $ for simple shell one-liners\n\n");
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

enum EmulatorAction {
    Text(String),
    ShellCommand(String),
    ExecuteCode(String),
}

enum ParserState {
    Normal,
    InCommand,
    InExecuteBlock,
}

struct StreamingEmulatorParser {
    buffer: String,
    state: ParserState,
    code_mode_enabled: bool,
}

impl StreamingEmulatorParser {
    fn new(code_mode_enabled: bool) -> Self {
        Self {
            buffer: String::new(),
            state: ParserState::Normal,
            code_mode_enabled,
        }
    }

    fn process_chunk(&mut self, chunk: &str) -> Vec<EmulatorAction> {
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
                ParserState::InExecuteBlock => {
                    // Look for closing ``` to end the execute block
                    if let Some(end_idx) = self.buffer.find("\n```") {
                        #[allow(clippy::string_slice)]
                        let code = self.buffer[..end_idx].to_string();
                        // Skip past the closing ``` and any trailing newline
                        #[allow(clippy::string_slice)]
                        let rest = &self.buffer[end_idx + 4..];
                        let rest = rest.strip_prefix('\n').unwrap_or(rest);
                        self.buffer = rest.to_string();
                        self.state = ParserState::Normal;
                        if !code.trim().is_empty() {
                            results.push(EmulatorAction::ExecuteCode(code));
                        }
                    } else {
                        // Still accumulating code — wait for closing fence
                        break;
                    }
                }
                ParserState::Normal => {
                    // Check for ```execute block (code mode)
                    if self.code_mode_enabled {
                        if let Some((before, after)) = self.buffer.split_once("```execute\n") {
                            if !before.trim().is_empty() {
                                results.push(EmulatorAction::Text(before.to_string()));
                            }
                            self.buffer = after.to_string();
                            self.state = ParserState::InExecuteBlock;
                            continue;
                        }
                        // Also handle without newline after tag (accumulating)
                        if self.buffer.ends_with("```execute") {
                            let before = self.buffer.trim_end_matches("```execute");
                            if !before.trim().is_empty() {
                                results.push(EmulatorAction::Text(before.to_string()));
                            }
                            self.buffer.clear();
                            self.state = ParserState::InExecuteBlock;
                            continue;
                        }
                    }

                    // Check for $ command
                    if let Some((before_dollar, from_dollar)) = self.buffer.split_once("\n$") {
                        let text = format!("{}\n", before_dollar);
                        if !text.trim().is_empty() {
                            results.push(EmulatorAction::Text(text));
                        }
                        self.buffer = format!("${}", from_dollar);
                        self.state = ParserState::InCommand;
                    } else if self.buffer.starts_with('$') && self.buffer.len() == chunk.len() {
                        self.state = ParserState::InCommand;
                    } else {
                        // Hold back a small tail in case it's the start of
                        // a ``` fence or a \n$ command prefix.
                        let hold_back = if self.code_mode_enabled { 12 } else { 2 };
                        let char_count = self.buffer.chars().count();
                        if char_count > hold_back && !self.buffer.ends_with('\n') {
                            let mut chars = self.buffer.chars();
                            let emit_count = char_count - hold_back;
                            let emit_text: String = chars.by_ref().take(emit_count).collect();
                            let keep_text: String = chars.collect();
                            if !emit_text.is_empty() {
                                results.push(EmulatorAction::Text(emit_text));
                            }
                            self.buffer = keep_text;
                        }
                        break;
                    }
                }
            }
        }

        results
    }

    fn flush(&mut self) -> Vec<EmulatorAction> {
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
                ParserState::InExecuteBlock => {
                    let code = self.buffer.trim();
                    if !code.is_empty() {
                        results.push(EmulatorAction::ExecuteCode(code.to_string()));
                    }
                }
                ParserState::Normal => {
                    results.push(EmulatorAction::Text(self.buffer.clone()));
                }
            }
            self.buffer.clear();
            self.state = ParserState::Normal;
        }

        results
    }
}

fn send_emulator_action(
    action: &EmulatorAction,
    message_id: &str,
    tx: &StreamSender,
) -> Result<bool, ()> {
    match action {
        EmulatorAction::Text(text) => {
            let mut message = Message::assistant().with_text(text);
            message.id = Some(message_id.to_string());
            tx.blocking_send(Ok((Some(message), None)))
                .map_err(|_| ())?;
            Ok(false)
        }
        EmulatorAction::ShellCommand(command) => {
            let tool_id = Uuid::new_v4().to_string();
            let mut args = serde_json::Map::new();
            args.insert("command".to_string(), json!(command));
            let tool_call = CallToolRequestParams {
                meta: None,
                task: None,
                name: Cow::Owned(SHELL_TOOL.to_string()),
                arguments: Some(args),
            };
            let mut message = Message::assistant();
            message
                .content
                .push(MessageContent::tool_request(tool_id, Ok(tool_call)));
            message.id = Some(message_id.to_string());
            tx.blocking_send(Ok((Some(message), None)))
                .map_err(|_| ())?;
            Ok(true)
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
            let tool_call = CallToolRequestParams {
                meta: None,
                task: None,
                name: Cow::Owned(CODE_EXECUTION_TOOL.to_string()),
                arguments: Some(args),
            };
            let mut message = Message::assistant();
            message
                .content
                .push(MessageContent::tool_request(tool_id, Ok(tool_call)));
            message.id = Some(message_id.to_string());
            tx.blocking_send(Ok((Some(message), None)))
                .map_err(|_| ())?;
            Ok(true)
        }
    }
}

#[allow(clippy::too_many_arguments)]
pub(super) fn run_emulator_path(
    loaded: &LoadedModel,
    runtime: &InferenceRuntime,
    chat_messages: &[LlamaChatMessage],
    settings: &crate::providers::local_inference::local_model_registry::ModelSettings,
    context_limit: usize,
    code_mode_enabled: bool,
    model_name: String,
    message_id: &str,
    tx: &StreamSender,
    log: &mut RequestLog,
) -> Result<(), ProviderError> {
    let prompt = loaded
        .model
        .apply_chat_template(&loaded.template, chat_messages, true)
        .map_err(|e| {
            ProviderError::ExecutionError(format!("Failed to apply chat template: {}", e))
        })?;

    let tokens = loaded
        .model
        .str_to_token(&prompt, AddBos::Never)
        .map_err(|e| ProviderError::ExecutionError(format!("Failed to tokenize prompt: {}", e)))?;

    let (prompt_token_count, effective_ctx) =
        validate_and_compute_context(loaded, runtime, tokens.len(), context_limit, settings)?;
    let mut ctx = create_and_prefill_context(loaded, runtime, &tokens, effective_ctx, settings)?;

    let mut emulator_parser = StreamingEmulatorParser::new(code_mode_enabled);
    let mut tool_call_emitted = false;
    let mut send_failed = false;

    let output_token_count = generation_loop(
        &loaded.model,
        &mut ctx,
        settings,
        prompt_token_count,
        effective_ctx,
        |piece| {
            let actions = emulator_parser.process_chunk(piece);
            for action in actions {
                match send_emulator_action(&action, message_id, tx) {
                    Ok(is_tool) => {
                        if is_tool {
                            tool_call_emitted = true;
                        }
                    }
                    Err(_) => {
                        send_failed = true;
                        return Ok(TokenAction::Stop);
                    }
                }
            }
            if tool_call_emitted {
                Ok(TokenAction::Stop)
            } else {
                Ok(TokenAction::Continue)
            }
        },
    )?;

    if !send_failed {
        for action in emulator_parser.flush() {
            if send_emulator_action(&action, message_id, tx).is_err() {
                break;
            }
        }
    }

    let provider_usage = finalize_usage(
        log,
        model_name,
        "emulator",
        prompt_token_count,
        output_token_count,
        None,
    );
    let _ = tx.blocking_send(Ok((None, Some(provider_usage))));
    Ok(())
}
