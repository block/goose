/// Heuristic used by the opt-in soft-continue path: after tools ran in a user
/// turn, a text-only reply that still looks unfinished gets one automatic nudge
/// instead of ending the agent loop.

const UNFINISHED_PREFIXES: &[&str] = &[
    "now i'll",
    "now i will",
    "now let me",
    "let me",
    "i'll",
    "i will",
    "next i'll",
    "next i will",
    "next,",
    "next:",
    "continuing",
    "continue —",
    "continue -",
    "i'm going to",
    "i am going to",
    "first i'll",
    "first i will",
    "then i'll",
    "then i will",
];

const TERMINAL_SHORT_REPLIES: &[&str] = &[
    "done",
    "done.",
    "fixed",
    "fixed.",
    "ok",
    "ok.",
    "okay",
    "okay.",
    "all done",
    "all done.",
];

/// Returns true when `text` looks like a mid-task pause rather than a finished reply.
pub fn looks_unfinished(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return false;
    }

    let lowercase = trimmed.to_lowercase();
    if TERMINAL_SHORT_REPLIES
        .iter()
        .any(|reply| lowercase == *reply)
    {
        return false;
    }

    if trimmed.ends_with(':') || trimmed.ends_with('…') || trimmed.ends_with("...") {
        return true;
    }

    if has_unclosed_fence(trimmed) {
        return true;
    }

    let first_line = lowercase.lines().next().unwrap_or("").trim();
    if UNFINISHED_PREFIXES
        .iter()
        .any(|prefix| first_line.starts_with(prefix))
    {
        return true;
    }

    // Very short non-terminal replies after tools are usually status, not completion.
    if trimmed.chars().count() < 80 && !trimmed.contains('?') && !looks_terminal_summary(&lowercase)
    {
        return true;
    }

    false
}

fn has_unclosed_fence(text: &str) -> bool {
    text.matches("```").count() % 2 == 1
}

fn looks_terminal_summary(lowercase: &str) -> bool {
    lowercase.starts_with("done")
        || lowercase.starts_with("fixed")
        || lowercase.starts_with("here's")
        || lowercase.starts_with("here is")
        || lowercase.starts_with("summary")
        || lowercase.contains("all set")
        || lowercase.contains("you're all set")
}

pub const SOFT_CONTINUE_MESSAGE: &str = "\
If work remains, continue with tools now. \
If you are fully done or need input from the user, reply with a clear final message and no tool calls.";

pub const CONTINUE_AFTER_TOOLS_CONFIG_KEY: &str = "GOOSE_CONTINUE_AFTER_TOOLS";

pub fn continue_after_tools_enabled() -> bool {
    crate::config::Config::global()
        .get_param::<bool>(CONTINUE_AFTER_TOOLS_CONFIG_KEY)
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_trailing_colon_and_ellipsis() {
        assert!(looks_unfinished("Now I'll wire the CSS:"));
        assert!(looks_unfinished("Next up…"));
        assert!(looks_unfinished("Working on it..."));
    }

    #[test]
    fn detects_unfinished_prefixes() {
        assert!(looks_unfinished("Let me check the scroll area next."));
        assert!(looks_unfinished("Continuing — wiring the fade prop."));
        assert!(looks_unfinished("I'll update BaseChat."));
    }

    #[test]
    fn detects_unclosed_code_fence() {
        assert!(looks_unfinished(
            "Here is the change:\n```rust\nfn main() {}"
        ));
        assert!(!looks_unfinished(
            "Here is the change:\n```rust\nfn main() {}\n```"
        ));
    }

    #[test]
    fn short_status_is_unfinished_but_done_is_not() {
        assert!(looks_unfinished("On it."));
        assert!(!looks_unfinished("Done."));
        assert!(!looks_unfinished("Fixed."));
        assert!(!looks_unfinished(
            "Done. The Anthropic default is now `claude-opus-5`."
        ));
    }

    #[test]
    fn complete_answers_are_finished() {
        assert!(!looks_unfinished(
            "Here's what's going on — nothing was lost, the work just isn't on main yet."
        ));
        assert!(!looks_unfinished(
            "I checked the config and the default model is already set correctly."
        ));
        assert!(!looks_unfinished("Want me to open a PR for this?"));
    }

    #[test]
    fn empty_is_not_unfinished() {
        assert!(!looks_unfinished(""));
        assert!(!looks_unfinished("   "));
    }
}
