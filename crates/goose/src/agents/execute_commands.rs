use std::collections::{HashMap, HashSet};

use anyhow::{anyhow, Result};

use crate::context_mgmt::compact_messages;
use crate::conversation::message::{Message, MessageContent};
use crate::slash_commands::util::normalize_command_name;
use crate::slash_commands::{recipe_slash_command, skill_slash_command};

use super::Agent;

pub const COMPACT_TRIGGERS: &[&str] =
    &["/compact", "Please compact this conversation", "/summarize"];

pub struct CommandDef {
    pub name: &'static str,
    pub description: &'static str,
}

static COMMANDS: &[CommandDef] = &[
    CommandDef {
        name: "prompts",
        description: "List available prompts, optionally filtered by extension",
    },
    CommandDef {
        name: "prompt",
        description: "Execute a prompt or show its info with --info",
    },
    CommandDef {
        name: "compact",
        description: "Compact the conversation history",
    },
    CommandDef {
        name: "clear",
        description: "Clear the conversation history",
    },
    CommandDef {
        name: "skills",
        description: "List installed skills and other available sources",
    },
    CommandDef {
        name: "doctor",
        description: "Check that your Goose setup is working",
    },
    CommandDef {
        name: "goal",
        description: "Set a goal the agent must satisfy before finishing, or clear with /goal off",
    },
    CommandDef {
        name: "grind",
        description:
            "Set a goal the agent pursues relentlessly until max_turns, or clear with /grind off",
    },
    CommandDef {
        name: "status",
        description: "Show session status: model, provider, mode, and token usage",
    },
];

pub struct ParsedSlashCommand<'a> {
    pub command: &'a str,
    pub params_str: &'a str,
}

/// A skill slash token found anywhere in the message at a whitespace boundary.
///
/// Unlike [`parse_slash_command`], this can match mid-message when `/name` sits
/// at a whitespace token boundary and `name` is a known skill. Builtins such as
/// `/compact` are intentionally excluded from mid-message matching.
pub struct InlineSkillCommand<'a> {
    pub command: &'a str,
    /// Byte range of `/command` within the original message.
    pub span: std::ops::Range<usize>,
}

pub fn parse_slash_command(message_text: &str) -> Option<ParsedSlashCommand<'_>> {
    let mut trimmed = message_text.trim();

    if COMPACT_TRIGGERS.contains(&trimmed) {
        trimmed = COMPACT_TRIGGERS[0];
    }

    if !trimmed.starts_with('/') {
        return None;
    }

    let command_str = trimmed.strip_prefix('/').unwrap_or(trimmed);
    let (command, params_str) = command_str
        .split_once(' ')
        .map(|(cmd, p)| (cmd, p.trim()))
        .unwrap_or((command_str, ""));

    Some(ParsedSlashCommand {
        command,
        params_str,
    })
}

fn is_skill_command_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, '-' | '_' | '.' | ':')
}

/// User-facing prose with skill slash tokens removed.
///
/// Shared across every expanded skill so `$ARGUMENTS` / `$1` placeholders see
/// one coherent request blob when multiple skills appear in the same message.
pub fn user_text_without_skill_tokens(
    message_text: &str,
    skills: &[InlineSkillCommand<'_>],
) -> String {
    if skills.is_empty() {
        return message_text.trim().to_string();
    }

    let mut ranges = skills
        .iter()
        .map(|skill| skill.span.clone())
        .collect::<Vec<_>>();
    ranges.sort_by_key(|range| range.start);

    let mut pieces = Vec::new();
    let mut cursor = 0usize;
    for range in ranges {
        if range.start > cursor {
            if let Some(piece) = message_text.get(cursor..range.start) {
                pieces.push(piece);
            }
        }
        cursor = range.end.max(cursor);
    }
    if cursor < message_text.len() {
        if let Some(piece) = message_text.get(cursor..) {
            pieces.push(piece);
        }
    }

    pieces
        .join(" ")
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Find every whitespace-bounded `/skill-name` token whose name is accepted by
/// `is_skill`, left-to-right.
///
/// Rules:
/// - `/` must start a token (start of string or preceded by whitespace)
/// - name chars: ASCII alnum plus `-_.:` (plugin skills use `:`)
/// - path-like `/usr/bin` is rejected (name followed immediately by `/`)
pub fn find_inline_skill_commands<'a, F>(
    message_text: &'a str,
    mut is_skill: F,
) -> Vec<InlineSkillCommand<'a>>
where
    F: FnMut(&str) -> bool,
{
    let mut found = Vec::new();
    let mut prev_was_whitespace = true;
    let mut chars = message_text.char_indices().peekable();

    while let Some((i, ch)) = chars.next() {
        if ch == '/' && prev_was_whitespace {
            let name_start = i + ch.len_utf8();
            let mut name_end = name_start;

            while let Some(&(j, c)) = chars.peek() {
                if is_skill_command_name_char(c) {
                    name_end = j + c.len_utf8();
                    chars.next();
                } else {
                    break;
                }
            }

            if name_end > name_start {
                // Indices come from char_indices(); slicing on those boundaries is safe.
                let Some(after_name) = message_text.get(name_end..) else {
                    prev_was_whitespace = false;
                    continue;
                };
                // `/usr/bin`-style paths: reject when another `/` follows immediately.
                if after_name.starts_with('/') {
                    prev_was_whitespace = false;
                    continue;
                }

                let Some(name) = message_text.get(name_start..name_end) else {
                    prev_was_whitespace = false;
                    continue;
                };
                if is_skill(name) {
                    found.push(InlineSkillCommand {
                        command: name,
                        span: i..name_end,
                    });
                }
            }

            prev_was_whitespace = false;
            continue;
        }

        prev_was_whitespace = ch.is_whitespace();
    }

    found
}

/// First matching skill token, if any.
pub fn find_inline_skill_command<'a, F>(
    message_text: &'a str,
    is_skill: F,
) -> Option<InlineSkillCommand<'a>>
where
    F: FnMut(&str) -> bool,
{
    find_inline_skill_commands(message_text, is_skill)
        .into_iter()
        .next()
}

/// Builtin and recipe slash names that must not expand as mid-message skills.
pub fn reserved_inline_skill_names() -> HashSet<String> {
    let mut reserved: HashSet<String> = COMMANDS
        .iter()
        .map(|command| normalize_command_name(command.name))
        .collect();

    for mapping in recipe_slash_command::list_commands() {
        reserved.insert(normalize_command_name(&mapping.command));
    }

    reserved
}

/// Whether `name` is reserved by a builtin or recipe slash command.
pub fn is_reserved_inline_skill_name(name: &str) -> bool {
    reserved_inline_skill_names().contains(&normalize_command_name(name))
}

/// Build the agent-visible message for expanded skills, keeping non-text
/// attachments (images, etc.) from the original user message.
pub fn message_with_skill_prompts(original: Option<&Message>, prompts: String) -> Message {
    let mut message = Message::user().with_text(prompts);
    if let Some(original) = original {
        for content in &original.agent_visible_content().content {
            if !matches!(content, MessageContent::Text(_)) {
                message = message.with_content(content.clone());
            }
        }
    }
    message
}

pub fn list_commands() -> &'static [CommandDef] {
    COMMANDS
}

fn is_clear_goal_param(params_str: &str) -> bool {
    matches!(params_str, "off" | "clear" | "none")
}

/// Whether a slash command should kick off an agent turn instead of just
/// returning a confirmation. Setting a `/goal` or `/grind` (with a description,
/// not the query or `off` forms) makes the agent start pursuing it immediately.
pub fn command_starts_turn(message_text: &str) -> bool {
    let Some(parsed) = parse_slash_command(message_text) else {
        return false;
    };
    matches!(parsed.command, "goal" | "grind")
        && !parsed.params_str.is_empty()
        && !is_clear_goal_param(parsed.params_str)
}

impl Agent {
    pub async fn execute_command(
        &self,
        message_text: &str,
        session_id: &str,
    ) -> Result<Option<Message>> {
        self.execute_command_for_message(message_text, None, session_id)
            .await
    }

    /// Like [`execute_command`], but preserves non-text attachments from
    /// `original_message` when expanding skills (images, etc.).
    pub async fn execute_command_for_message(
        &self,
        message_text: &str,
        original_message: Option<&Message>,
        session_id: &str,
    ) -> Result<Option<Message>> {
        if let Some(parsed) = parse_slash_command(message_text) {
            let command = parsed.command;
            let params_str = parsed.params_str;

            let params: Vec<&str> = if params_str.is_empty() {
                vec![]
            } else {
                params_str.split_whitespace().collect()
            };

            return match command {
                "prompts" => self.handle_prompts_command(&params, session_id).await,
                "prompt" => self.handle_prompt_command(&params, session_id).await,
                "compact" => self.handle_compact_command(session_id).await,
                "clear" => self.handle_clear_command(session_id).await,
                "skills" => self.handle_skills_command(session_id).await,
                "doctor" => Ok(Some(crate::doctor::run(self, session_id).await?)),
                "status" => self.handle_status_command(session_id).await,
                "goal" => self.handle_goal_command(params_str).await,
                "grind" => self.handle_grind_command(params_str).await,
                _ => {
                    if let Some(message) = self
                        .handle_recipe_command(command, params_str, session_id)
                        .await?
                    {
                        #[cfg(feature = "telemetry")]
                        crate::posthog::emit_custom_slash_command_used();
                        return Ok(Some(message));
                    }

                    // Known skills go through the multi-skill expander so a
                    // leading `/a /b prose` loads every skill, not only the first.
                    // Unknown leading tokens still fall through as plain text.
                    if let Some(message) = self
                        .handle_inline_skill_command(message_text, original_message, session_id)
                        .await?
                    {
                        return Ok(Some(message));
                    }

                    Ok(None)
                }
            };
        }

        // Mid-message skill slash: "fix login /code-review focusing on auth"
        self.handle_inline_skill_command(message_text, original_message, session_id)
            .await
    }

    async fn handle_inline_skill_command(
        &self,
        message_text: &str,
        original_message: Option<&Message>,
        session_id: &str,
    ) -> Result<Option<Message>> {
        // Cheap gates before skill-directory discovery (list_commands walks
        // project/global/plugin dirs). Ordinary prompts and URL/path-only `/`
        // forms must not pay for that traversal every turn.
        if !message_text.as_bytes().contains(&b'/') {
            return Ok(None);
        }
        // Token-shape scan (always-true predicate): rejects https://… and /usr/…
        // while still allowing a later real skill after a non-skill token.
        if find_inline_skill_commands(message_text, |_| true).is_empty() {
            return Ok(None);
        }

        let working_dir = self
            .config
            .session_manager
            .get_session(session_id, false)
            .await
            .ok()
            .map(|session| session.working_dir);

        let reserved = reserved_inline_skill_names();
        let skill_entries = skill_slash_command::list_commands(working_dir.as_deref());
        let matches = find_inline_skill_commands(message_text, |name| {
            let normalized = normalize_command_name(name);
            !reserved.contains(&normalized)
                && skill_entries
                    .iter()
                    .any(|skill| skill.name.eq_ignore_ascii_case(name))
        });
        if matches.is_empty() {
            return Ok(None);
        }

        let params_str = user_text_without_skill_tokens(message_text, &matches);
        let mut prompts = Vec::with_capacity(matches.len());

        for inline in &matches {
            // Prefer the installed skill's canonical casing.
            let command = skill_entries
                .iter()
                .find(|skill| skill.name.eq_ignore_ascii_case(inline.command))
                .map(|skill| skill.name.as_str())
                .unwrap_or(inline.command);

            match skill_slash_command::resolve_command(command, &params_str, working_dir.as_deref())
            {
                Ok(None) => {}
                Ok(Some(prompt)) => prompts.push(prompt),
                Err(text) => {
                    // Surface the first resolve error the same way a single
                    // skill slash does (assistant-visible message).
                    return Ok(Some(Message::assistant().with_text(text)));
                }
            }
        }

        if prompts.is_empty() {
            return Ok(None);
        }

        Ok(Some(message_with_skill_prompts(
            original_message,
            prompts.join("\n\n"),
        )))
    }

    async fn handle_compact_command(&self, session_id: &str) -> Result<Option<Message>> {
        let manager = self.config.session_manager.clone();
        let session = manager.get_session(session_id, true).await?;
        let conversation = session
            .conversation
            .ok_or_else(|| anyhow!("Session has no conversation"))?;

        let model_config = self.model_config_for_session(session_id).await?;
        let compaction = compact_messages(
            self.provider().await?.as_ref(),
            &model_config,
            session_id,
            &conversation,
            true, // is_manual_compact
        )
        .await?;

        manager
            .replace_conversation(session_id, &compaction.conversation)
            .await?;

        self.update_session_metrics(
            session_id,
            session.schedule_id,
            &compaction.usage,
            Some(compaction.retained_context_tokens),
        )
        .await?;

        Ok(Some(user_only_assistant_text("Compaction complete")))
    }

    async fn handle_clear_command(&self, session_id: &str) -> Result<Option<Message>> {
        use crate::conversation::Conversation;

        let manager = self.config.session_manager.clone();
        manager
            .replace_conversation(session_id, &Conversation::default())
            .await?;

        manager
            .update(session_id)
            .usage(goose_providers::conversation::token_usage::Usage::new(
                Some(0),
                Some(0),
                Some(0),
            ))
            .apply()
            .await?;

        Ok(Some(user_only_assistant_text("Conversation cleared")))
    }

    async fn handle_skills_command(&self, session_id: &str) -> Result<Option<Message>> {
        let working_dir = self
            .config
            .session_manager
            .get_session(session_id, false)
            .await
            .ok()
            .map(|s| s.working_dir);
        let output = skill_slash_command::format_installed_skills(working_dir.as_deref());
        Ok(Some(Message::assistant().with_text(output)))
    }

    async fn handle_status_command(&self, session_id: &str) -> Result<Option<Message>> {
        let provider = self.provider().await?;
        let model_config = self.model_config_for_session(session_id).await?;
        let context_limit = provider
            .get_context_limit(&model_config)
            .await
            .unwrap_or_else(|_| model_config.context_limit());

        let goose_mode = self.goose_mode().await;

        let metadata = self
            .config
            .session_manager
            .get_session(session_id, false)
            .await
            .ok();

        // `usage` is the current context-window usage (reset by /compact);
        // `accumulated_usage` is the lifetime sum across all responses. The context
        // percentage must use the former, or it inflates and pegs at 100% in any
        // long or post-compaction session.
        let context_tokens = metadata
            .as_ref()
            .and_then(|s| s.usage.total_tokens)
            .unwrap_or(0)
            .max(0) as usize;
        let lifetime_tokens = metadata
            .as_ref()
            .and_then(|s| s.accumulated_usage.total_tokens)
            .unwrap_or(0)
            .max(0) as usize;

        let context_pct = if context_limit > 0 {
            let pct = ((context_tokens as f64 / context_limit as f64) * 100.0).round() as usize;
            format!("{}%", pct.min(100))
        } else {
            "N/A".to_string()
        };

        let text = format!(
            "**Session status**\n\n\
             - Model: {}\n\
             - Provider: {}\n\
             - Mode: {}\n\
             - Tokens (lifetime): {}\n\
             - Context: {} / {} tokens ({})",
            model_config.model_name,
            provider.get_name(),
            goose_mode,
            lifetime_tokens,
            context_tokens,
            context_limit,
            context_pct,
        );

        Ok(Some(user_only_assistant_text(text)))
    }

    async fn handle_prompts_command(
        &self,
        params: &[&str],
        session_id: &str,
    ) -> Result<Option<Message>> {
        let extension_filter = params.first().map(|s| s.to_string());

        let prompts = self.list_extension_prompts(session_id).await;

        if let Some(filter) = &extension_filter {
            if !prompts.contains_key(filter) {
                let error_msg = format!("Extension '{}' not found", filter);
                return Ok(Some(Message::assistant().with_text(error_msg)));
            }
        }

        let filtered_prompts: HashMap<String, Vec<String>> = prompts
            .into_iter()
            .filter(|(ext, _)| extension_filter.as_ref().is_none_or(|f| f == ext))
            .map(|(extension, prompt_list)| {
                let names = prompt_list.into_iter().map(|p| p.name).collect();
                (extension, names)
            })
            .collect();

        let mut output = String::new();
        if filtered_prompts.is_empty() {
            output.push_str("No prompts available.\n");
        } else {
            output.push_str("Available prompts:\n\n");
            for (extension, prompt_names) in filtered_prompts {
                output.push_str(&format!("**{}**:\n", extension));
                for name in prompt_names {
                    output.push_str(&format!("  - {}\n", name));
                }
                output.push('\n');
            }
        }

        Ok(Some(Message::assistant().with_text(output)))
    }

    async fn handle_prompt_command(
        &self,
        params: &[&str],
        session_id: &str,
    ) -> Result<Option<Message>> {
        if params.is_empty() {
            return Ok(Some(
                Message::assistant().with_text("Prompt name argument is required"),
            ));
        }

        let prompt_name = params[0].to_string();
        let is_info = params.get(1).map(|s| *s == "--info").unwrap_or(false);

        if is_info {
            let prompts = self.list_extension_prompts(session_id).await;
            let mut prompt_info = None;

            for (extension, prompt_list) in prompts {
                if let Some(prompt) = prompt_list.iter().find(|p| p.name == prompt_name) {
                    let mut output = format!("**Prompt: {}**\n\n", prompt.name);
                    if let Some(desc) = &prompt.description {
                        output.push_str(&format!("Description: {}\n\n", desc));
                    }
                    output.push_str(&format!("Extension: {}\n\n", extension));

                    if let Some(args) = &prompt.arguments {
                        output.push_str("Arguments:\n");
                        for arg in args {
                            output.push_str(&format!("  - {}", arg.name));
                            if let Some(desc) = &arg.description {
                                output.push_str(&format!(": {}", desc));
                            }
                            output.push('\n');
                        }
                    }

                    prompt_info = Some(output);
                    break;
                }
            }

            return Ok(Some(Message::assistant().with_text(
                prompt_info.unwrap_or_else(|| format!("Prompt '{}' not found", prompt_name)),
            )));
        }

        let mut arguments = HashMap::new();
        for param in params.iter().skip(1) {
            if let Some((key, value)) = param.split_once('=') {
                let value = value.trim_matches('"');
                arguments.insert(key.to_string(), value.to_string());
            }
        }

        let arguments_value = serde_json::to_value(arguments)
            .map_err(|e| anyhow!("Failed to serialize arguments: {}", e))?;

        match self
            .get_prompt(session_id, &prompt_name, arguments_value)
            .await
        {
            Ok(prompt_result) => {
                for (i, prompt_message) in prompt_result.messages.into_iter().enumerate() {
                    let msg = Message::from(prompt_message);

                    let expected_role = if i % 2 == 0 {
                        rmcp::model::Role::User
                    } else {
                        rmcp::model::Role::Assistant
                    };

                    if msg.role != expected_role {
                        let error_msg = format!(
                            "Expected {:?} message at position {}, but found {:?}",
                            expected_role, i, msg.role
                        );
                        return Ok(Some(Message::assistant().with_text(error_msg)));
                    }

                    self.config
                        .session_manager
                        .clone()
                        .add_message(session_id, &msg)
                        .await?;
                }

                let last_message = self
                    .config
                    .session_manager
                    .get_session(session_id, true)
                    .await?
                    .conversation
                    .ok_or_else(|| anyhow!("No conversation found"))?
                    .messages()
                    .last()
                    .cloned()
                    .ok_or_else(|| anyhow!("No messages in conversation"))?;

                Ok(Some(last_message))
            }
            Err(e) => Ok(Some(
                Message::assistant().with_text(format!("Error getting prompt: {}", e)),
            )),
        }
    }

    async fn handle_recipe_command(
        &self,
        command: &str,
        params_str: &str,
        _session_id: &str,
    ) -> Result<Option<Message>> {
        match recipe_slash_command::resolve_command(command, params_str) {
            Ok(None) => Ok(None),
            Ok(Some((response, prompt))) => {
                self.apply_recipe_components(response, true).await;
                Ok(Some(Message::user().with_text(prompt)))
            }
            Err(text) => Ok(Some(Message::assistant().with_text(text))),
        }
    }

    async fn handle_goal_command(&self, params_str: &str) -> Result<Option<Message>> {
        if params_str.is_empty() {
            let current = self.get_goal().await;
            let text = match current {
                Some(goal) => format!("Current goal: {goal}"),
                None => "No goal set. Use `/goal <description>` to set one.".to_string(),
            };
            return Ok(Some(Message::assistant().with_text(text)));
        }

        if is_clear_goal_param(params_str) {
            self.set_goal(None).await;
            return Ok(Some(
                Message::assistant().with_text("Goal cleared. The agent will finish normally."),
            ));
        }

        let goal = params_str.to_string();
        self.set_goal(Some(goal.clone())).await;
        Ok(Some(Message::assistant().with_text(format!(
            "Goal set. The agent will verify this goal is met before finishing:\n\n> {goal}"
        ))))
    }

    async fn handle_grind_command(&self, params_str: &str) -> Result<Option<Message>> {
        if params_str.is_empty() {
            let current = self.get_grind().await;
            let text = match current {
                Some(goal) => format!("Current grind goal: {goal}"),
                None => "No grind goal set. Use `/grind <description>` to set one.".to_string(),
            };
            return Ok(Some(Message::assistant().with_text(text)));
        }

        if is_clear_goal_param(params_str) {
            self.set_grind(None).await;
            return Ok(Some(
                Message::assistant().with_text("Grind cleared. The agent will finish normally."),
            ));
        }

        let goal = params_str.to_string();
        self.set_grind(Some(goal.clone())).await;
        Ok(Some(Message::assistant().with_text(format!(
            "Grind goal set. The agent will keep working until max_turns is reached:\n\n> {goal}"
        ))))
    }
}

fn user_only_assistant_text(text: impl Into<String>) -> Message {
    Message::assistant().with_text(text).user_only()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::MessageContent;

    #[test]
    fn parse_slash_command_splits_on_literal_space() {
        let parsed = parse_slash_command("/speckit.plan hello world").unwrap();

        assert_eq!(parsed.command, "speckit.plan");
        assert_eq!(parsed.params_str, "hello world");
    }

    #[test]
    fn parse_slash_command_does_not_split_on_tab_or_newline() {
        let parsed = parse_slash_command("/speckit.plan\thello").unwrap();
        assert_eq!(parsed.command, "speckit.plan\thello");
        assert_eq!(parsed.params_str, "");

        let parsed = parse_slash_command("/speckit.plan\nhello").unwrap();
        assert_eq!(parsed.command, "speckit.plan\nhello");
        assert_eq!(parsed.params_str, "");
    }

    fn known_skill(name: &str) -> bool {
        matches!(
            name.to_ascii_lowercase().as_str(),
            "code-review" | "deploy" | "plugin:triage"
        )
    }

    #[test]
    fn find_inline_skill_at_start_of_message() {
        let found = find_inline_skill_command("/code-review auth paths", known_skill).unwrap();
        assert_eq!(found.command, "code-review");
        assert_eq!(found.span, 0..12);
        assert_eq!(
            user_text_without_skill_tokens("/code-review auth paths", &[found]),
            "auth paths"
        );
    }

    #[test]
    fn find_inline_skill_mid_message_with_prefix_and_args() {
        let text = "fix login /code-review focusing on auth";
        let found = find_inline_skill_command(text, known_skill).unwrap();
        assert_eq!(found.command, "code-review");
        assert_eq!(
            user_text_without_skill_tokens(text, &[found]),
            "fix login focusing on auth"
        );
    }

    #[test]
    fn find_inline_skill_ignores_unknown_and_builtins() {
        assert!(find_inline_skill_command("please /compact now", known_skill).is_none());
        assert!(find_inline_skill_command("run /unknown-skill please", known_skill).is_none());
    }

    #[test]
    fn find_inline_skill_ignores_urls_and_paths() {
        assert!(
            find_inline_skill_command("see https://example.com/code-review", known_skill).is_none()
        );
        assert!(find_inline_skill_command("open /usr/bin/code-review", known_skill).is_none());
        assert!(find_inline_skill_command("path /code-review/extra", known_skill).is_none());
    }

    #[test]
    fn find_inline_skill_returns_all_matches_left_to_right() {
        let text = "do /deploy then /code-review please";
        let found = find_inline_skill_commands(text, known_skill);
        assert_eq!(
            found.iter().map(|skill| skill.command).collect::<Vec<_>>(),
            vec!["deploy", "code-review"]
        );
        assert_eq!(
            user_text_without_skill_tokens(text, &found),
            "do then please"
        );
    }

    #[test]
    fn find_inline_skill_leading_multi_skill_strips_all_tokens() {
        // Leading-slash prompts used to expand only the first skill via
        // parse_slash_command; multi-skill expansion must cover this shape too.
        let text = "/code-review /deploy review this PR";
        let found = find_inline_skill_commands(text, known_skill);
        assert_eq!(
            found.iter().map(|skill| skill.command).collect::<Vec<_>>(),
            vec!["code-review", "deploy"]
        );
        assert_eq!(
            user_text_without_skill_tokens(text, &found),
            "review this PR"
        );
    }

    #[test]
    fn find_inline_skill_supports_plugin_colon_names() {
        let found =
            find_inline_skill_command("please /plugin:triage this PR", known_skill).unwrap();
        assert_eq!(found.command, "plugin:triage");
        assert_eq!(found.span, 7..21);
    }

    #[test]
    fn find_inline_skill_is_case_insensitive_via_predicate() {
        let found = find_inline_skill_command("run /Code-Review now", known_skill).unwrap();
        assert_eq!(found.command, "Code-Review");
    }

    #[test]
    fn user_text_without_skill_tokens_collapses_whitespace() {
        assert_eq!(
            user_text_without_skill_tokens(
                "  fix   login  /code-review   and  /deploy  now  ",
                &find_inline_skill_commands(
                    "  fix   login  /code-review   and  /deploy  now  ",
                    known_skill
                )
            ),
            "fix login and now"
        );
        assert_eq!(
            user_text_without_skill_tokens("only prose", &[]),
            "only prose"
        );
    }

    #[test]
    fn find_inline_skill_shape_scan_skips_messages_without_token() {
        // handle_inline_skill_command uses an always-true predicate as a cheap
        // precheck before listing installed skills from disk.
        assert!(find_inline_skill_commands("fix the login bug", |_| true).is_empty());
        assert!(find_inline_skill_commands("see https://example.com/path", |_| true).is_empty());

        let candidates = find_inline_skill_commands("please /maybe-a-skill now", |_| true);
        assert_eq!(candidates.len(), 1);
        assert_eq!(candidates[0].command, "maybe-a-skill");
        assert_eq!(
            user_text_without_skill_tokens("please /maybe-a-skill now", &candidates),
            "please now"
        );
    }

    #[test]
    fn reserved_inline_skill_names_include_builtins() {
        let reserved = reserved_inline_skill_names();
        assert!(reserved.contains("compact"));
        assert!(reserved.contains("clear"));
        assert!(reserved.contains("status"));
        assert!(is_reserved_inline_skill_name("Compact"));
        assert!(!is_reserved_inline_skill_name("code-review"));
    }

    #[test]
    fn inline_skill_predicate_excludes_reserved_even_if_skill_exists() {
        // A skill named "compact" must not match mid-message.
        let is_skill = |name: &str| {
            !is_reserved_inline_skill_name(name)
                && matches!(
                    name.to_ascii_lowercase().as_str(),
                    "compact" | "code-review"
                )
        };
        assert!(find_inline_skill_command("please /compact now", is_skill).is_none());
        assert_eq!(
            find_inline_skill_command("please /code-review now", is_skill)
                .unwrap()
                .command,
            "code-review"
        );
    }

    #[test]
    fn message_with_skill_prompts_preserves_images() {
        let original = Message::user()
            .with_text("analyze this /ui-review")
            .with_image("abc123", "image/png");
        let expanded =
            message_with_skill_prompts(Some(&original), "# Loaded Skill: ui-review".to_string());

        assert_eq!(expanded.as_concat_text(), "# Loaded Skill: ui-review");
        assert!(
            expanded
                .content
                .iter()
                .any(|c| matches!(c, MessageContent::Image(_))),
            "expected image attachment to be preserved"
        );
        // Original user text is replaced by skill prompts, not duplicated.
        assert!(!expanded.as_concat_text().contains("analyze this"));
    }

    #[test]
    fn message_with_skill_prompts_without_original_is_text_only() {
        let expanded = message_with_skill_prompts(None, "skill body".to_string());
        assert_eq!(expanded.as_concat_text(), "skill body");
        assert_eq!(expanded.content.len(), 1);
    }

    #[test]
    fn command_starts_turn_only_for_goal_and_grind_with_description() {
        assert!(command_starts_turn("/goal make all tests pass"));
        assert!(command_starts_turn("/grind keep refactoring"));

        // Query and clear forms must not start a turn.
        assert!(!command_starts_turn("/goal"));
        assert!(!command_starts_turn("/goal off"));
        assert!(!command_starts_turn("/goal clear"));
        assert!(!command_starts_turn("/goal none"));
        assert!(!command_starts_turn("/grind"));
        assert!(!command_starts_turn("/grind off"));

        // Other commands and plain prompts never start a turn here.
        assert!(!command_starts_turn("/compact"));
        assert!(!command_starts_turn("just a normal message"));
    }

    #[test]
    fn user_only_assistant_text_is_durable_text_not_system_notification() {
        let message = user_only_assistant_text("Conversation cleared");

        assert!(message.metadata.user_visible);
        assert!(!message.metadata.agent_visible);
        assert_eq!(message.role, rmcp::model::Role::Assistant);
        assert!(matches!(
            message.content.as_slice(),
            [MessageContent::Text(text)] if text.text == "Conversation cleared"
        ));
    }

    #[test]
    fn status_is_registered_as_a_builtin_command() {
        assert!(list_commands()
            .iter()
            .any(|command| command.name == "status"));
    }
}
