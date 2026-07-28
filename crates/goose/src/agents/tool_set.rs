use std::collections::{HashMap, HashSet};

use rmcp::model::Tool;
use tokio::sync::Mutex;

use crate::agents::extension::ExtensionInfo;
use crate::conversation::message::{
    Message, MessageContentBlock, MessageMetadata, ToolSetUpdateContent,
};
use crate::conversation::{resolve_visible_tools, Conversation};
use crate::providers::formats::anthropic::defer_tool_loading;

/// Everything goose has offered the model during a session, per session.
///
/// Anthropic hashes a request prefix as `tools` -> `system` -> `messages`, so the cache only
/// survives if both stay byte-identical for the life of the conversation. Disabling an
/// extension therefore leaves its tools declared and hides them with a delta instead.
///
/// A cache, not a source of truth: it is rebuilt from what is enabled on session resume.
#[derive(Default)]
pub struct DeclaredSurfaces(Mutex<HashMap<String, Surface>>);

#[derive(Default)]
struct Surface {
    tools: Vec<Tool>,
    /// Names withheld in the array until an addition reveals them. An addition only carries a
    /// reference, so a tool the array never held still costs one rewrite of it; withholding is
    /// what stops the model reading that tool as available before it was enabled.
    deferred: HashSet<String>,
    /// Every tool the session has held, counted before the filters that decide what the model is
    /// offered. `tool_count` is reported in those unfiltered terms, so an adjustment measured
    /// against `tools` would shrink whenever a hidden tool's extension went away.
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

impl DeclaredSurfaces {
    pub async fn declare(
        &self,
        session_id: &str,
        enabled_tools: &[Tool],
        counted_tool_names: Vec<String>,
        enabled_extensions: Vec<ExtensionInfo>,
        enabled_frontend_instructions: Vec<(String, String)>,
        enabled_counts: (usize, usize),
    ) -> DeclaredPrompt {
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

        // `enabled_counts` covers tool sources the declared prompt lists do not hold, so add
        // what the surface declares beyond the currently enabled set rather than recounting.
        DeclaredPrompt {
            extension_count: enabled_counts.0
                + surface
                    .extensions
                    .len()
                    .saturating_sub(enabled_extension_count)
                + surface
                    .frontend_instructions
                    .len()
                    .saturating_sub(enabled_frontend_extension_count),
            tool_count: enabled_counts.1 + surface.counted.len().saturating_sub(counted_enabled),
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

        // What the array offers by itself. A withheld tool is declared but unavailable until an
        // addition names it, so the model's view is this set with the recorded deltas replayed:
        // one archived by compaction simply means the addition has to be made again.
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
        self.tools
            .iter()
            .map(|tool| {
                if self.deferred.contains(tool.name.as_ref()) {
                    defer_tool_loading(tool)
                } else {
                    tool.clone()
                }
            })
            .collect()
    }

    fn declare_tools(&mut self, enabled: &[Tool]) -> HashSet<String> {
        // Whichever of `declare` and `resolve` runs first for a turn lands here, so this is the
        // only place that can tell a tool the model has never been offered from one it has. The
        // first non-empty array has to be offered outright: the API rejects one whose every tool
        // is withheld, so a session that opens with nothing model-visible cannot withhold what it
        // gains later.
        let opening = self.tools.is_empty();
        // A changed schema can arrive under an unchanged name, and keeping the old one would
        // have the model calling it with stale arguments.
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
    async fn test_declared_prompt_keeps_disabled_extensions() {
        let surfaces = DeclaredSurfaces::default();
        let apps = ExtensionInfo::new("apps", "build apps", false);
        let developer = ExtensionInfo::new("developer", "write code", false);

        // `apps__hidden` is counted but never offered, as an app-only tool is.
        surfaces
            .declare(
                "s",
                &tools(&["apps__a", "developer__b"]),
                names(&tools(&["apps__a", "apps__hidden", "developer__b"])),
                vec![apps, developer.clone()],
                Vec::new(),
                (2, 3),
            )
            .await;
        let prompt = surfaces
            .declare(
                "s",
                &tools(&["developer__b"]),
                names(&tools(&["developer__b"])),
                vec![developer],
                Vec::new(),
                (1, 1),
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
        assert_eq!((prompt.extension_count, prompt.tool_count), (2, 3));

        let other = ExtensionInfo::new("other", "do other things", false);
        let prompt = surfaces
            .declare(
                "s",
                &tools(&["other__c"]),
                names(&tools(&["other__c"])),
                vec![other],
                Vec::new(),
                (1, 1),
            )
            .await;

        assert_eq!((prompt.extension_count, prompt.tool_count), (3, 4));
    }

    #[tokio::test]
    async fn test_declared_prompt_keeps_disabled_frontend_instructions() {
        let surfaces = DeclaredSurfaces::default();

        let prompt = surfaces
            .declare(
                "s",
                &tools(&["apps__a", "developer__b"]),
                names(&tools(&["apps__a", "developer__b"])),
                Vec::new(),
                vec![
                    ("developer".to_string(), "write code".to_string()),
                    ("apps".to_string(), "build apps".to_string()),
                ],
                (2, 2),
            )
            .await;
        assert_eq!(
            prompt.frontend_instructions.as_deref(),
            Some("apps: build apps\n\ndeveloper: write code")
        );

        let prompt = surfaces
            .declare(
                "s",
                &tools(&["developer__b"]),
                names(&tools(&["developer__b"])),
                Vec::new(),
                vec![("developer".to_string(), "write code".to_string())],
                (1, 1),
            )
            .await;

        assert_eq!(
            prompt.frontend_instructions.as_deref(),
            Some("apps: build apps\n\ndeveloper: write code")
        );
        assert_eq!((prompt.extension_count, prompt.tool_count), (2, 2));
    }
}
