mod handlers;
mod render;
mod tools;
mod widgets;

use crate::components::Component;
use crate::services::config::CustomCommand;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{ActivePopup, AppState};
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::MouseEventKind;
use goose_client::ToolInfo;
use ratatui::layout::Rect;
use ratatui::widgets::{Clear, ListState, ScrollbarState};
use ratatui::Frame;
use ratatui_textarea::TextArea;
use widgets::new_text_input;

#[derive(Clone, Copy, PartialEq, Default)]
pub(super) enum View {
    #[default]
    ToolSelect,
    AliasManage,
    Editor,
}

pub struct BuilderPopup<'a> {
    pub(super) view: View,
    pub(super) list_state: ListState,
    pub(super) scroll_state: ScrollbarState,
    pub(super) search: String,
    pub(super) selected_tool_idx: Option<usize>,
    pub(super) editing_alias: Option<String>,
    pub(super) alias_name: TextArea<'a>,
    pub(super) param_inputs: Vec<(String, TextArea<'a>)>,
    pub(super) focused_field: usize,
}

impl Default for BuilderPopup<'_> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> BuilderPopup<'a> {
    pub fn new() -> Self {
        Self {
            view: View::default(),
            list_state: ListState::default().with_selected(Some(0)),
            scroll_state: ScrollbarState::default(),
            search: String::new(),
            selected_tool_idx: None,
            editing_alias: None,
            alias_name: new_text_input("Alias name"),
            param_inputs: Vec::new(),
            focused_field: 0,
        }
    }

    pub(super) fn reset(&mut self) {
        *self = Self::new();
    }

    pub(super) fn list_count(&self, state: &AppState) -> usize {
        match self.view {
            View::ToolSelect => self.tool_indices(state).len(),
            View::AliasManage => state.config.custom_commands.len(),
            View::Editor => 0,
        }
    }

    pub(super) fn navigate(&mut self, delta: i32, state: &AppState) {
        let count = self.list_count(state);
        if count == 0 {
            return;
        }

        let indices = if self.view == View::ToolSelect {
            Some(self.tool_indices(state))
        } else {
            None
        };

        let mut i = self.list_state.selected().unwrap_or(0) as i32;
        loop {
            i = (i + delta).rem_euclid(count as i32);
            if let Some(ref idx) = indices {
                if self.search.is_empty()
                    && i != 0
                    && !idx.get(i as usize).is_some_and(|x| x.is_some())
                {
                    continue;
                }
            }
            break;
        }
        self.list_state.select(Some(i as usize));
        self.scroll_state = self.scroll_state.position(i as usize);
    }

    pub(super) fn setup_editor(&mut self, tool: &ToolInfo, existing: Option<&CustomCommand>) {
        self.param_inputs = tool
            .parameters
            .iter()
            .map(|param| {
                let mut ta = new_text_input(param);
                if let Some(cmd) = existing {
                    if let Some(val) = cmd.args.get(param).and_then(|v| v.as_str()) {
                        ta.insert_str(val);
                    }
                }
                (param.clone(), ta)
            })
            .collect();

        self.alias_name = new_text_input("Alias name (e.g., gs)");
        if let Some(cmd) = existing {
            self.alias_name.insert_str(&cmd.name);
            self.editing_alias = Some(cmd.name.clone());
        }

        self.focused_field = 0;
        self.view = View::Editor;
    }

    pub(super) fn build_command(&self, tools: &[ToolInfo]) -> Option<CustomCommand> {
        let name = self.alias_name.lines().join("").trim().replace('/', "");
        if name.is_empty() {
            return None;
        }

        let tool = tools.get(self.selected_tool_idx?)?;
        let args: serde_json::Map<String, serde_json::Value> = self
            .param_inputs
            .iter()
            .map(|(k, ta)| (k.clone(), serde_json::Value::String(ta.lines().join("\n"))))
            .collect();

        Some(CustomCommand {
            name,
            description: format!("Alias for {}", tool.name),
            tool: tool.name.clone(),
            args: serde_json::Value::Object(args),
        })
    }

    pub(super) fn preview_text(&self, tools: &[ToolInfo]) -> String {
        let name = self.alias_name.lines().join("").trim().replace('/', "");
        let name_display = if name.is_empty() { "..." } else { &name };

        let Some(tool) = self.selected_tool_idx.and_then(|i| tools.get(i)) else {
            return format!("/{name_display}");
        };

        let short = tool.name.split("__").last().unwrap_or(&tool.name);
        let params: String = self
            .param_inputs
            .iter()
            .map(|(k, ta)| {
                format!(
                    "{}={}",
                    k,
                    ta.lines().join("").chars().take(20).collect::<String>()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");

        if params.is_empty() {
            format!("/{name_display} → {short}")
        } else {
            format!("/{name_display} → {short}({params})")
        }
    }

    pub(super) fn focus_next(&mut self) {
        let total = self.param_inputs.len() + 1;
        self.focused_field = (self.focused_field + 1) % total;
    }

    pub(super) fn focus_prev(&mut self) {
        let total = self.param_inputs.len() + 1;
        self.focused_field = (self.focused_field + total - 1) % total;
    }

    pub(super) fn has_input_placeholder(&self) -> bool {
        self.param_inputs
            .iter()
            .any(|(_, ta)| ta.lines().join("").contains("{input}"))
    }
}

impl<'a> Component for BuilderPopup<'a> {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if state.active_popup != ActivePopup::CommandBuilder {
            self.reset();
            return Ok(None);
        }

        match event {
            Event::Input(key) => match self.view {
                View::ToolSelect => self.handle_tool_select(key, state),
                View::AliasManage => self.handle_alias_manage(key, state),
                View::Editor => self.handle_editor(key, state),
            },
            Event::Mouse(m) => {
                match m.kind {
                    MouseEventKind::ScrollDown => self.navigate(1, state),
                    MouseEventKind::ScrollUp => self.navigate(-1, state),
                    _ => {}
                }
                Ok(None)
            }
            _ => Ok(None),
        }
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let area = centered_rect(70, 70, area);
        f.render_widget(Clear, area);

        match self.view {
            View::ToolSelect => self.render_tool_select(f, area, state),
            View::AliasManage => self.render_alias_manage(f, area, state),
            View::Editor => self.render_editor(f, area, state),
        }
    }
}
