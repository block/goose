use std::borrow::Cow;
use std::path::PathBuf;

use nu_ansi_term::Style;
use reedline::{
    ColumnarMenu, Completer, EditCommand, Emacs, FileBackedHistory, Hinter, History, KeyCode,
    KeyModifiers, MenuBuilder, Prompt, PromptEditMode, PromptHistorySearch,
    PromptHistorySearchStatus, Reedline, ReedlineEvent, ReedlineMenu, Signal, Span, Suggestion,
};

use crate::commands::CommandDef;
use crate::context;
use crate::display::style;
use crate::slash;

pub enum InputEvent {
    Line(String),
    CtrlC,
    CtrlD,
}

pub struct GoosePrompt {
    pub cwd: String,
    pub git_branch: Option<String>,
    pub context_percent: Option<u8>,
    pub session_id: Option<String>,
    pub model_name: Option<String>,
}

impl Default for GoosePrompt {
    fn default() -> Self {
        Self::new()
    }
}

struct PromptSegment {
    label: &'static str,
    plain: String,
    styled: String,
}

impl PromptSegment {
    fn new(label: &'static str, text: &str, style_fn: impl Fn(&str) -> String) -> Self {
        let clean = crate::display::sanitize_control_chars(text);
        Self {
            label,
            styled: style_fn(&clean),
            plain: clean,
        }
    }

    fn truncate(&mut self, excess: usize, from_left: bool, style_fn: impl Fn(&str) -> String) {
        let chars = self.plain.chars().count();
        let truncated = if chars > excess + 1 {
            let keep = chars - excess - 1;
            if from_left {
                std::iter::once('…')
                    .chain(self.plain.chars().skip(chars - keep))
                    .collect()
            } else {
                self.plain
                    .chars()
                    .take(keep)
                    .chain(std::iter::once('…'))
                    .collect()
            }
        } else {
            "…".to_string()
        };
        self.styled = style_fn(&truncated);
        self.plain = truncated;
    }
}

impl GoosePrompt {
    pub fn new() -> Self {
        Self {
            cwd: context::format_cwd(),
            git_branch: context::git_branch(),
            context_percent: None,
            session_id: None,
            model_name: None,
        }
    }

    pub fn refresh(&mut self) {
        self.cwd = context::format_cwd();
        self.git_branch = context::git_branch();
    }

    pub fn update_context(&mut self, percent: u8) {
        self.context_percent = Some(percent);
    }

    fn context_color(pct: u8) -> String {
        match pct {
            0..70 => style::prompt_context_ok(),
            70..90 => style::prompt_context_warn(),
            _ => style::prompt_context_crit(),
        }
    }

    /// Build the info line string (cwd, branch, context%, session, model).
    /// Returns the styled string ready for display. Does NOT include a trailing newline.
    fn build_info_string(&self) -> String {
        let term_width = crate::display::term_size().0 as usize;
        let sep_plain = " | ";
        let sep_styled = format!(" {}|{} ", style::prompt_separator(), style::reset());

        let mut segments: Vec<PromptSegment> = Vec::new();

        segments.push(PromptSegment::new("cwd", &self.cwd, |s| {
            format!("{}{}{}", style::prompt_cwd(), s, style::reset())
        }));

        if let Some(ref branch) = self.git_branch {
            segments.push(PromptSegment::new("branch", branch, |s| {
                format!("{}{}{}", style::prompt_branch(), s, style::reset())
            }));
        }

        if let Some(pct) = self.context_percent {
            let pct_text = format!("{pct}%");
            segments.push(PromptSegment::new("context", &pct_text, |s| {
                format!("{}{}{}", Self::context_color(pct), s, style::reset())
            }));
        }

        if let Some(ref sid) = self.session_id {
            segments.push(PromptSegment::new("session", sid, |s| {
                format!("{}{}{}", style::prompt_session_id(), s, style::reset())
            }));
        }

        if let Some(ref model) = self.model_name {
            segments.push(PromptSegment::new("model", model, |s| {
                format!("{}{}{}", style::prompt_model(), s, style::reset())
            }));
        }

        let visible_width = |segs: &[PromptSegment]| -> usize {
            segs.iter().map(|s| s.plain.chars().count()).sum::<usize>()
                + if segs.len() > 1 {
                    sep_plain.len() * (segs.len() - 1)
                } else {
                    0
                }
        };

        // Truncate CWD first (left-truncate: keep rightmost path portion).
        let mut total = visible_width(&segments);
        if total > term_width {
            if let Some(seg) = segments.iter_mut().find(|s| s.label == "cwd") {
                seg.truncate(total - term_width, true, |s| {
                    format!("{}{}{}", style::prompt_cwd(), s, style::reset())
                });
            }
        }

        // If still too wide, truncate model name (right-truncate).
        total = visible_width(&segments);
        if total > term_width {
            if let Some(seg) = segments.iter_mut().find(|s| s.label == "model") {
                seg.truncate(total - term_width, false, |s| {
                    format!("{}{}{}", style::prompt_model(), s, style::reset())
                });
            }
        }

        segments
            .iter()
            .map(|s| s.styled.as_str())
            .collect::<Vec<_>>()
            .join(&sep_styled)
    }
}

impl Prompt for GoosePrompt {
    fn render_prompt_left(&self) -> Cow<'_, str> {
        let info = self.build_info_string();
        if info.is_empty() {
            Cow::Owned(format!("{}>{} ", style::prompt_indicator(), style::reset()))
        } else {
            Cow::Owned(format!(
                "{info}\n{}>{} ",
                style::prompt_indicator(),
                style::reset()
            ))
        }
    }

    fn render_prompt_right(&self) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_indicator(&self, _mode: PromptEditMode) -> Cow<'_, str> {
        Cow::Borrowed("")
    }

    fn render_prompt_multiline_indicator(&self) -> Cow<'_, str> {
        Cow::Borrowed("  ")
    }

    fn render_prompt_history_search_indicator(
        &self,
        history_search: PromptHistorySearch,
    ) -> Cow<'_, str> {
        let prefix = match history_search.status {
            PromptHistorySearchStatus::Passing => "",
            PromptHistorySearchStatus::Failing => "failing ",
        };
        Cow::Owned(format!(
            "({}reverse-search: {}) ",
            prefix, history_search.term
        ))
    }
}

pub struct GooseHinter {
    commands: Vec<String>,
    style: Style,
    current_hint: String,
}

impl GooseHinter {
    pub fn new(commands: Vec<String>, style: Style) -> Self {
        Self {
            commands,
            style,
            current_hint: String::new(),
        }
    }

    fn complete_from<S: AsRef<str>>(partial: &str, candidates: &[S]) -> String {
        candidates
            .iter()
            .map(|c| c.as_ref())
            .find(|c| c.starts_with(partial) && c.len() > partial.len())
            .and_then(|c| c.get(partial.len()..))
            .unwrap_or_default()
            .to_string()
    }

    fn paint(&self, hint: &str, use_ansi_coloring: bool) -> String {
        if use_ansi_coloring && !hint.is_empty() {
            self.style.paint(hint).to_string()
        } else {
            hint.to_string()
        }
    }

    fn build_overview(&self, available_width: usize) -> String {
        let sep = " | ";
        let mut result = String::new();
        for (i, cmd) in self.commands.iter().enumerate() {
            let candidate = if i == 0 {
                cmd.clone()
            } else {
                format!("{sep}{cmd}")
            };
            if result.chars().count() + candidate.chars().count() > available_width {
                break;
            }
            result.push_str(&candidate);
        }
        result
    }
}

impl Hinter for GooseHinter {
    fn handle(
        &mut self,
        line: &str,
        _pos: usize,
        _history: &dyn History,
        use_ansi_coloring: bool,
        _cwd: &str,
    ) -> String {
        if line.is_empty() {
            // Show overview of all commands; right-arrow completes to the first.
            let term_width = crate::display::term_size().0 as usize;
            let available = term_width.saturating_sub(2); // "> " prefix
            let overview = self.build_overview(available);
            self.current_hint = self.commands.first().cloned().unwrap_or_default();
            return self.paint(&overview, use_ansi_coloring);
        }

        self.current_hint = if let Some(partial) = line.strip_prefix("/theme ") {
            let names = crate::display::theme::BUILT_IN_THEMES;
            Self::complete_from(partial, names)
        } else if let Some(partial) = line.strip_prefix("/alias ") {
            Self::complete_from(partial, &["--list", "--remove "])
        } else if let Some(partial) = line.strip_prefix("/show ") {
            Self::complete_from(partial, &["last"])
        } else if line.starts_with('/') {
            Self::complete_from(line, &self.commands)
        } else {
            String::new()
        };

        self.paint(&self.current_hint, use_ansi_coloring)
    }

    fn complete_hint(&self) -> String {
        self.current_hint.clone()
    }

    fn next_hint_token(&self) -> String {
        self.current_hint
            .split_whitespace()
            .next()
            .unwrap_or_default()
            .to_string()
    }
}

pub struct CommandCompleter {
    commands: Vec<String>,
}

impl CommandCompleter {
    pub fn new(commands: Vec<String>) -> Self {
        Self { commands }
    }
}

impl Completer for CommandCompleter {
    fn complete(&mut self, line: &str, pos: usize) -> Vec<Suggestion> {
        let prefix = line.get(..pos).unwrap_or(line);
        let slash_pos = match prefix.find('/') {
            Some(p) => p,
            None => return vec![],
        };
        let typed = prefix.get(slash_pos..).unwrap_or("");

        self.commands
            .iter()
            .filter(|cmd| cmd.starts_with(typed))
            .map(|cmd| Suggestion {
                value: cmd.clone(),
                description: None,
                style: None,
                extra: None,
                span: Span::new(slash_pos, pos),
                append_whitespace: true,
                match_indices: None,
            })
            .collect()
    }
}

pub fn create_editor(history_path: Option<PathBuf>, extra_commands: &[CommandDef]) -> Reedline {
    let mut commands: Vec<String> = slash::BUILT_IN_COMMANDS
        .iter()
        .map(|(name, _)| format!("/{name}"))
        .collect();
    for cmd in extra_commands {
        let name = format!("/{}", cmd.name);
        if !commands.contains(&name) {
            commands.push(name);
        }
    }

    let completer = Box::new(CommandCompleter::new(commands.clone()));

    let completion_menu = ColumnarMenu::default()
        .with_name("completion_menu")
        .with_text_style(Style::new().fg(nu_ansi_term::Color::Default))
        .with_selected_text_style(Style::new().fg(nu_ansi_term::Color::Default).bold());

    let mut keybindings = reedline::default_emacs_keybindings();

    keybindings.add_binding(
        KeyModifiers::ALT,
        KeyCode::Enter,
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );
    keybindings.add_binding(
        KeyModifiers::CONTROL,
        KeyCode::Char('j'),
        ReedlineEvent::Edit(vec![EditCommand::InsertNewline]),
    );

    keybindings.add_binding(
        KeyModifiers::NONE,
        KeyCode::Tab,
        ReedlineEvent::UntilFound(vec![
            ReedlineEvent::Menu("completion_menu".to_string()),
            ReedlineEvent::MenuNext,
        ]),
    );

    let edit_mode = Box::new(Emacs::new(keybindings));

    let hinter = Box::new(GooseHinter::new(
        commands,
        Style::new().fg(style::hinter_color()),
    ));

    let mut editor = Reedline::create()
        .with_completer(completer)
        .with_menu(ReedlineMenu::EngineCompleter(Box::new(completion_menu)))
        .with_edit_mode(edit_mode)
        .with_hinter(hinter)
        .with_ansi_colors(true)
        .use_bracketed_paste(true)
        .with_buffer_editor(editor_command(), editor_temp_path());

    if let Some(path) = history_path {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(history) = FileBackedHistory::with_file(1000, path) {
            editor = editor.with_history(Box::new(history));
        }
    }

    editor
}

pub async fn read_input(
    mut editor: Reedline,
    prompt: GoosePrompt,
) -> (Reedline, GoosePrompt, InputEvent) {
    tokio::task::spawn_blocking(move || {
        let event = match editor.read_line(&prompt) {
            Ok(Signal::Success(line)) => InputEvent::Line(line),
            Ok(Signal::CtrlC) => InputEvent::CtrlC,
            Ok(Signal::CtrlD) => InputEvent::CtrlD,
            Err(_) => InputEvent::CtrlD,
        };
        (editor, prompt, event)
    })
    .await
    .unwrap_or_else(|e| {
        tracing::warn!("input thread panicked: {e}");
        (
            create_editor(None, &[]),
            GoosePrompt::new(),
            InputEvent::CtrlD,
        )
    })
}

/// Build the editor Command from `$VISUAL` / `$EDITOR`, handling quoted paths and args.
/// Falls back to `vi` if neither var is set or if the value has malformed quoting.
fn editor_command() -> std::process::Command {
    let raw = std::env::var("VISUAL")
        .or_else(|_| std::env::var("EDITOR"))
        .unwrap_or_else(|_| "vi".to_string());
    let tokens = shlex::split(&raw).unwrap_or_else(|| {
        tracing::warn!("malformed $EDITOR quoting ({raw:?}), falling back to vi");
        vec!["vi".to_string()]
    });
    let (exe, args) = tokens
        .split_first()
        .map_or(("vi", &[][..]), |(e, a)| (e.as_str(), a));
    let mut cmd = std::process::Command::new(exe);
    cmd.args(args);
    cmd
}

/// Temp file for Ctrl+O editor. PID suffix prevents concurrent session collisions.
/// Reedline overwrites this file with the current buffer on each Ctrl+O invocation,
/// so stale content is not a concern.
fn editor_temp_path() -> PathBuf {
    std::env::temp_dir().join(format!("goose-edit-{}.md", std::process::id()))
}

#[cfg(test)]
mod editor_tests {
    fn parse_editor(raw: &str) -> (String, Vec<String>) {
        let tokens = shlex::split(raw).unwrap_or_else(|| vec!["vi".to_string()]);
        let (exe, args) = tokens
            .split_first()
            .map_or(("vi".to_string(), vec![]), |(e, a)| (e.clone(), a.to_vec()));
        (exe, args)
    }

    #[test]
    fn editor_with_args() {
        let (exe, args) = parse_editor("code -w");
        assert_eq!(exe, "code");
        assert_eq!(args, vec!["-w"]);
    }

    #[test]
    fn quoted_path_with_spaces() {
        let (exe, args) = parse_editor(r#""/Applications/My Editor/bin/edit" --wait"#);
        assert_eq!(exe, "/Applications/My Editor/bin/edit");
        assert_eq!(args, vec!["--wait"]);
    }

    #[test]
    fn malformed_quotes_fallback() {
        // shlex::split returns None for unclosed quotes — falls back to vi
        let (exe, args) = parse_editor(r#""unclosed"#);
        assert_eq!(exe, "vi");
        assert!(args.is_empty());
    }
}
