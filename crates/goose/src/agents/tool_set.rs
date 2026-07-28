use std::collections::{HashMap, HashSet};

use rmcp::model::Tool;
use tokio::sync::Mutex;

use crate::agents::extension::ExtensionInfo;
use crate::conversation::message::{
    Message, MessageContentBlock, MessageMetadata, ToolSetUpdateContent,
};
use crate::conversation::{resolve_visible_tools, Conversation};
use crate::providers::formats::anthropic::{defer_tool_loading, offer_tool_immediately};

/// Everything goose has offered the model during a session, per session.
///
/// Anthropic hashes a request prefix as `tools` -> `system` -> `messages`, so the cache only
/// survives if the tools and the system prompt both stay byte-identical for the life of the
/// conversation. Disabling an extension therefore leaves its tools declared and hides them with a
/// delta instead.
///
/// A cache, not a source of truth: it is rebuilt from what is enabled on session resume.
#[derive(Default)]
pub struct DeclaredSurfaces(Mutex<HashMap<String, Surface>>);

#[derive(Default)]
struct Surface {
    tools: Vec<Tool>,
    /// Withheld in the array until an addition reveals them, so the model does not read a tool as
    /// available before it was enabled.
    deferred: HashSet<String>,
    /// Counted before the filters that decide what is offered, the unit `tool_count` is in.
    counted: HashSet<String>,
    extensions: Vec<ExtensionInfo>,
    frontend_instructions: Vec<(String, String)>,
}

/// Covers the declared surface, not only what is enabled now.
pub struct DeclaredPrompt {
    pub extensions: Vec<ExtensionInfo>,
    pub frontend_instructions: Option<String>,
    pub extension_count: usize,
    pub tool_count: usize,
}

/// What a session has enabled right now, as the prompt would describe it without a declared
/// surface. The totals cover tool and extension sources the lists themselves do not hold.
pub struct EnabledSurface {
    /// Taken before the filters that decide what the model is offered, the unit `tool_total` is in.
    pub counted_tool_names: Vec<String>,
    pub extensions: Vec<ExtensionInfo>,
    pub frontend_instructions: Vec<(String, String)>,
    pub extension_total: usize,
    pub tool_total: usize,
}

impl DeclaredSurfaces {
    pub async fn declare(
        &self,
        session_id: &str,
        enabled_tools: &[Tool],
        enabled: EnabledSurface,
    ) -> DeclaredPrompt {
        let EnabledSurface {
            counted_tool_names,
            extensions: enabled_extensions,
            frontend_instructions: enabled_frontend_instructions,
            extension_total,
            tool_total,
        } = enabled;

        let mut sessions = self.0.lock().await;
        let surface = sessions.entry(session_id.to_string()).or_default();
        surface.declare_tools(enabled_tools);

        let counted_enabled = counted_tool_names.len();
        surface.counted.extend(counted_tool_names);

        let enabled_extension_count = enabled_extensions.len();
        for extension in enabled_extensions {
            match surface
                .extensions
                .iter_mut()
                .find(|declared| declared.name == extension.name)
            {
                Some(declared) => *declared = extension,
                None => surface.extensions.push(extension),
            }
        }

        let enabled_frontend_extension_count = enabled_frontend_instructions.len();
        for (name, instructions) in enabled_frontend_instructions {
            match surface
                .frontend_instructions
                .iter_mut()
                .find(|(declared_name, _)| declared_name == &name)
            {
                Some(declared) => *declared = (name, instructions),
                None => surface.frontend_instructions.push((name, instructions)),
            }
        }
        surface
            .frontend_instructions
            .sort_by(|(a, _), (b, _)| a.cmp(b));

        DeclaredPrompt {
            extension_count: extension_total
                + surface
                    .extensions
                    .len()
                    .saturating_sub(enabled_extension_count)
                + surface
                    .frontend_instructions
                    .len()
                    .saturating_sub(enabled_frontend_extension_count),
            tool_count: tool_total + surface.counted.len().saturating_sub(counted_enabled),
            extensions: surface.extensions.clone(),
            frontend_instructions: render_frontend_instructions(&surface.frontend_instructions),
        }
    }

    /// The tools to send, plus the delta bringing the model's view in line with what is enabled.
    pub async fn resolve(
        &self,
        session_id: &str,
        enabled_tools: &[Tool],
        conversation: &Conversation,
    ) -> (Vec<Tool>, Option<ToolSetUpdateContent>) {
        let mut sessions = self.0.lock().await;
        let surface = sessions.entry(session_id.to_string()).or_default();
        let declared_names = surface.declare_tools(enabled_tools);

        // An addition archived by compaction simply has to be made again.
        let offered: HashSet<String> = declared_names
            .difference(&surface.deferred)
            .cloned()
            .collect();
        let visible = resolve_visible_tools(&declared_names, &offered, conversation.messages());
        let enabled_names: HashSet<String> = enabled_tools
            .iter()
            .map(|tool| tool.name.to_string())
            .collect();

        let mut update = ToolSetUpdateContent {
            added: enabled_names.difference(&visible).cloned().collect(),
            removed: visible.difference(&enabled_names).cloned().collect(),
        };
        update.added.sort();
        update.removed.sort();

        (
            surface.declared_tools(),
            (!update.is_empty()).then_some(update),
        )
    }
}

fn render_frontend_instructions(instructions: &[(String, String)]) -> Option<String> {
    match instructions {
        [] => None,
        [(_, instructions)] => Some(instructions.clone()),
        instructions => Some(
            instructions
                .iter()
                .map(|(name, instructions)| format!("{name}: {instructions}"))
                .collect::<Vec<_>>()
                .join("\n\n"),
        ),
    }
}

impl Surface {
    fn declared_tools(&self) -> Vec<Tool> {
        // A marker on an enabled tool is not goose's, and would withhold it for good.
        self.tools
            .iter()
            .map(|tool| {
                if self.deferred.contains(tool.name.as_ref()) {
                    defer_tool_loading(tool)
                } else {
                    offer_tool_immediately(tool)
                }
            })
            .collect()
    }

    fn declare_tools(&mut self, enabled: &[Tool]) -> HashSet<String> {
        // Whichever of `declare` and `resolve` runs first for a turn lands here, so this is the
        // only place that can tell a never-offered tool from an offered one. The opening array is
        // offered outright because the API rejects one whose every tool is withheld.
        let opening = self.tools.is_empty();
        // A changed schema can arrive under an unchanged name, and a stale one means wrong
        // arguments.
        for tool in enabled {
            match self
                .tools
                .iter_mut()
                .find(|declared| declared.name == tool.name)
            {
                Some(declared) => *declared = tool.clone(),
                None => {
                    if !opening {
                        self.deferred.insert(tool.name.to_string());
                    }
                    self.tools.push(tool.clone());
                }
            }
        }
        // Order must not depend on when a tool was declared, so a resume rebuilds it byte-wise.
        self.tools.sort_by(|a, b| a.name.cmp(&b.name));
        self.tools.iter().map(|t| t.name.to_string()).collect()
    }
}

/// A carrier only holds a delta at a wire position, so nothing else may be injected into it:
/// that would render as the message's only block, which the Anthropic formatter cannot
/// relocate out of the cached prefix once later messages exist.
pub fn is_tool_set_update_carrier(message: &Message) -> bool {
    !message.content.is_empty()
        && message
            .content
            .iter()
            .all(|content| matches!(content, MessageContentBlock::ToolSetUpdate(_)))
}

/// A delta renders as a wire-level system message, which must follow a user turn.
pub fn can_record_tool_set_update(conversation: &Conversation) -> bool {
    conversation
        .messages()
        .last()
        .is_some_and(|message| message.role == rmcp::model::Role::User)
}

/// Deltas only mean anything against the declared surface, so a caller building its own tool
/// array must drop them or an earlier removal hides a tool it just supplied.
pub fn without_tool_set_updates(conversation: &Conversation) -> Conversation {
    Conversation::new_unvalidated(
        conversation
            .messages()
            .iter()
            .filter_map(|message| {
                let mut message = message.clone();
                message
                    .content
                    .retain(|content| !matches!(content, MessageContentBlock::ToolSetUpdate(_)));
                (!message.content.is_empty()).then_some(message)
            })
            .collect::<Vec<_>>(),
    )
}

pub fn tool_set_update_message(update: ToolSetUpdateContent) -> Message {
    Message::user()
        .with_content(MessageContentBlock::ToolSetUpdate(update))
        .with_metadata(MessageMetadata::agent_only())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::formats::anthropic::tool_defers_loading;
    use rmcp::object;

    fn tools(names: &[&str]) -> Vec<Tool> {
        names
            .iter()
            .map(|name| Tool::new(name.to_string(), "test tool", object!({"type": "object"})))
            .collect()
    }

    fn names(tools: &[Tool]) -> Vec<String> {
        tools.iter().map(|tool| tool.name.to_string()).collect()
    }

    #[tokio::test]
    async fn test_disable_then_reenable_replays_as_deltas() {
        let surfaces = DeclaredSurfaces::default();
        let mut conversation = Conversation::new_unvalidated(vec![Message::user().with_text("hi")]);

        let all = tools(&["b", "a", "c"]);
        let (declared, update) = surfaces.resolve("s", &all, &conversation).await;
        assert_eq!(names(&declared), ["a", "b", "c"]);
        assert!(update.is_none());

        let remaining = tools(&["a"]);
        let (declared, update) = surfaces.resolve("s", &remaining, &conversation).await;
        assert_eq!(names(&declared), ["a", "b", "c"]);
        let update = update.unwrap();
        assert_eq!(update.removed, ["b", "c"]);
        assert!(update.added.is_empty());
        conversation.push(tool_set_update_message(update));

        let (_, update) = surfaces.resolve("s", &remaining, &conversation).await;
        assert!(update.is_none());

        let (declared, update) = surfaces.resolve("s", &all, &conversation).await;
        assert_eq!(names(&declared), ["a", "b", "c"]);
        let update = update.unwrap();
        assert_eq!(update.added, ["b", "c"]);
        assert!(update.removed.is_empty());
    }

    #[tokio::test]
    async fn test_tool_enabled_mid_session_is_withheld_until_announced() {
        let surfaces = DeclaredSurfaces::default();
        let conversation = Conversation::new_unvalidated(vec![Message::user().with_text("hi")]);

        surfaces.resolve("s", &tools(&["a"]), &conversation).await;
        let (declared, update) = surfaces
            .resolve("s", &tools(&["a", "b"]), &conversation)
            .await;

        assert_eq!(update.unwrap().added, ["b"]);
        let withheld: Vec<&str> = declared
            .iter()
            .filter(|tool| tool_defers_loading(tool))
            .map(|tool| tool.name.as_ref())
            .collect();
        // The API rejects an array whose every tool is withheld, so the opening set stays offered.
        assert_eq!(withheld, ["b"]);
    }

    #[tokio::test]
    async fn test_extension_cannot_withhold_its_own_tool() {
        let surfaces = DeclaredSurfaces::default();
        let conversation = Conversation::new_unvalidated(vec![Message::user().with_text("hi")]);
        let spoofed = tools(&["a"])
            .iter()
            .map(defer_tool_loading)
            .collect::<Vec<_>>();

        let (declared, update) = surfaces.resolve("s", &spoofed, &conversation).await;

        assert!(update.is_none());
        assert!(!tool_defers_loading(&declared[0]));
    }

    #[tokio::test]
    async fn test_archived_delta_is_replaced() {
        let surfaces = DeclaredSurfaces::default();
        let conversation = Conversation::new_unvalidated(vec![
            Message::user().with_text("hi"),
            tool_set_update_message(ToolSetUpdateContent {
                added: Vec::new(),
                removed: vec!["b".to_string()],
            })
            .with_metadata(MessageMetadata::user_only()),
        ]);

        surfaces
            .resolve("s", &tools(&["a", "b"]), &conversation)
            .await;
        let (_, update) = surfaces.resolve("s", &tools(&["a"]), &conversation).await;

        assert_eq!(update.unwrap().removed, ["b"]);
    }

    #[tokio::test]
    async fn test_declared_prompt_keeps_what_was_disabled() {
        let surfaces = DeclaredSurfaces::default();
        let apps = ExtensionInfo::new("apps", "build apps", false);
        let developer = ExtensionInfo::new("developer", "write code", false);

        // `apps__hidden` is counted but never offered, as an app-only tool is.
        let prompt = surfaces
            .declare(
                "s",
                &tools(&["apps__a", "developer__b"]),
                EnabledSurface {
                    counted_tool_names: names(&tools(&["apps__a", "apps__hidden", "developer__b"])),
                    extensions: vec![apps, developer.clone()],
                    frontend_instructions: vec![
                        ("ui".to_string(), "render things".to_string()),
                        ("chat".to_string(), "talk".to_string()),
                    ],
                    extension_total: 4,
                    tool_total: 3,
                },
            )
            .await;
        assert_eq!(
            prompt.frontend_instructions.as_deref(),
            Some("chat: talk\n\nui: render things")
        );
        assert_eq!((prompt.extension_count, prompt.tool_count), (4, 3));

        let prompt = surfaces
            .declare(
                "s",
                &tools(&["developer__b"]),
                EnabledSurface {
                    counted_tool_names: names(&tools(&["developer__b"])),
                    extensions: vec![developer],
                    frontend_instructions: Vec::new(),
                    extension_total: 1,
                    tool_total: 1,
                },
            )
            .await;
        assert_eq!(
            prompt
                .extensions
                .iter()
                .map(|extension| extension.name.as_str())
                .collect::<Vec<_>>(),
            ["apps", "developer"]
        );
        assert_eq!(
            prompt.frontend_instructions.as_deref(),
            Some("chat: talk\n\nui: render things")
        );
        assert_eq!((prompt.extension_count, prompt.tool_count), (4, 3));

        let prompt = surfaces
            .declare(
                "s",
                &tools(&["other__c"]),
                EnabledSurface {
                    counted_tool_names: names(&tools(&["other__c"])),
                    extensions: vec![ExtensionInfo::new("other", "do other things", false)],
                    frontend_instructions: Vec::new(),
                    extension_total: 1,
                    tool_total: 1,
                },
            )
            .await;

        assert_eq!((prompt.extension_count, prompt.tool_count), (5, 4));
    }
}
