use std::collections::HashMap;
use std::time::Duration;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use sacp::schema::{
    PermissionOptionKind, RequestPermissionOutcome, RequestPermissionRequest,
    SelectedPermissionOutcome,
};

use crate::display;

pub struct PermissionHandler {
    memory: HashMap<String, PermissionOptionKind>,
    auto_approve: bool,
}

impl PermissionHandler {
    pub fn new(auto_approve: bool) -> Self {
        Self {
            memory: HashMap::new(),
            auto_approve,
        }
    }

    pub fn try_resolve(&self, req: &RequestPermissionRequest) -> Option<RequestPermissionOutcome> {
        if self.auto_approve {
            return select_option_by_kind(req, PermissionOptionKind::AllowOnce);
        }
        let tool = tool_name(req);
        let remembered = self.memory.get(tool)?;
        select_option_by_kind(req, *remembered)
    }

    pub fn resolve_with_key(
        &mut self,
        req: &RequestPermissionRequest,
        key: char,
    ) -> RequestPermissionOutcome {
        let kind = match key {
            'y' => PermissionOptionKind::AllowOnce,
            'a' => PermissionOptionKind::AllowAlways,
            'n' => PermissionOptionKind::RejectOnce,
            'v' => PermissionOptionKind::RejectAlways,
            _ => return RequestPermissionOutcome::Cancelled,
        };

        let name = tool_name(req);
        if matches!(
            kind,
            PermissionOptionKind::AllowAlways | PermissionOptionKind::RejectAlways
        ) && name != "unknown"
        {
            self.memory.insert(name.to_owned(), kind);
        }

        select_option_by_kind(req, kind).unwrap_or(RequestPermissionOutcome::Cancelled)
    }
}

/// Line-based permission prompt for non-raw-mode interactive sessions (Plain mode).
/// Prints to stderr, reads a single line from stdin. Blocking.
pub(crate) fn prompt_permission_line(title: &str) -> char {
    let title = crate::display::sanitize_control_chars(title);
    eprintln!(
        "  Tool call: {title} — approve? [y]es / [n]o / [a]lways / ne[v]er / (Esc to cancel)"
    );
    eprint!("  > ");
    let mut buf = String::new();
    if std::io::stdin().read_line(&mut buf).is_err() {
        return '\x1b';
    }
    match buf.trim() {
        "y" | "yes" => 'y',
        "n" | "no" => 'n',
        "a" | "always" => 'a',
        "v" | "never" => 'v',
        _ => '\x1b',
    }
}

pub fn poll_permission_key(timeout: Duration) -> Option<char> {
    if !event::poll(timeout).ok()? {
        return None;
    }
    match event::read().ok()? {
        Event::Key(KeyEvent {
            code: KeyCode::Char('c'),
            modifiers,
            ..
        }) if modifiers.contains(KeyModifiers::CONTROL) => Some('\x1b'),
        Event::Key(KeyEvent {
            code: KeyCode::Esc, ..
        }) => Some('\x1b'),
        Event::Key(KeyEvent {
            code: KeyCode::Char(c),
            ..
        }) if matches!(c, 'y' | 'n' | 'a' | 'v') => Some(c),
        _ => None,
    }
}

/// Requires raw mode to be active so individual keystrokes are read without Enter.
pub fn render_permission_prompt(title: &str, input: Option<&serde_json::Value>) {
    display::print_permission_prompt(title, input);
    eprint!(
        "  {}{}  {}{}  {}{}  {}{}{}  {}\r\n",
        display::style::hotkey("[y]"),
        display::style::dim("es"),
        display::style::hotkey("[n]"),
        display::style::dim("o"),
        display::style::hotkey("[a]"),
        display::style::dim("lways"),
        display::style::dim("ne"),
        display::style::hotkey("[v]"),
        display::style::dim("er"),
        display::style::dim("(Esc to cancel)"),
    );
}

/// Extract the tool identity for permission memory keying.
/// ACP's `ToolCallUpdateFields` only exposes `title` (display name) — there is no
/// separate stable tool identifier in the schema. The server sets `title` to the
/// tool name (e.g. "shell", "read_file"), so this is the best available key.
fn tool_name(req: &RequestPermissionRequest) -> &str {
    req.tool_call.fields.title.as_deref().unwrap_or("unknown")
}

fn select_option_by_kind(
    req: &RequestPermissionRequest,
    kind: PermissionOptionKind,
) -> Option<RequestPermissionOutcome> {
    let option = req.options.iter().find(|o| o.kind == kind)?;
    Some(RequestPermissionOutcome::Selected(
        SelectedPermissionOutcome::new(option.option_id.clone()),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use sacp::schema::{
        PermissionOption, PermissionOptionKind, RequestPermissionRequest, ToolCallUpdate,
        ToolCallUpdateFields,
    };

    fn make_request(tool_title: &str) -> RequestPermissionRequest {
        let fields = ToolCallUpdateFields::new().title(tool_title.to_owned());
        let tool_call = ToolCallUpdate::new("tc-1", fields);
        RequestPermissionRequest::new(
            "session-1",
            tool_call,
            vec![
                PermissionOption::new("allow-once", "Allow Once", PermissionOptionKind::AllowOnce),
                PermissionOption::new(
                    "allow-always",
                    "Allow Always",
                    PermissionOptionKind::AllowAlways,
                ),
                PermissionOption::new(
                    "reject-once",
                    "Reject Once",
                    PermissionOptionKind::RejectOnce,
                ),
                PermissionOption::new(
                    "reject-always",
                    "Reject Always",
                    PermissionOptionKind::RejectAlways,
                ),
            ],
        )
    }

    #[test]
    fn auto_approve_resolves_allow_once() {
        let handler = PermissionHandler::new(true);
        let req = make_request("shell");
        let outcome = handler.try_resolve(&req).unwrap();
        assert_eq!(
            outcome,
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new("allow-once"))
        );
    }

    #[test]
    fn no_memory_returns_none() {
        let handler = PermissionHandler::new(false);
        let req = make_request("shell");
        assert!(handler.try_resolve(&req).is_none());
    }

    #[test]
    fn always_allow_remembered() {
        let mut handler = PermissionHandler::new(false);
        let req = make_request("shell");

        handler.resolve_with_key(&req, 'a');
        let outcome = handler.try_resolve(&req).unwrap();
        assert_eq!(
            outcome,
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new("allow-always"))
        );
    }

    #[test]
    fn never_allow_remembered() {
        let mut handler = PermissionHandler::new(false);
        let req = make_request("read_file");

        handler.resolve_with_key(&req, 'v');
        let outcome = handler.try_resolve(&req).unwrap();
        assert_eq!(
            outcome,
            RequestPermissionOutcome::Selected(SelectedPermissionOutcome::new("reject-always"))
        );
    }

    #[test]
    fn once_decisions_not_remembered() {
        let mut handler = PermissionHandler::new(false);
        let req = make_request("shell");

        handler.resolve_with_key(&req, 'y');
        assert!(handler.try_resolve(&req).is_none());

        handler.resolve_with_key(&req, 'n');
        assert!(handler.try_resolve(&req).is_none());
    }

    #[test]
    fn memory_is_per_tool() {
        let mut handler = PermissionHandler::new(false);
        let shell_req = make_request("shell");
        let read_req = make_request("read_file");

        handler.resolve_with_key(&shell_req, 'a');

        let shell_outcome = handler.try_resolve(&shell_req);
        assert!(shell_outcome.is_some());

        assert!(handler.try_resolve(&read_req).is_none());
    }

    #[test]
    fn escape_maps_to_cancelled() {
        let mut handler = PermissionHandler::new(false);
        let req = make_request("shell");
        let outcome = handler.resolve_with_key(&req, '\x1b');
        assert_eq!(outcome, RequestPermissionOutcome::Cancelled);
    }

    #[test]
    fn always_not_persisted_for_missing_title() {
        let mut handler = PermissionHandler::new(false);
        // Build a request with no title (falls back to "unknown")
        let fields = ToolCallUpdateFields::new();
        let tool_call = ToolCallUpdate::new("tc-1", fields);
        let req = RequestPermissionRequest::new(
            "session-1",
            tool_call,
            vec![
                PermissionOption::new("allow-once", "Allow Once", PermissionOptionKind::AllowOnce),
                PermissionOption::new(
                    "allow-always",
                    "Allow Always",
                    PermissionOptionKind::AllowAlways,
                ),
            ],
        );

        // "Always allow" should resolve but NOT be remembered
        handler.resolve_with_key(&req, 'a');
        assert!(
            handler.try_resolve(&req).is_none(),
            "should not persist memory for unknown tool title"
        );
    }
}
