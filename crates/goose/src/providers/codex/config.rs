use super::*;

pub const CODEX_PROVIDER_NAME: &str = "codex";
pub const CODEX_DEFAULT_MODEL: &str = "gpt-5.2-codex";
pub const CODEX_KNOWN_MODELS: &[&str] = &[
    "gpt-5.2-codex",
    "gpt-5.2",
    "gpt-5.1-codex-max",
    "gpt-5.1-codex-mini",
];
pub const CODEX_DOC_URL: &str = "https://developers.openai.com/codex/cli";

/// Valid reasoning effort levels for Codex
pub const CODEX_REASONING_LEVELS: &[&str] = &["none", "low", "medium", "high", "xhigh"];

/// Builds the text prompt and extracts images to temp files in a single pass.
/// Text goes to the prompt string (piped via stdin); images become temp files
/// (passed via `-i` flags). Returns (prompt, temp_files).
pub fn prepare_input(
    system: &str,
    messages: &[Message],
    image_dir: &Path,
) -> Result<(String, Vec<NamedTempFile>), ProviderError> {
    let mut prompt = String::new();
    let mut temp_files = Vec::new();

    let filtered_system = filter_extensions_from_system_prompt(system);
    if !filtered_system.is_empty() {
        prompt.push_str(&filtered_system);
        prompt.push_str("\n\n");
    }

    for message in messages.iter().filter(|m| m.is_agent_visible()) {
        let role_prefix = match message.role {
            Role::User => "Human: ",
            Role::Assistant => "Assistant: ",
        };
        prompt.push_str(role_prefix);

        for content in &message.content {
            match content {
                MessageContent::Text(t) => {
                    prompt.push_str(&t.text);
                    prompt.push('\n');
                }
                MessageContent::Image(img) => {
                    let decoded = BASE64.decode(&img.data).map_err(|e| {
                        ProviderError::RequestFailed(format!("Failed to decode image: {}", e))
                    })?;
                    // Codex only supports png and jpeg:
                    // https://github.com/openai/codex/blob/aea7610c/codex-rs/utils/image/src/lib.rs#L162-L167
                    let ext = match img.mime_type.as_str() {
                        "image/png" => "png",
                        "image/jpeg" => "jpg",
                        _ => {
                            return Err(ProviderError::RequestFailed(format!(
                                "Unsupported image MIME type for Codex: {}",
                                img.mime_type
                            )));
                        }
                    };
                    let mut tmp = tempfile::Builder::new()
                        .suffix(&format!(".{}", ext))
                        .tempfile_in(image_dir)
                        .map_err(|e| {
                            ProviderError::RequestFailed(format!(
                                "Failed to create temp file: {}",
                                e
                            ))
                        })?;
                    tmp.write_all(&decoded).map_err(|e| {
                        ProviderError::RequestFailed(format!("Failed to write image: {}", e))
                    })?;
                    temp_files.push(tmp);
                }
                MessageContent::ToolRequest(req) => {
                    if let Ok(call) = &req.tool_call {
                        prompt.push_str(&format!("[tool_use: {} id={}]\n", call.name, req.id));
                    }
                }
                MessageContent::ToolResponse(resp) => {
                    if let Ok(result) = &resp.tool_result {
                        let text: String = result
                            .content
                            .iter()
                            .filter_map(|c| match &c.raw {
                                rmcp::model::RawContent::Text(t) => Some(t.text.as_str()),
                                _ => None,
                            })
                            .collect::<Vec<&str>>()
                            .join("\n");
                        prompt.push_str(&format!("[tool_result id={}] {}\n", resp.id, text));
                    }
                }
                _ => {}
            }
        }
        prompt.push('\n');
    }

    prompt.push_str("Assistant: ");
    Ok((prompt, temp_files))
}

pub fn toml_quote(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for ch in s.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            '\u{0008}' => out.push_str("\\b"),
            '\u{000C}' => out.push_str("\\f"),
            c if c.is_control() => {
                // TOML \uXXXX for other control characters
                for unit in c.encode_utf16(&mut [0; 2]) {
                    out.push_str(&format!("\\u{:04X}", unit));
                }
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

// Codex CLI only supports inline `-c key=value` TOML overrides — no file-based
// config merging. Resolved secrets (from env_keys/keystore) in envs/headers end
// up in process argv, visible via `ps`. Claude Code avoids this by writing to a
// temp file with 0o600 permissions.
// Tracking: https://github.com/openai/codex/issues/2628
pub fn codex_mcp_config_overrides(extensions: &[ExtensionConfig]) -> Vec<String> {
    let mut overrides = Vec::new();
    for extension in extensions {
        match extension {
            ExtensionConfig::StreamableHttp { uri, headers, .. } => {
                let key = extension.key();
                overrides.push(format!("mcp_servers.{}.url={}", key, toml_quote(uri)));
                if !headers.is_empty() {
                    let mut hkeys: Vec<_> = headers.keys().collect();
                    hkeys.sort();
                    let entries: Vec<_> = hkeys
                        .iter()
                        .map(|k| format!("{} = {}", toml_quote(k), toml_quote(&headers[*k])))
                        .collect();
                    overrides.push(format!(
                        "mcp_servers.{}.http_headers={{{}}}",
                        key,
                        entries.join(", ")
                    ));
                }
            }
            ExtensionConfig::Stdio {
                cmd, args, envs, ..
            } => {
                let key = extension.key();
                overrides.push(format!("mcp_servers.{}.command={}", key, toml_quote(cmd)));
                if !args.is_empty() {
                    let items: Vec<_> = args.iter().map(|a| toml_quote(a)).collect();
                    overrides.push(format!("mcp_servers.{}.args=[{}]", key, items.join(", ")));
                }
                let env_map = envs.get_env();
                if !env_map.is_empty() {
                    let mut ekeys: Vec<_> = env_map.keys().collect();
                    ekeys.sort();
                    let entries: Vec<_> = ekeys
                        .iter()
                        .map(|k| {
                            format!("{} = {}", toml_quote(k), toml_quote(&env_map[k.as_str()]))
                        })
                        .collect();
                    overrides.push(format!(
                        "mcp_servers.{}.env={{{}}}",
                        key,
                        entries.join(", ")
                    ));
                }
            }
            ExtensionConfig::Sse { name, .. } => {
                tracing::debug!(name, "skipping SSE extension, migrate to streamable_http");
            }
            _ => {}
        }
    }
    overrides
}
