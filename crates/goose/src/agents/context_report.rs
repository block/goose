use std::collections::BTreeMap;

use anyhow::Result;
use rmcp::model::{Role, Tool};

pub use goose_sdk_types::custom_requests::{
    ContextCategory, ContextPart, ContextReportModel, ContextReportResponse, ContextSegment,
};

use crate::agents::extension_manager::get_tool_owner;
use crate::agents::Agent;
use crate::conversation::message::Message;
use crate::conversation::{is_turn_context_text, Conversation};
use crate::hints::{combine_hint_sources, HintScope, HintSource};
use crate::providers::toolshim::{
    convert_tool_messages_to_text, format_tool_info, toolshim_system_prompt_appendix,
};
use crate::token_counter::TokenCounter;
use crate::utils::sanitize_unicode_tags;

const MAX_PREVIEW_CHARS: usize = 2_000;
const TURN_CONTEXT_PLACEHOLDER_ID: &str = "context-report-turn-context-placeholder";
const TURN_CONTEXT_PLACEHOLDER_TEXT: &str = "[context report placeholder]";

fn preview(text: &str) -> String {
    let count = text.chars().count();
    if count <= MAX_PREVIEW_CHARS {
        return text.to_string();
    }
    let kept: String = text.chars().take(MAX_PREVIEW_CHARS).collect();
    format!("{kept}… (+{} more chars)", count - MAX_PREVIEW_CHARS)
}

fn text_segment(
    category: ContextCategory,
    label: String,
    source: Option<String>,
    text: &str,
    token_counter: &TokenCounter,
) -> ContextSegment {
    ContextSegment {
        category,
        label,
        source,
        token_count: token_counter.count_tokens(text) as u64,
        char_count: text.chars().count() as u64,
        content_preview: Some(preview(text)),
        parts: Vec::new(),
    }
}

impl Agent {
    pub async fn build_context_report(
        &self,
        session_id: &str,
        token_counter: &TokenCounter,
    ) -> Result<ContextReportResponse> {
        let session = self
            .config
            .session_manager
            .get_session(session_id, true)
            .await?;
        let working_dir = session.working_dir.as_path();
        let persisted_conversation = session
            .conversation
            .clone()
            .unwrap_or_else(|| Conversation::new_unvalidated(Vec::new()));
        let report_is_empty = persisted_conversation.is_empty();
        let (conversation_for_turn_context, added_placeholder) =
            with_turn_context_placeholder(persisted_conversation);
        let context = self
            .prepare_reply_context(session_id, conversation_for_turn_context, working_dir)
            .await?;
        let tools = context.tools;
        let toolshim_tools = context.toolshim_tools;
        let mut system_prompt = context.system_prompt;
        let model_config = context.model_config;
        let hint_sources = context.hint_sources;
        let project_addendum = self.load_project_instructions(&session).await;
        let conversation = if report_is_empty {
            Conversation::empty()
        } else {
            context.conversation
        };

        if let Some(project_addendum) = &project_addendum {
            system_prompt = format!("{system_prompt}\n\n{project_addendum}");
        }

        let mut conversation_with_moim = super::moim::inject_moim(
            session_id,
            conversation.clone(),
            &self.extension_manager,
            0,
            0,
        )
        .await;
        let turn_context = turn_context_text(&conversation_with_moim);

        if added_placeholder {
            if turn_context.is_some() {
                remove_turn_context_placeholder_marker(&mut conversation_with_moim);
            } else {
                conversation_with_moim
                    .messages_mut()
                    .retain(|message| !is_turn_context_placeholder(message));
            }
        }

        let display_tools = if model_config.toolshim {
            &toolshim_tools
        } else {
            &tools
        };

        let extensions_info = self
            .extension_manager
            .get_extensions_info(working_dir)
            .await;
        let (extension_count, tool_count) = self.total_extension_and_tool_counts(session_id).await;
        let goose_mode = *self.current_goose_mode.lock().await;
        let frontend_instructions = self.frontend_instructions.lock().await.clone();
        let code_execution_active = self.is_code_execution_active().await;

        let pieces = {
            let prompt_manager = self.prompt_manager.lock().await;
            prompt_manager
                .builder()
                .with_extensions(extensions_info.into_iter())
                .with_frontend_instructions(frontend_instructions)
                .with_extension_and_tool_counts(extension_count, tool_count)
                .with_code_execution_mode(code_execution_active)
                .with_hint_sources(&hint_sources)
                .with_goose_mode(goose_mode)
                .build_segments()
        };

        let base_source = if pieces.base_is_override {
            "override".to_string()
        } else {
            "prompts/system.md".to_string()
        };

        let mut segments = Vec::new();

        segments.push(text_segment(
            ContextCategory::SystemPrompt,
            "Base system prompt".to_string(),
            Some(base_source),
            &pieces.base_template_blanked,
            token_counter,
        ));

        for (name, instructions) in &pieces.extension_instructions {
            segments.push(text_segment(
                ContextCategory::ExtensionInstructions,
                name.clone(),
                Some(format!("extension:{name}")),
                instructions,
                token_counter,
            ));
        }

        for (key, value) in &pieces.extras {
            let parts = if key == "hints" {
                hint_parts(&hint_sources, token_counter)
            } else {
                Vec::new()
            };
            let source = key.strip_prefix("subdir_hints:").map(|dir| dir.to_string());
            let mut segment = text_segment(
                ContextCategory::AdditionalInstructions,
                key.clone(),
                source,
                value,
                token_counter,
            );
            segment.parts = parts;
            segments.push(segment);
        }

        if let Some(project_addendum) = &project_addendum {
            segments.push(text_segment(
                ContextCategory::AdditionalInstructions,
                "Project instructions".to_string(),
                session
                    .project_id
                    .as_ref()
                    .map(|id| format!("project:{id}")),
                project_addendum,
                token_counter,
            ));
        }

        if let Some(moim) = &pieces.moim_block {
            segments.push(text_segment(
                ContextCategory::TurnContext,
                "Turn context instructions".to_string(),
                None,
                moim,
                token_counter,
            ));
        }

        if let Some(turn_context) = &turn_context {
            segments.push(text_segment(
                ContextCategory::TurnContext,
                "Current turn context".to_string(),
                None,
                turn_context,
                token_counter,
            ));
        }

        if model_config.toolshim {
            segments.push(toolshim_tool_segment(display_tools, token_counter));
        } else {
            segments.extend(tool_segments(display_tools, token_counter));
        }

        let visible_messages: Vec<Message> = conversation
            .messages()
            .iter()
            .filter(|message| message.is_agent_visible() && !is_turn_context_placeholder(message))
            .map(|m| m.agent_visible_content())
            .collect();

        let display_provider_messages = if model_config.toolshim {
            convert_tool_messages_to_text(&visible_messages)
        } else {
            crate::conversation::Conversation::new_unvalidated(visible_messages.clone())
        };

        let provider_visible_messages: Vec<Message> = conversation_with_moim
            .messages()
            .iter()
            .filter(|message| message.is_agent_visible())
            .map(|m| m.agent_visible_content())
            .collect();
        let provider_messages = if model_config.toolshim {
            convert_tool_messages_to_text(&provider_visible_messages)
        } else {
            crate::conversation::Conversation::new_unvalidated(provider_visible_messages)
        };

        let wire_total_tokens =
            token_counter.count_chat_tokens(&system_prompt, provider_messages.messages(), &tools)
                as u64;

        segments.extend(message_segments(
            &visible_messages,
            display_provider_messages.messages(),
            token_counter,
        ));

        let segment_tokens: u64 = segments.iter().map(|segment| segment.token_count).sum();
        let residual = wire_total_tokens.saturating_sub(segment_tokens);
        if residual > 0 {
            segments.push(ContextSegment {
                category: ContextCategory::SystemPrompt,
                label: "Prompt overhead".to_string(),
                source: None,
                token_count: residual,
                char_count: 0,
                content_preview: None,
                parts: Vec::new(),
            });
        }

        let estimated_total_tokens = segments.iter().map(|segment| segment.token_count).sum();

        let live_total_tokens = session
            .usage
            .total_tokens
            .and_then(|total| u64::try_from(total).ok());

        let context_limit = match self.provider().await {
            Ok(provider) => provider
                .get_context_limit(&model_config)
                .await
                .unwrap_or_else(|_| model_config.context_limit()),
            Err(_) => model_config.context_limit(),
        } as u64;

        Ok(ContextReportResponse {
            model: ContextReportModel {
                provider: session.provider_name.clone(),
                model_name: model_config.model_name.clone(),
                context_limit,
            },
            estimated_total_tokens,
            wire_total_tokens,
            live_total_tokens,
            segments,
        })
    }
}

fn with_turn_context_placeholder(mut conversation: Conversation) -> (Conversation, bool) {
    let last_visible_role = conversation
        .messages()
        .iter()
        .rev()
        .find(|message| message.is_agent_visible())
        .map(|message| message.role.clone());

    if last_visible_role == Some(Role::Assistant) {
        conversation.push(
            Message::user()
                .with_id(TURN_CONTEXT_PLACEHOLDER_ID)
                .with_text(TURN_CONTEXT_PLACEHOLDER_TEXT),
        );
        (conversation, true)
    } else {
        (conversation, false)
    }
}

fn is_turn_context_placeholder(message: &Message) -> bool {
    message.id.as_deref() == Some(TURN_CONTEXT_PLACEHOLDER_ID)
}

fn remove_turn_context_placeholder_marker(conversation: &mut Conversation) {
    if let Some(message) = conversation
        .messages_mut()
        .iter_mut()
        .find(|message| is_turn_context_placeholder(message))
    {
        message
            .content
            .retain(|content| content.as_text() != Some(TURN_CONTEXT_PLACEHOLDER_TEXT));
    }
}

fn turn_context_text(conversation: &Conversation) -> Option<String> {
    conversation
        .messages()
        .iter()
        .rev()
        .flat_map(|message| message.content.iter().rev())
        .filter_map(|content| content.as_text())
        .find(|text| is_turn_context_text(text))
        .map(str::to_string)
}

/// Marginal diffing over a cumulative prefix, so the scope headers
/// `combine_hint_sources` inserts are charged to the file that introduces them
/// and the parts sum to the combined hint block the segment measures. The
/// sanitization mirrors what the builder applies to every extra.
fn hint_parts(sources: &[HintSource], token_counter: &TokenCounter) -> Vec<ContextPart> {
    let mut cumulative: Vec<HintSource> = Vec::with_capacity(sources.len());
    let mut cumulative_tokens = 0usize;
    let mut cumulative_chars = 0usize;
    let mut parts = Vec::with_capacity(sources.len());

    for source in sources {
        cumulative.push(source.clone());
        let combined = sanitize_unicode_tags(&combine_hint_sources(&cumulative));
        let before_tokens = cumulative_tokens;
        let before_chars = cumulative_chars;
        cumulative_tokens = token_counter.count_tokens(&combined);
        cumulative_chars = combined.chars().count();
        parts.push(ContextPart {
            label: match source.scope {
                HintScope::Global => "global",
                HintScope::Project => "project",
            }
            .to_string(),
            source: Some(source.path.display().to_string()),
            token_count: (cumulative_tokens - before_tokens) as u64,
            char_count: (cumulative_chars - before_chars) as u64,
            content_preview: Some(preview(&source.content)),
        });
    }

    parts
}

fn tool_content(tool: &Tool) -> String {
    serde_json::to_string_pretty(&serde_json::json!({
        "name": tool.name,
        "description": tool.description,
        "inputSchema": tool.input_schema,
    }))
    .unwrap_or_default()
}

fn tool_segments(tools: &[Tool], token_counter: &TokenCounter) -> Vec<ContextSegment> {
    let mut groups: BTreeMap<String, Vec<Tool>> = BTreeMap::new();
    for tool in tools {
        let owner = get_tool_owner(tool).unwrap_or_else(|| "ungrouped".to_string());
        groups.entry(owner).or_default().push(tool.clone());
    }

    // Marginal diffing over a cumulative tool prefix, so parts sum to their segment and
    // segments sum to the single count_tokens_for_tools call inside wire_total_tokens.
    let mut cumulative: Vec<Tool> = Vec::new();
    let mut cumulative_tokens = 0usize;
    let mut segments = Vec::new();
    for (owner, group) in groups {
        let group_start = cumulative_tokens;
        let mut parts = Vec::with_capacity(group.len());
        for tool in group {
            let content = tool_content(&tool);
            let before = cumulative_tokens;
            cumulative.push(tool.clone());
            cumulative_tokens = token_counter.count_tokens_for_tools(&cumulative);
            parts.push(ContextPart {
                label: tool.name.to_string(),
                source: None,
                token_count: (cumulative_tokens - before) as u64,
                char_count: content.chars().count() as u64,
                content_preview: Some(preview(&content)),
            });
        }
        segments.push(ContextSegment {
            category: ContextCategory::ToolDefinitions,
            token_count: (cumulative_tokens - group_start) as u64,
            char_count: parts.iter().map(|part| part.char_count).sum(),
            label: owner,
            source: Some(format!("{} tools", parts.len())),
            content_preview: None,
            parts,
        });
    }
    segments
}

/// Toolshim tools live in the system prompt as `format_tool_info` text, not as a
/// tool spec, so parts are marginal diffs over that same rendering with the fixed
/// calling-convention preamble carried as its own part.
fn toolshim_tool_segment(tools: &[Tool], token_counter: &TokenCounter) -> ContextSegment {
    let mut cumulative: Vec<Tool> = Vec::with_capacity(tools.len());
    let mut appendix = toolshim_system_prompt_appendix(&cumulative);
    let mut cumulative_tokens = token_counter.count_tokens(&appendix);
    let mut cumulative_chars = appendix.chars().count();

    let mut parts = Vec::with_capacity(tools.len() + 1);
    parts.push(ContextPart {
        label: "Tool calling instructions".to_string(),
        source: None,
        token_count: cumulative_tokens as u64,
        char_count: cumulative_chars as u64,
        content_preview: Some(preview(&appendix)),
    });

    for tool in tools {
        cumulative.push(tool.clone());
        appendix = toolshim_system_prompt_appendix(&cumulative);
        let before_tokens = cumulative_tokens;
        let before_chars = cumulative_chars;
        cumulative_tokens = token_counter.count_tokens(&appendix);
        cumulative_chars = appendix.chars().count();
        parts.push(ContextPart {
            label: tool.name.to_string(),
            source: None,
            token_count: (cumulative_tokens - before_tokens) as u64,
            char_count: (cumulative_chars - before_chars) as u64,
            content_preview: Some(preview(&format_tool_info(std::slice::from_ref(tool)))),
        });
    }

    ContextSegment {
        category: ContextCategory::ToolDefinitions,
        label: "Toolshim tool instructions".to_string(),
        source: Some(format!("{} tools", tools.len())),
        token_count: cumulative_tokens as u64,
        char_count: cumulative_chars as u64,
        content_preview: Some(preview(&appendix)),
        parts,
    }
}

fn message_text(message: &Message) -> String {
    message
        .content
        .iter()
        .filter_map(crate::token_counter::message_content_token_text)
        .collect::<Vec<_>>()
        .join("\n")
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum MessageKind {
    User,
    Assistant,
    ToolCalls,
    ToolResults,
}

impl MessageKind {
    fn classify(message: &Message) -> Self {
        if message
            .content
            .iter()
            .any(|c| c.as_tool_response().is_some())
        {
            MessageKind::ToolResults
        } else if message
            .content
            .iter()
            .any(|c| c.as_tool_request().is_some())
        {
            MessageKind::ToolCalls
        } else if message.role == Role::Assistant {
            MessageKind::Assistant
        } else {
            MessageKind::User
        }
    }

    fn label(self) -> &'static str {
        match self {
            MessageKind::User => "User messages",
            MessageKind::Assistant => "Assistant messages",
            MessageKind::ToolCalls => "Tool calls",
            MessageKind::ToolResults => "Tool results",
        }
    }
}

/// Splits a rendered compaction summary into its `##` sections. Boundaries come
/// from the rendered markdown rather than the parsed `StructuredSummary` so the
/// breakdown reports what the context window actually holds, including when
/// `compaction_summary.md` has been customized to drop or rename sections.
///
/// Fenced blocks are skipped over because `key_code` content can legally
/// contain heading lines.
fn summary_sections(text: &str) -> Vec<(String, String)> {
    let mut sections: Vec<(String, String)> = Vec::new();
    let mut open_fence: Option<usize> = None;

    for line in text.lines() {
        let backticks = line.trim_start().chars().take_while(|c| *c == '`').count();
        match open_fence {
            Some(len) if backticks >= len => open_fence = None,
            Some(_) => {}
            None if backticks >= 3 => open_fence = Some(backticks),
            None => {
                if let Some(heading) = line.strip_prefix("## ") {
                    sections.push((heading.trim().to_string(), String::new()));
                    continue;
                }
            }
        }
        if let Some((_, body)) = sections.last_mut() {
            body.push_str(line);
            body.push('\n');
        }
    }

    sections.retain_mut(|(_, body)| {
        *body = body.trim().to_string();
        !body.is_empty()
    });
    sections
}

fn compaction_summary_segment(
    message: &Message,
    counted_message: &Message,
    structured: bool,
    token_counter: &TokenCounter,
) -> ContextSegment {
    let text = message_text(message);
    let parts: Vec<ContextPart> = if structured {
        summary_sections(&text)
            .into_iter()
            .map(|(heading, body)| ContextPart {
                token_count: token_counter.count_tokens(&body) as u64,
                char_count: body.chars().count() as u64,
                content_preview: Some(preview(&body)),
                label: heading,
                source: None,
            })
            .collect()
    } else {
        Vec::new()
    };

    ContextSegment {
        category: ContextCategory::CompactionSummary,
        label: "Conversation summary".to_string(),
        source: Some(
            if structured {
                "structured"
            } else {
                "raw fallback"
            }
            .to_string(),
        ),
        token_count: token_counter.count_message_tokens(counted_message) as u64,
        char_count: text.chars().count() as u64,
        content_preview: parts.is_empty().then(|| preview(&text)),
        parts,
    }
}

fn message_segments(
    display_messages: &[Message],
    counted_messages: &[Message],
    token_counter: &TokenCounter,
) -> Vec<ContextSegment> {
    let mut summary_segments = Vec::new();
    let mut user_parts = Vec::new();
    let mut assistant_parts = Vec::new();
    let mut tool_call_parts = Vec::new();
    let mut tool_result_parts = Vec::new();

    for (index, message) in display_messages.iter().enumerate() {
        let counted_message = counted_messages.get(index).unwrap_or(message);

        if let Some(summary) = message.metadata.compaction_summary {
            summary_segments.push(compaction_summary_segment(
                message,
                counted_message,
                summary.structured,
                token_counter,
            ));
            continue;
        }

        let kind = MessageKind::classify(message);
        let role = match message.role {
            Role::Assistant => "assistant",
            _ => "user",
        };
        let text = message_text(message);
        let part = ContextPart {
            label: format!("#{} {role}", index + 1),
            source: None,
            token_count: token_counter.count_message_tokens(counted_message) as u64,
            char_count: text.chars().count() as u64,
            content_preview: Some(preview(&text)),
        };
        match kind {
            MessageKind::User => user_parts.push(part),
            MessageKind::Assistant => assistant_parts.push(part),
            MessageKind::ToolCalls => tool_call_parts.push(part),
            MessageKind::ToolResults => tool_result_parts.push(part),
        }
    }

    let message_kind_segments = [
        (MessageKind::User, user_parts),
        (MessageKind::Assistant, assistant_parts),
        (MessageKind::ToolCalls, tool_call_parts),
        (MessageKind::ToolResults, tool_result_parts),
    ]
    .into_iter()
    .filter(|(_, parts)| !parts.is_empty())
    .map(|(kind, parts)| ContextSegment {
        category: ContextCategory::Messages,
        label: kind.label().to_string(),
        source: None,
        token_count: parts.iter().map(|part| part.token_count).sum(),
        char_count: parts.iter().map(|part| part.char_count).sum(),
        content_preview: None,
        parts,
    });

    summary_segments
        .into_iter()
        .chain(message_kind_segments)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::conversation::message::{CompactionSummaryMetadata, MessageMetadata};
    use std::sync::Arc;

    #[test]
    fn turn_context_placeholder_preserves_idle_assistant_message() {
        let assistant = Message::assistant().with_text("latest assistant reply");
        let conversation = Conversation::new_unvalidated([
            Message::user().with_text("first user message"),
            assistant.clone(),
        ]);

        let (with_placeholder, added_placeholder) = with_turn_context_placeholder(conversation);

        assert!(added_placeholder);
        assert_eq!(with_placeholder.messages()[1], assistant);
        assert!(is_turn_context_placeholder(
            with_placeholder.messages().last().unwrap()
        ));
    }

    #[test]
    fn turn_context_text_finds_only_dynamic_turn_context() {
        let turn_context = "<turn-context>\n<current-time>2026-07-09 12:00:00</current-time>\n<working-directory>/tmp</working-directory>\n</turn-context>";
        let conversation = Conversation::new_unvalidated([
            Message::user().with_text("ordinary user message"),
            Message::user().with_text(turn_context),
        ]);

        assert_eq!(
            turn_context_text(&conversation).as_deref(),
            Some(turn_context)
        );
    }

    #[test]
    fn turn_context_text_prefers_freshest_turn_context() {
        let stale_turn_context = "<turn-context>\n<current-time>2026-07-09 12:00:00</current-time>\n<working-directory>/old</working-directory>\n</turn-context>";
        let fresh_turn_context = "<turn-context>\n<current-time>2026-07-13 09:30:00</current-time>\n<working-directory>/new</working-directory>\n</turn-context>";
        let conversation = Conversation::new_unvalidated([
            Message::user().with_text(stale_turn_context),
            Message::user().with_text("do some work"),
            Message::assistant().with_text("done"),
            Message::user().with_text(fresh_turn_context),
        ]);

        assert_eq!(
            turn_context_text(&conversation).as_deref(),
            Some(fresh_turn_context)
        );
    }

    fn test_tool(name: &str, description: &str, owner: Option<&str>) -> Tool {
        let schema = serde_json::json!({
            "type": "object",
            "properties": {
                "path": { "type": "string", "description": "File path to operate on." },
                "mode": { "type": "string", "enum": ["read", "write"] }
            }
        });
        let mut tool = Tool::new(
            name.to_string(),
            description.to_string(),
            Arc::new(schema.as_object().unwrap().clone()),
        );
        if let Some(owner) = owner {
            let mut meta = serde_json::Map::new();
            meta.insert(
                "goose_extension".to_string(),
                serde_json::Value::String(owner.to_string()),
            );
            tool.meta = Some(rmcp::model::Meta(meta));
        }
        tool
    }

    #[tokio::test]
    async fn tool_segments_account_for_every_tool_token_exactly() {
        let token_counter = TokenCounter::new().await.unwrap();
        let tools = vec![
            test_tool("alpha__read", "Read a file from disk.", Some("alpha")),
            test_tool("alpha__write", "Write a file to disk.", Some("alpha")),
            test_tool("beta__search", "Search the project tree.", Some("beta")),
            test_tool("orphan", "A tool without an owner.", None),
        ];

        let segments = tool_segments(&tools, &token_counter);
        let expected_total = token_counter.count_tokens_for_tools(&tools) as u64;

        assert_eq!(segments.len(), 3);
        for segment in &segments {
            assert_eq!(segment.category, ContextCategory::ToolDefinitions);
            assert_eq!(
                segment.token_count,
                segment
                    .parts
                    .iter()
                    .map(|part| part.token_count)
                    .sum::<u64>()
            );
        }
        let segment_total: u64 = segments.iter().map(|segment| segment.token_count).sum();
        let part_total: u64 = segments
            .iter()
            .flat_map(|segment| &segment.parts)
            .map(|part| part.token_count)
            .sum();
        assert_eq!(segment_total, expected_total);
        assert_eq!(part_total, expected_total);
    }

    #[tokio::test]
    async fn toolshim_segment_parts_account_for_every_appendix_token_exactly() {
        let token_counter = TokenCounter::new().await.unwrap();
        let tools = vec![
            test_tool("alpha__read", "Read a file from disk.", Some("alpha")),
            test_tool("beta__search", "Search the project tree.", Some("beta")),
        ];

        let segment = toolshim_tool_segment(&tools, &token_counter);
        let appendix = toolshim_system_prompt_appendix(&tools);

        assert_eq!(
            segment.token_count,
            token_counter.count_tokens(&appendix) as u64
        );
        assert_eq!(segment.char_count, appendix.chars().count() as u64);
        assert_eq!(segment.parts.len(), tools.len() + 1);
        assert_eq!(
            segment.token_count,
            segment
                .parts
                .iter()
                .map(|part| part.token_count)
                .sum::<u64>()
        );
        assert_eq!(
            segment.char_count,
            segment
                .parts
                .iter()
                .map(|part| part.char_count)
                .sum::<u64>()
        );
    }

    #[tokio::test]
    async fn hint_parts_account_for_the_combined_hint_block_exactly() {
        let token_counter = TokenCounter::new().await.unwrap();
        let sources = vec![
            HintSource {
                path: "/config/goose/AGENTS.md".into(),
                scope: HintScope::Global,
                content: "Never use an em dash.".to_string(),
            },
            HintSource {
                path: "/repo/AGENTS.md".into(),
                scope: HintScope::Project,
                content: "Run cargo fmt before committing.".to_string(),
            },
            HintSource {
                path: "/repo/CLAUDE.md".into(),
                scope: HintScope::Project,
                content: "Tests live in tests/.".to_string(),
            },
        ];

        let combined = combine_hint_sources(&sources);
        let parts = hint_parts(&sources, &token_counter);

        assert_eq!(parts.len(), sources.len());
        assert_eq!(
            parts.iter().map(|part| part.token_count).sum::<u64>(),
            token_counter.count_tokens(&combined) as u64,
            "the scope headers combine_hint_sources adds must be charged to a part"
        );
        assert_eq!(
            parts.iter().map(|part| part.char_count).sum::<u64>(),
            combined.chars().count() as u64
        );
    }

    const RENDERED_SUMMARY: &str = "# Conversation Summary

## User Intent
- Fix the parser bug

## Files + Code
### src/parser.rs
Fixed off-by-one in scan loop
````
## heading inside key_code
fn scan() {}
````

## Next Step
Finish the regression test
";

    fn summary_message(text: &str, structured: bool) -> Message {
        Message::user()
            .with_text(text)
            .with_metadata(MessageMetadata {
                compaction_summary: Some(CompactionSummaryMetadata { structured }),
                ..MessageMetadata::agent_only()
            })
    }

    #[test]
    fn summary_sections_split_on_headings_outside_fences() {
        let sections = summary_sections(RENDERED_SUMMARY);

        let headings: Vec<&str> = sections
            .iter()
            .map(|(heading, _)| heading.as_str())
            .collect();
        assert_eq!(headings, vec!["User Intent", "Files + Code", "Next Step"]);
        assert!(
            sections[1].1.contains("## heading inside key_code"),
            "a heading inside a fenced block must stay in its section"
        );
    }

    #[tokio::test]
    async fn compaction_summary_is_its_own_segment_and_leaves_totals_intact() {
        let token_counter = TokenCounter::new().await.unwrap();
        let messages = vec![
            summary_message(RENDERED_SUMMARY, true),
            Message::assistant().with_text("continuing where we left off"),
            Message::user().with_text("now add the test"),
        ];

        let segments = message_segments(&messages, &messages, &token_counter);
        let summary = segments
            .iter()
            .find(|segment| segment.category == ContextCategory::CompactionSummary)
            .expect("summary should have its own segment");

        assert_eq!(summary.source.as_deref(), Some("structured"));
        assert_eq!(summary.parts.len(), 3);
        assert!(summary.content_preview.is_none());
        assert_eq!(
            summary.token_count,
            token_counter.count_message_tokens(&messages[0]) as u64
        );

        let expected_total: u64 = messages
            .iter()
            .map(|message| token_counter.count_message_tokens(message) as u64)
            .sum();
        let segment_total: u64 = segments.iter().map(|segment| segment.token_count).sum();
        let conversation_total: u64 = segments
            .iter()
            .filter(|segment| segment.category == ContextCategory::Messages)
            .map(|segment| segment.token_count)
            .sum();

        assert_eq!(segment_total, expected_total);
        assert_eq!(conversation_total, expected_total - summary.token_count);
    }

    #[tokio::test]
    async fn raw_fallback_summary_reports_no_sections() {
        let token_counter = TokenCounter::new().await.unwrap();
        let messages = vec![summary_message(
            "I ran out of room and could not produce the summary document.",
            false,
        )];

        let segments = message_segments(&messages, &messages, &token_counter);

        assert_eq!(segments.len(), 1);
        assert_eq!(segments[0].source.as_deref(), Some("raw fallback"));
        assert!(segments[0].parts.is_empty());
        assert!(segments[0].content_preview.is_some());
    }
}
