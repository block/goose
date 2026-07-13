//! Pure, testable formatting helpers for CLI message rendering.
//!
//! These functions build the *text* of what gets printed (indentation,
//! glyphs, structure) without performing any I/O, so the visual shape of
//! CLI output can be unit tested directly. `output.rs` calls into these
//! helpers and applies `console` styling (via [`role_style`]/[`apply`])
//! before writing to stdout.
//!
//! The palette here intentionally mirrors the Ink TUI (`ui/text`): a
//! restrained, mostly dim/grey scheme with a single accent color, plus
//! green/red reserved for outcomes.

use console::{style, Color, StyledObject};

/// Left margin used for top-level chrome: user prompt, tool headers,
/// status lines, and error messages. Matches the visual width of the
/// user-message prompt glyph (`USER_PROMPT_GLYPH` + a space) so everything
/// lines up in a single gutter.
pub const GUTTER: &str = "  ";

/// Indentation used for content nested under a tool header (parameters,
/// tool output).
pub const PARAM_INDENT: &str = "    ";

/// Glyph used to mark a user-submitted message.
pub const USER_PROMPT_GLYPH: &str = "❯";

/// Lifecycle state of a tool call, used to pick a status glyph/color.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToolStatus {
    Running,
    Success,
    Error,
}

/// Semantic roles used to keep colors consistent across the CLI, rather
/// than picking a color ad hoc at each call site.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Primary conversational text (assistant/user body copy).
    Primary,
    /// Secondary/structural text: labels, tool names, headers. Bold but
    /// colorless, so it stands out from `Muted` by weight rather than hue,
    /// keeping the palette mostly grey.
    Secondary,
    /// Muted/auxiliary text: parameter values, timestamps, hints.
    Muted,
    /// The single accent color used for the user prompt glyph and pending
    /// state. Kept distinct from every other role's color, since it's the
    /// only splash of hue in an otherwise grey/dim palette.
    Accent,
    /// Successful/completed outcomes.
    Success,
    /// Errors and failures.
    Error,
}

/// The concrete color/weight for a [`Role`]. Kept as plain data (rather
/// than a `console::StyledObject`) so it can be compared in tests without
/// depending on ANSI rendering or terminal color support.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RoleStyle {
    pub color: Option<Color>,
    pub dim: bool,
    pub bold: bool,
    pub italic: bool,
}

/// The color/weight to use for a given semantic role.
pub fn role_style(role: Role) -> RoleStyle {
    match role {
        Role::Primary => RoleStyle {
            color: None,
            dim: false,
            bold: false,
            italic: false,
        },
        Role::Secondary => RoleStyle {
            color: None,
            dim: false,
            bold: true,
            italic: false,
        },
        Role::Muted => RoleStyle {
            color: None,
            dim: true,
            bold: false,
            italic: false,
        },
        Role::Accent => RoleStyle {
            color: Some(Color::Cyan),
            dim: false,
            bold: true,
            italic: false,
        },
        Role::Success => RoleStyle {
            color: Some(Color::Green),
            dim: false,
            bold: false,
            italic: false,
        },
        Role::Error => RoleStyle {
            color: Some(Color::Red),
            dim: false,
            bold: false,
            italic: false,
        },
    }
}

/// Apply a [`Role`]'s style to a piece of text, returning a `console`
/// styled object ready to print.
pub fn apply(role: Role, text: &str) -> StyledObject<&str> {
    let RoleStyle {
        color,
        dim,
        bold,
        italic,
    } = role_style(role);
    let mut styled = style(text);
    if let Some(color) = color {
        styled = styled.fg(color);
    }
    if dim {
        styled = styled.dim();
    }
    if bold {
        styled = styled.bold();
    }
    if italic {
        styled = styled.italic();
    }
    styled
}

/// The glyph shown next to a tool call to indicate its status.
pub fn status_glyph(status: ToolStatus) -> &'static str {
    match status {
        ToolStatus::Running => "○",
        ToolStatus::Success => "●",
        ToolStatus::Error => "✗",
    }
}

/// The semantic role used to color a status glyph.
pub fn status_role(status: ToolStatus) -> Role {
    match status {
        ToolStatus::Running => Role::Accent,
        ToolStatus::Success => Role::Success,
        ToolStatus::Error => Role::Error,
    }
}

/// Format a user-submitted message for echo into the transcript: the
/// first line is marked with the accent prompt glyph, continuation lines
/// align under it using [`GUTTER`]. Always includes the glyph, even for
/// an empty message, so a user turn is never rendered as nothing at all.
pub fn format_user_message_plain(text: &str) -> String {
    let mut lines = text.lines();
    let mut out = String::from(USER_PROMPT_GLYPH);
    if let Some(first) = lines.next() {
        out.push(' ');
        out.push_str(first);
    }
    for line in lines {
        out.push('\n');
        out.push_str(GUTTER);
        out.push_str(line);
    }
    out
}

/// Format the plain-text (no ANSI) tool-call header line: gutter + status
/// glyph + tool name + optional extension name.
pub fn format_tool_header_plain(tool: &str, extension: &str, status: ToolStatus) -> String {
    let glyph = status_glyph(status);
    if extension.is_empty() {
        format!("{GUTTER}{glyph} {tool}")
    } else {
        format!("{GUTTER}{glyph} {tool} {extension}")
    }
}

/// Format the plain-text tool-call status footer shown once a tool call
/// has finished (success or error). Returns `None` when there is nothing
/// to show (e.g. while still running).
pub fn format_tool_status_line_plain(status: ToolStatus) -> Option<String> {
    match status {
        ToolStatus::Running => None,
        ToolStatus::Success => Some(format!("{GUTTER}{} done", status_glyph(status))),
        ToolStatus::Error => Some(format!("{GUTTER}{} error", status_glyph(status))),
    }
}

/// Indent every non-empty line of a rendered block (e.g. an assistant
/// reply's already-highlighted markdown) under the shared [`GUTTER`], so
/// normal model output aligns with the user-message prompt glyph instead
/// of sitting flush-left. Blank lines are left empty rather than padded
/// with trailing whitespace, and the exact number of lines (including a
/// trailing blank one from a trailing newline) is preserved.
pub fn indent_block(text: &str) -> String {
    text.split('\n')
        .map(|line| {
            if line.is_empty() {
                String::new()
            } else {
                format!("{GUTTER}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Format the plain-text error line shown for CLI-level errors.
pub fn format_error_line_plain(message: &str) -> String {
    format!(
        "{GUTTER}{} error: {message}",
        status_glyph(ToolStatus::Error)
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_message_is_echoed_with_accent_prompt_glyph() {
        assert_eq!(
            format_user_message_plain("hello"),
            format!("{USER_PROMPT_GLYPH} hello")
        );
    }

    #[test]
    fn empty_user_message_still_renders_the_prompt_glyph() {
        assert_eq!(format_user_message_plain(""), USER_PROMPT_GLYPH);
    }

    #[test]
    fn multiline_user_message_continuation_lines_align_under_the_gutter() {
        assert_eq!(
            format_user_message_plain("first\nsecond\nthird"),
            format!("{USER_PROMPT_GLYPH} first\n{GUTTER}second\n{GUTTER}third")
        );
    }

    #[test]
    fn running_tool_header_uses_the_hollow_circle_glyph() {
        let header = format_tool_header_plain("shell", "", ToolStatus::Running);
        assert!(
            header.contains('○'),
            "running tool header should use the ○ glyph, got: {header:?}"
        );
    }

    #[test]
    fn tool_header_has_no_heavy_horizontal_rule() {
        let header = format_tool_header_plain("shell", "", ToolStatus::Running);
        assert!(
            !header.contains('─'),
            "tool header should not draw a full-width rule, got: {header:?}"
        );
    }

    #[test]
    fn tool_header_is_a_single_line_starting_with_the_gutter() {
        let header = format_tool_header_plain("shell", "developer", ToolStatus::Running);
        assert_eq!(header.lines().count(), 1);
        assert!(header.starts_with(GUTTER));
        assert!(header.contains("shell"));
        assert!(header.contains("developer"));
    }

    #[test]
    fn tool_status_glyph_differs_by_outcome() {
        let running = status_glyph(ToolStatus::Running);
        let success = status_glyph(ToolStatus::Success);
        let error = status_glyph(ToolStatus::Error);
        assert_ne!(running, success);
        assert_ne!(success, error);
        assert_ne!(running, error);
    }

    #[test]
    fn successful_tool_calls_get_a_status_footer() {
        assert_eq!(
            format_tool_status_line_plain(ToolStatus::Success),
            Some(format!("{GUTTER}● done"))
        );
    }

    #[test]
    fn failed_tool_calls_get_a_status_footer_with_the_error_glyph() {
        assert_eq!(
            format_tool_status_line_plain(ToolStatus::Error),
            Some(format!("{GUTTER}✗ error"))
        );
    }

    #[test]
    fn running_tool_calls_get_no_status_footer() {
        assert_eq!(format_tool_status_line_plain(ToolStatus::Running), None);
    }

    #[test]
    fn error_lines_are_indented_under_the_gutter_with_a_glyph() {
        let line = format_error_line_plain("boom");
        assert!(line.starts_with(GUTTER));
        assert!(line.contains(status_glyph(ToolStatus::Error)));
        assert!(line.contains("boom"));
    }

    #[test]
    fn pending_tool_status_uses_the_accent_role() {
        assert_eq!(status_role(ToolStatus::Running), Role::Accent);
    }

    #[test]
    fn accent_role_has_a_distinct_color_set() {
        assert!(
            role_style(Role::Accent).color.is_some(),
            "accent role should have an explicit color, not fall back to the default"
        );
    }

    #[test]
    fn success_and_error_roles_map_to_green_and_red() {
        assert_eq!(role_style(Role::Success).color, Some(Color::Green));
        assert_eq!(role_style(Role::Error).color, Some(Color::Red));
    }

    #[test]
    fn muted_role_is_dim_and_uses_no_extra_color() {
        let muted = role_style(Role::Muted);
        assert!(muted.dim);
        assert_eq!(muted.color, None);
    }

    #[test]
    fn secondary_role_is_distinct_from_muted() {
        assert_ne!(role_style(Role::Secondary), role_style(Role::Muted));
    }

    #[test]
    fn secondary_role_is_colorless_so_accent_stays_the_only_hue() {
        assert_eq!(
            role_style(Role::Secondary).color,
            None,
            "Secondary should stand out from Muted by weight (bold), not by \
             borrowing Accent's color, to keep the palette mostly grey"
        );
    }

    #[test]
    fn apply_is_a_no_op_on_plain_text_when_colors_are_disabled() {
        // `console`'s color setting is process-global, so save/restore it
        // rather than leaking `false` into other tests that run in the
        // same process.
        let was_enabled = console::colors_enabled();
        console::set_colors_enabled(false);
        let result = std::panic::catch_unwind(|| {
            assert_eq!(apply(Role::Error, "boom").to_string(), "boom");
            assert_eq!(apply(Role::Primary, "hello").to_string(), "hello");
        });
        console::set_colors_enabled(was_enabled);
        result.unwrap();
    }

    #[test]
    fn param_indent_is_wider_than_the_gutter() {
        assert!(PARAM_INDENT.len() > GUTTER.len());
    }

    #[test]
    fn normal_model_messages_are_indented_under_the_gutter() {
        assert_eq!(
            indent_block("Sure, here's the answer."),
            format!("{GUTTER}Sure, here's the answer.")
        );
    }

    #[test]
    fn multiline_model_messages_have_every_line_indented_under_the_gutter() {
        assert_eq!(
            indent_block("first line\nsecond line\nthird line"),
            format!("{GUTTER}first line\n{GUTTER}second line\n{GUTTER}third line")
        );
    }

    #[test]
    fn blank_lines_in_a_model_message_are_not_padded_with_trailing_whitespace() {
        assert_eq!(
            indent_block("first paragraph\n\nsecond paragraph"),
            format!("{GUTTER}first paragraph\n\n{GUTTER}second paragraph")
        );
    }

    #[test]
    fn indent_block_preserves_a_trailing_newline() {
        assert_eq!(indent_block("hello\n"), format!("{GUTTER}hello\n"));
    }

    #[test]
    fn indent_block_is_a_no_op_on_empty_input() {
        assert_eq!(indent_block(""), "");
    }
}
