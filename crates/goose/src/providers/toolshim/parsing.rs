use super::*;

#[allow(clippy::string_slice)] // All markers/delimiters are ASCII; byte indexing is safe.
pub fn extract_shell_command_from_execute_code(code: &str) -> Option<String> {
    let marker = "command";
    let marker_idx = code.find(marker)?;
    let after_marker = &code[marker_idx + marker.len()..];
    let colon_idx = after_marker.find(':')?;
    let after_colon = after_marker[colon_idx + 1..].trim_start();

    let quote = after_colon.chars().next()?;
    if quote != '"' && quote != '\'' {
        return None;
    }

    let mut escaped = false;
    let mut command = String::new();
    for ch in after_colon[1..].chars() {
        if escaped {
            command.push(ch);
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
            continue;
        }

        if ch == quote {
            return Some(command);
        }

        command.push(ch);
    }

    None
}

pub fn maybe_convert_execute_to_shell_tool_call(
    raw_tool_name: &str,
    arguments_value: &Value,
    tools: &[Tool],
) -> Option<CallToolRequestParams> {
    let alias = normalized_tool_alias(raw_tool_name);
    if alias != "execute" && alias != "execute_code" {
        return None;
    }

    let shell_tool_name = resolve_tool_name("shell", tools)?;
    let code = arguments_value.get("code")?.as_str()?;
    let command = extract_shell_command_from_execute_code(code)?;

    let shell_args = json!({ "command": command });
    Some(CallToolRequestParams::new(shell_tool_name).with_arguments(object(shell_args)))
}

pub fn escape_invalid_backslashes_in_json_strings(input: &str) -> String {
    let mut out = String::with_capacity(input.len() + 8);
    let mut in_string = false;
    let mut escaped = false;

    for ch in input.chars() {
        if in_string {
            if escaped {
                if !matches!(ch, '"' | '\\' | '/' | 'b' | 'f' | 'n' | 'r' | 't' | 'u') {
                    out.push('\\');
                }
                out.push(ch);
                escaped = false;
                continue;
            }

            match ch {
                '\\' => {
                    out.push('\\');
                    escaped = true;
                }
                '"' => {
                    out.push('"');
                    in_string = false;
                }
                _ => out.push(ch),
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
        }
        out.push(ch);
    }

    if escaped {
        out.push('\\');
    }

    out
}

pub fn parse_json_value_tolerant(input: &str) -> Option<Value> {
    serde_json::from_str::<Value>(input).ok().or_else(|| {
        let escaped = escape_invalid_backslashes_in_json_strings(input);
        serde_json::from_str::<Value>(&escaped).ok()
    })
}

#[allow(clippy::string_slice)] // All markers are ASCII; byte indexing is safe.
pub fn parse_tokenized_tool_calls(content: &str, tools: &[Tool]) -> Vec<CallToolRequestParams> {
    let mut calls = Vec::new();
    let mut remainder = content;

    while let Some(begin_idx) = remainder.find(TOOL_CALL_BEGIN) {
        let after_begin = &remainder[begin_idx + TOOL_CALL_BEGIN.len()..];

        // Find the end of this tool call first
        let Some(call_end_offset) = after_begin.find(TOOL_CALL_END) else {
            break;
        };
        let call_body = &after_begin[..call_end_offset];

        // Try standard format: name <|tool_call_argument_begin|> {json}
        // Fall back to: name {json} (no argument marker)
        let (raw_tool_name, raw_args) =
            if let Some(arg_idx) = call_body.find(TOOL_CALL_ARGUMENT_BEGIN) {
                let name = call_body[..arg_idx].trim();
                let args = call_body[arg_idx + TOOL_CALL_ARGUMENT_BEGIN.len()..].trim();
                (name, args)
            } else if let Some(json_start) = call_body.find('{') {
                let name = call_body[..json_start].trim();
                let args = call_body[json_start..].trim();
                (name, args)
            } else {
                remainder = &after_begin[call_end_offset + TOOL_CALL_END.len()..];
                continue;
            };

        if let Some(arguments_value) = parse_json_value_tolerant(raw_args) {
            if let Some(tool_name) = resolve_tool_name(raw_tool_name, tools) {
                if arguments_value.is_object() {
                    calls.push(
                        CallToolRequestParams::new(tool_name)
                            .with_arguments(object(arguments_value.clone())),
                    );
                }
            } else if let Some(shell_call) =
                maybe_convert_execute_to_shell_tool_call(raw_tool_name, &arguments_value, tools)
            {
                calls.push(shell_call);
            }
        }

        remainder = &after_begin[call_end_offset + TOOL_CALL_END.len()..];
    }

    calls
}

#[allow(clippy::string_slice)] // Indices come from char_indices(); slicing is safe.
pub fn extract_first_json_object(input: &str) -> Option<(&str, usize)> {
    if !input.starts_with('{') {
        return None;
    }

    let mut depth = 0usize;
    let mut in_string = false;
    let mut escaped = false;

    for (idx, ch) in input.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
                continue;
            }
            match ch {
                '\\' => escaped = true,
                '"' => in_string = false,
                _ => {}
            }
            continue;
        }

        match ch {
            '"' => in_string = true,
            '{' => depth += 1,
            '}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    let end = idx + ch.len_utf8();
                    return Some((&input[..end], end));
                }
            }
            _ => {}
        }
    }

    None
}

#[allow(clippy::string_slice)] // Indices from find('{') on ASCII; byte slicing is safe.
pub fn parse_inline_json_tool_calls(content: &str, tools: &[Tool]) -> Vec<CallToolRequestParams> {
    let mut calls = Vec::new();
    let mut remainder = content;

    while let Some(start_idx) = remainder.find('{') {
        let maybe_json = &remainder[start_idx..];
        let Some((json_obj, consumed_len)) = extract_first_json_object(maybe_json) else {
            break;
        };

        if let Some(value) = parse_json_value_tolerant(json_obj) {
            let maybe_name = value.get("name").and_then(Value::as_str);
            let maybe_args = value.get("arguments").and_then(Value::as_object);
            if let (Some(raw_name), Some(arguments)) = (maybe_name, maybe_args) {
                if let Some(tool_name) = resolve_tool_name(raw_name, tools) {
                    calls.push(
                        CallToolRequestParams::new(tool_name).with_arguments(arguments.clone()),
                    );
                }
            }
        }

        remainder = &maybe_json[consumed_len..];
    }

    calls
}
