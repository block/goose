use super::tools::build_tool_list;
use super::BuilderPopup;
use crate::state::action::Action;
use crate::state::AppState;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::ScrollbarState;

impl BuilderPopup<'_> {
    pub(super) fn handle_tool_select(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                if self.search.is_empty() {
                    self.reset();
                    return Ok(Some(Action::ClosePopup));
                }
                self.search.clear();
                self.list_state.select(Some(0));
            }
            KeyCode::Char(c) if !key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.search.push(c);
                self.list_state.select(Some(0));
            }
            KeyCode::Backspace => {
                self.search.pop();
                self.list_state.select(Some(0));
            }
            KeyCode::Down | KeyCode::Tab => self.navigate(1, state),
            KeyCode::Up | KeyCode::BackTab => self.navigate(-1, state),
            KeyCode::Enter => {
                let indices = self.tool_indices(state);
                if let Some(sel) = self.list_state.selected() {
                    if sel == 0 && self.search.is_empty() {
                        self.set_view_alias_manage();
                    } else if let Some(&Some(tool_idx)) = indices.get(sel) {
                        self.selected_tool_idx = Some(tool_idx);
                        if let Some(tool) = state.available_tools.get(tool_idx) {
                            self.setup_editor(tool, None);
                        }
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    pub(super) fn handle_alias_manage(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                self.set_view_tool_select();
            }
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => self.navigate(1, state),
            KeyCode::Up | KeyCode::Char('k') | KeyCode::BackTab => self.navigate(-1, state),
            KeyCode::Char('d') | KeyCode::Delete => {
                if let Some(selected) = self.list_state.selected() {
                    if let Some(cmd) = state.config.custom_commands.get(selected) {
                        let name = cmd.name.clone();
                        let new_len = state.config.custom_commands.len().saturating_sub(1);
                        if new_len > 0 && selected >= new_len {
                            self.list_state.select(Some(new_len - 1));
                        }
                        return Ok(Some(Action::DeleteCustomCommand(name)));
                    }
                }
            }
            KeyCode::Enter | KeyCode::Char('e') => {
                if let Some(cmd) = self
                    .list_state
                    .selected()
                    .and_then(|i| state.config.custom_commands.get(i))
                {
                    if let Some((idx, tool)) = state
                        .available_tools
                        .iter()
                        .enumerate()
                        .find(|(_, t)| t.name == cmd.tool)
                    {
                        self.selected_tool_idx = Some(idx);
                        self.setup_editor(tool, Some(cmd));
                    }
                }
            }
            _ => {}
        }
        Ok(None)
    }

    pub(super) fn handle_editor(
        &mut self,
        key: &crossterm::event::KeyEvent,
        state: &AppState,
    ) -> Result<Option<Action>> {
        match key.code {
            KeyCode::Esc => {
                self.set_view_tool_select();
            }
            KeyCode::Tab | KeyCode::Down => self.focus_next(),
            KeyCode::BackTab | KeyCode::Up => self.focus_prev(),
            KeyCode::Enter => {
                if let Some(cmd) = self.build_command(&state.available_tools) {
                    let msg = if self.editing_alias.is_some() {
                        format!("✓ Updated /{}", cmd.name)
                    } else {
                        format!("✓ Created /{}", cmd.name)
                    };
                    self.reset();
                    return Ok(Some(Action::SubmitCommandBuilder(cmd, msg)));
                }
            }
            _ => {
                if self.focused_field == 0 {
                    self.alias_name.input(*key);
                } else if let Some((_, ta)) = self.param_inputs.get_mut(self.focused_field - 1) {
                    ta.input(*key);
                }
            }
        }
        Ok(None)
    }

    pub(super) fn tool_indices(&self, state: &AppState) -> Vec<Option<usize>> {
        build_tool_list(&state.available_tools, &self.search, &state.config.theme).1
    }

    fn set_view_tool_select(&mut self) {
        self.view = super::View::ToolSelect;
        self.search.clear();
        self.list_state.select(Some(0));
    }

    fn set_view_alias_manage(&mut self) {
        self.view = super::View::AliasManage;
        self.list_state.select(Some(0));
        self.scroll_state = ScrollbarState::default();
    }
}
