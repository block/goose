//! Tool call emulation for models without native tool-calling support.
//!
//! The model is prompted to emit shell commands as `$ command` on a new line and
//! code blocks as `` ```execute `` fenced blocks. A streaming parser detects these
//! patterns and converts them into tool-call messages.
//!
//! # Known false-positive scenarios
//!
//! Because detection is purely text-based, the parser can misinterpret model output:
//!
//! - **`$` at line start in explanatory text.** If the model writes a line starting
//!   with `$` as an example (e.g. "$ is the jQuery selector"), it will be treated as
//!   a shell command. Mid-sentence `$` (e.g. "costs $50") is safe — only `\n$` or
//!   `$` at the very start of output triggers command detection.
//!
//! - **`` ```execute `` in explanatory code fences.** If the model uses this exact
//!   fence tag in prose, the content will be executed. Standard `` ```js `` or
//!   `` ```python `` fences are not affected.
//!
//! These are inherent to text-based tool emulation. Models with native tool-calling
//! support should use the `inference_native_tools` path instead.

use goose_provider_types::conversation::message::{Message, MessageContent};
use goose_provider_types::errors::ProviderError;
use rmcp::model::{CallToolRequestParams, Tool};
use serde_json::json;
use std::borrow::Cow;
use uuid::Uuid;

use super::super::{finalize_usage, thinking_output::ThinkingOutputFilter, StreamSender};
use super::inference_engine::{
    generation_loop, prepare_generation, GenerationContext, StopSuffixTrimmer, TokenAction,
};
use crate::tool_emulation::{EmulatorAction, StreamingEmulatorParser};

const SHELL_TOOL: &str = "developer__shell";
const CODE_EXECUTION_TOOL: &str = "code_execution__execute_typescript";

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
        tracing::warn!("Failed to load tiny_model_system.md: {:?}", e);
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
        tool_desc.push_str("```execute_typescript\nasync function run() {\n");
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
            let tool_call =
                CallToolRequestParams::new(Cow::Borrowed(SHELL_TOOL)).with_arguments(args);
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
            let tool_call =
                CallToolRequestParams::new(Cow::Borrowed(CODE_EXECUTION_TOOL)).with_arguments(args);
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

pub(super) fn generate_with_emulated_tools(
    ctx: &mut GenerationContext<'_>,
    code_mode_enabled: bool,
    oai_messages_json: &str,
) -> Result<(), ProviderError> {
    let prepared = prepare_generation(ctx, oai_messages_json, None, None)?;
    let template_result = prepared.template_result;
    let mut llama_ctx = prepared.llama_ctx;
    let prompt_token_count = prepared.prompt_token_count;
    let effective_ctx = prepared.effective_ctx;

    let message_id = ctx.message_id;
    let tx = ctx.tx;
    let mut emulator_parser = StreamingEmulatorParser::new(code_mode_enabled);
    let mut output_filter = ThinkingOutputFilter::new(
        ctx.settings.enable_thinking,
        &template_result.generation_prompt,
    );
    let mut stop_trimmer = StopSuffixTrimmer::new(&template_result.additional_stops);
    let mut generated_text = String::new();
    let mut tool_call_emitted = false;
    let mut send_failed = false;
    let mut stop_string_emitted = false;

    let output_token_count = generation_loop(
        &ctx.loaded.model,
        &mut llama_ctx,
        ctx.settings,
        prompt_token_count,
        effective_ctx,
        |piece| {
            generated_text.push_str(piece);
            let filtered = output_filter.push_text(piece);
            let (content, stop_seen) = stop_trimmer.push(&filtered.content);
            let actions = emulator_parser.process_chunk(&content);
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
            } else if stop_seen
                || template_result
                    .additional_stops
                    .iter()
                    .any(|stop| generated_text.ends_with(stop))
            {
                stop_string_emitted = true;
                Ok(TokenAction::Stop)
            } else {
                Ok(TokenAction::Continue)
            }
        },
    )?;

    if !send_failed {
        let filtered = output_filter.finish();
        if !filtered.thinking.is_empty() {
            let mut message = Message::assistant().with_thinking(filtered.thinking, "");
            message.id = Some(message_id.to_string());
            send_failed = tx.blocking_send(Ok((Some(message), None))).is_err();
        }
        if !send_failed {
            let content = if stop_string_emitted {
                String::new()
            } else {
                let (content, stop_seen) = stop_trimmer.push(&filtered.content);
                let mut content = content;
                if !stop_seen {
                    content.push_str(&stop_trimmer.finish());
                }
                content
            };
            for action in emulator_parser.process_chunk(&content) {
                if send_emulator_action(&action, message_id, tx).is_err() {
                    send_failed = true;
                    break;
                }
            }
        }
    }

    if !send_failed {
        for action in emulator_parser.flush() {
            if send_emulator_action(&action, message_id, tx).is_err() {
                break;
            }
        }
    }

    let provider_usage = finalize_usage(
        ctx.log,
        std::mem::take(&mut ctx.model_name),
        "emulator",
        prompt_token_count,
        output_token_count,
        None,
    );
    let _ = ctx.tx.blocking_send(Ok((None, Some(provider_usage))));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Collect all actions from feeding chunks through the parser, then flushing.
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

    fn trim_chunks(chunks: &[&str], stops: &[String]) -> (String, bool) {
        let mut trimmer = StopSuffixTrimmer::new(stops);
        let mut output = String::new();
        let mut stopped = false;

        for chunk in chunks {
            let (content, stop_seen) = trimmer.push(chunk);
            output.push_str(&content);
            if stop_seen {
                stopped = true;
                break;
            }
        }

        if !stopped {
            output.push_str(&trimmer.finish());
        }

        (output, stopped)
    }

    fn parse_with_seeded_thinking(
        chunks: &[&str],
        code_mode: bool,
    ) -> (String, Vec<EmulatorAction>) {
        let mut output_filter = ThinkingOutputFilter::new(true, "<|assistant|><think>\n");
        let mut parser = StreamingEmulatorParser::new(code_mode);
        let mut thinking = String::new();
        let mut actions = Vec::new();

        for chunk in chunks {
            let filtered = output_filter.push_text(chunk);
            thinking.push_str(&filtered.thinking);
            actions.extend(parser.process_chunk(&filtered.content));
        }

        let filtered = output_filter.finish();
        thinking.push_str(&filtered.thinking);
        actions.extend(parser.process_chunk(&filtered.content));
        actions.extend(parser.flush());

        (thinking, actions)
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
    fn stop_suffix_trimmer_strips_split_stop() {
        let stops = vec!["<|eom_id|>".to_string()];
        let (content, stopped) = trim_chunks(&["The answer", "<|e", "om_id|>"], &stops);

        assert!(stopped);
        assert_eq!(content, "The answer");
    }

    #[test]
    fn stop_suffix_trimmer_flushes_partial_non_stop() {
        let stops = vec!["<|eom_id|>".to_string()];
        let (content, stopped) = trim_chunks(&["Use the <", " symbol"], &stops);

        assert!(!stopped);
        assert_eq!(content, "Use the < symbol");
    }

    #[test]
    fn plain_text_no_tools() {
        let actions = parse_all("Hello, world!", false);
        // Hold-back may split text across actions; concatenate all text
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
        // Flush should handle unterminated command
        let actions = parse_all("$ whoami", false);
        assert_eq!(actions.len(), 1);
        assert_shell(&actions[0], "whoami");
    }

    #[test]
    fn dollar_sign_mid_sentence_is_not_command() {
        let actions = parse_all("It costs $50 per month", false);
        for action in &actions {
            assert!(
                matches!(action, EmulatorAction::Text(_)),
                "mid-sentence $ should not trigger a shell command"
            );
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
    fn execute_block_not_detected_without_code_mode() {
        let input = "```execute_typescript\nconsole.log('hi');\n```\n";
        let actions = parse_all(input, false);
        // Should be treated as plain text
        for action in &actions {
            assert!(matches!(action, EmulatorAction::Text(_)));
        }
    }

    #[test]
    fn dollar_split_across_chunks() {
        // The \n and $ arrive in separate chunks
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
        // In practice, generation stops after the first tool call. But the
        // parser should detect commands separated by \n$ when fed as chunks.
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
            assert!(
                matches!(action, EmulatorAction::Text(_)),
                "regular code fence should be text"
            );
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
    fn impossible_fence_prefix_is_streamed_as_text() {
        let mut parser = StreamingEmulatorParser::new(true);
        assert!(parser.process_chunk("``").is_empty());
        let actions = parser.process_chunk("status");

        assert_eq!(actions.len(), 1);
        assert_text(&actions[0], "``status");
    }

    #[test]
    fn execute_fence_nested_in_list_item_fence_remains_text() {
        let input = "- ````markdown\n  ```execute_typescript\n  await Developer.shell({ command: \"id\" });\n  ```\n  ````\n```execute_typescript\nlet safe = 1;\n```\n";
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
        let input = "- quoted output:\n\n  ```execute_typescript\n  malicious();\n  ```\n```execute_typescript\nlet safe = 1;\n```\n";
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

    #[test]
    fn thematic_break_does_not_start_list_context() {
        let input = "* * *\n  ```execute_typescript\nlet safe = 1;\n  ```\n";
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
    fn empty_list_item_keeps_indented_execute_inert() {
        for (marker, indent) in [("-", "  "), ("10.   ", "    ")] {
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
        }
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
        // Empty command after $ should not produce a ShellCommand
        let shells: Vec<_> = actions
            .iter()
            .filter(|a| matches!(a, EmulatorAction::ShellCommand(_)))
            .collect();
        assert_eq!(shells.len(), 0);
    }

    #[test]
    fn token_by_token_streaming() {
        // Simulate LLM generating one token at a time
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
    fn thinking_seeded_from_generation_prompt_is_not_emulated_text() {
        let (thinking, actions) =
            parse_with_seeded_thinking(&["reasoning\n$ echo hidden\n</think>The answer."], false);

        assert_eq!(thinking.trim(), "reasoning\n$ echo hidden");
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
        assert_eq!(text.trim(), "The answer.");
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
        // Model stops generating mid-block
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
