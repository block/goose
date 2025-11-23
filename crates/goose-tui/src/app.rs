use crate::components::chat::ChatComponent;
use crate::components::info::InfoComponent;
use crate::components::input::InputComponent;
use crate::components::popups::builder::BuilderPopup;
use crate::components::popups::help::HelpPopup;
use crate::components::popups::message::MessagePopup;
use crate::components::popups::session::SessionPopup;
use crate::components::popups::todo::TodoPopup;
use crate::components::status::StatusComponent;
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Direction, Layout, Rect};
use ratatui::Frame;
use std::time::Instant;

pub struct App<'a> {
    chat: ChatComponent,
    input: InputComponent<'a>,
    info: InfoComponent,
    status: StatusComponent,
    todo_popup: TodoPopup,
    help_popup: HelpPopup,
    session_popup: SessionPopup,
    builder_popup: BuilderPopup<'a>,
    message_popup: MessagePopup,

    pub last_popup_close_time: Option<Instant>,
}

impl<'a> Default for App<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> App<'a> {
    pub fn new() -> Self {
        Self {
            chat: ChatComponent::new(),
            input: InputComponent::new(),
            info: InfoComponent::new(),
            status: StatusComponent::new(),
            todo_popup: TodoPopup::new(),
            help_popup: HelpPopup,
            session_popup: SessionPopup::new(),
            builder_popup: BuilderPopup::new(),
            message_popup: MessagePopup::new(),

            last_popup_close_time: None,
        }
    }

    fn handle_popups(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if state.showing_todo {
            if let Some(action) = self.todo_popup.handle_event(event, state)? {
                if matches!(action, Action::ClosePopup) {
                    self.last_popup_close_time = Some(std::time::Instant::now());
                }
                return Ok(Some(action));
            }
            return Ok(None);
        }
        if state.showing_help {
            if let Some(action) = self.help_popup.handle_event(event, state)? {
                if matches!(action, Action::ClosePopup) {
                    self.last_popup_close_time = Some(std::time::Instant::now());
                }
                return Ok(Some(action));
            }
            return Ok(None);
        }
        if state.showing_session_picker {
            if let Some(action) = self.session_popup.handle_event(event, state)? {
                if matches!(
                    action,
                    Action::ClosePopup | Action::ResumeSession(_) | Action::CreateNewSession
                ) {
                    self.last_popup_close_time = Some(std::time::Instant::now());
                }
                return Ok(Some(action));
            }
            return Ok(None);
        }
        if state.showing_command_builder {
            if let Some(action) = self.builder_popup.handle_event(event, state)? {
                if matches!(
                    action,
                    Action::ClosePopup
                        | Action::DeleteCustomCommand(_)
                        | Action::SubmitCommandBuilder(_)
                ) {
                    self.last_popup_close_time = Some(std::time::Instant::now());
                }
                return Ok(Some(action));
            }
            return Ok(None);
        }
        if state.showing_message_info.is_some() {
            if let Some(action) = self.message_popup.handle_event(event, state)? {
                if matches!(action, Action::ClosePopup) {
                    self.last_popup_close_time = Some(std::time::Instant::now());
                }
                return Ok(Some(action));
            }
            return Ok(None);
        }
        Ok(None)
    }

    fn handle_global_shortcuts(&mut self, event: &Event, state: &AppState) -> Option<Action> {
        if let Event::Input(key) = event {
            if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
                if state.is_working {
                    return Some(Action::Interrupt);
                } else if self.input.is_empty() {
                    return Some(Action::Quit);
                } else {
                    self.input.clear();
                    return None;
                }
            }
            if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Some(Action::ToggleTodo);
            }
        }
        None
    }
}

impl<'a> Component for App<'a> {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        // 0. Priority System Events
        match event {
            Event::Server(msg) => return Ok(Some(Action::ServerMessage(msg.clone()))),
            Event::SessionsList(sessions) => {
                return Ok(Some(Action::SessionsListLoaded(sessions.clone())))
            }
            Event::SessionResumed(session) => {
                return Ok(Some(Action::SessionResumed((*session).clone())))
            }
            Event::ToolsLoaded(tools) => return Ok(Some(Action::ToolsLoaded(tools.clone()))),
            Event::Error(e) => return Ok(Some(Action::Error(e.clone()))),
            Event::Resize => return Ok(Some(Action::Resize)),
            _ => {}
        }

        // 1. Popups
        if let Some(action) = self.handle_popups(event, state)? {
            return Ok(Some(action));
        }

        // Check scroll debounce
        if let Event::Mouse(m) = event {
            if matches!(
                m.kind,
                crossterm::event::MouseEventKind::ScrollDown
                    | crossterm::event::MouseEventKind::ScrollUp
            ) {
                if let Some(last) = self.last_popup_close_time {
                    if last.elapsed() < std::time::Duration::from_millis(300) {
                        return Ok(None);
                    }
                }
            }
        }

        // 2. Global Shortcuts
        if let Some(action) = self.handle_global_shortcuts(event, state) {
            return Ok(Some(action));
        }

        // 3. Input
        if let Some(action) = self.input.handle_event(event, state)? {
            return Ok(Some(action));
        }

        // 4. Chat (Navigation/Scroll)
        if let Some(action) = self.chat.handle_event(event, state)? {
            return Ok(Some(action));
        }

        // 5. Status & Info (Ticks)
        if let Event::Tick = event {
            self.status.handle_event(event, state)?;
            self.info.handle_event(event, state)?;
            return Ok(Some(Action::Tick));
        }

        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let max_input_height = (f.area().height / 2).max(3);
        let input_height = self.input.height(max_input_height);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),               // Chat
                Constraint::Length(1),            // Info Line (Puns/Todos)
                Constraint::Length(input_height), // Input
                Constraint::Length(1),            // Status Bar
            ])
            .split(area);

        self.chat.render(f, chunks[0], state);
        self.info.render(f, chunks[1], state);
        self.input.render(f, chunks[2], state);
        self.status.render(f, chunks[3], state);

        self.todo_popup.render(f, f.area(), state);
        self.help_popup.render(f, f.area(), state);
        self.session_popup.render(f, f.area(), state);
        self.builder_popup.render(f, f.area(), state);
        if state.showing_message_info.is_some() {
            self.message_popup.render(f, f.area(), state);
        }
    }
}
