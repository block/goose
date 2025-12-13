use crate::components::chat::ChatComponent;
use crate::components::info::InfoComponent;
use crate::components::input::InputComponent;
use crate::components::popups::builder::BuilderPopup;
use crate::components::popups::config::ConfigPopup;
use crate::components::popups::help::HelpPopup;
use crate::components::popups::message::MessagePopup;
use crate::components::popups::session::SessionPopup;
use crate::components::popups::theme::ThemePopup;
use crate::components::popups::todo::TodoPopup;
use crate::components::status::StatusComponent;
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{ActivePopup, AppState};
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
    config_popup: ConfigPopup,
    theme_popup: ThemePopup,

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
            help_popup: HelpPopup::new(),
            session_popup: SessionPopup::new(),
            builder_popup: BuilderPopup::new(),
            message_popup: MessagePopup::new(),
            config_popup: ConfigPopup::new(),
            theme_popup: ThemePopup::new(),

            last_popup_close_time: None,
        }
    }

    fn handle_popups(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        let popup_handler: Option<&mut dyn Component> = match &state.active_popup {
            ActivePopup::Todo => Some(&mut self.todo_popup),
            ActivePopup::Help => Some(&mut self.help_popup),
            ActivePopup::SessionPicker => Some(&mut self.session_popup),
            ActivePopup::CommandBuilder => Some(&mut self.builder_popup),
            ActivePopup::MessageInfo(_) => Some(&mut self.message_popup),
            ActivePopup::Config(_) => Some(&mut self.config_popup),
            ActivePopup::ThemePicker => Some(&mut self.theme_popup),
            ActivePopup::None => None,
        };

        if let Some(handler) = popup_handler {
            if let Some(action) = handler.handle_event(event, state)? {
                if matches!(
                    action,
                    Action::ClosePopup
                        | Action::ResumeSession(_)
                        | Action::CreateNewSession
                        | Action::DeleteCustomCommand(_)
                        | Action::SubmitCommandBuilder(_, _)
                        | Action::ChangeTheme(_)
                ) {
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
            if key.code == KeyCode::Char('l') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Some(Action::Refresh);
            }
            if key.code == KeyCode::Char('t') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Some(Action::ToggleTodo);
            }
            if key.code == KeyCode::Char('s') && key.modifiers.contains(KeyModifiers::CONTROL) {
                return Some(Action::ToggleCopyMode);
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
            Event::ProvidersLoaded(providers) => {
                return Ok(Some(Action::ProvidersLoaded(providers.clone())))
            }
            Event::ExtensionsLoaded(extensions) => {
                return Ok(Some(Action::ExtensionsLoaded(extensions.clone())))
            }
            Event::ModelsLoaded { provider, models } => {
                return Ok(Some(Action::ModelsLoaded {
                    provider: provider.clone(),
                    models: models.clone(),
                }))
            }
            Event::ConfigLoaded(config) => return Ok(Some(Action::ConfigLoaded(config.clone()))),
            Event::Error(e) => return Ok(Some(Action::Error(e.clone()))),
            Event::Flash(msg) => return Ok(Some(Action::ShowFlash(msg.clone()))),
            Event::Resize => return Ok(Some(Action::Resize)),
            Event::CwdAnalysisComplete(result) => {
                return Ok(Some(Action::CwdAnalysisComplete(result.clone())))
            }
            _ => {}
        }

        // 1. Global Shortcuts (Priority over popups for shortcuts like ToggleTodo)
        if let Some(action) = self.handle_global_shortcuts(event, state) {
            return Ok(Some(action));
        }

        // 2. Popups
        if state.active_popup != ActivePopup::None {
            if let Some(action) = self.handle_popups(event, state)? {
                return Ok(Some(action));
            }
            // If popup is active but returned no action, we still consume the event
            // to prevent it from leaking to underlying layers (input/chat).
            return Ok(None);
        }

        // Check scroll debounce (only relevant if no popup is active)
        if let Event::Mouse(m) = event {
            if matches!(
                m.kind,
                crossterm::event::MouseEventKind::ScrollDown
                    | crossterm::event::MouseEventKind::ScrollUp
            ) {
                if let Some(last) = self.last_popup_close_time {
                    if last.elapsed() < std::time::Duration::from_millis(500) {
                        return Ok(None);
                    }
                }
            }
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
        let theme = &state.config.theme;
        let bg_block = ratatui::widgets::Block::default()
            .style(ratatui::style::Style::default().bg(theme.base.background));
        f.render_widget(bg_block, f.area());

        let max_input_height = (f.area().height / 2).max(3);
        let input_height = self.input.height(max_input_height);
        let status_height = self.status.height(area.width, state);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Min(1),
                Constraint::Length(1),
                Constraint::Length(input_height),
                Constraint::Length(status_height),
            ])
            .split(area);

        self.chat.render(f, chunks[0], state);
        self.info.render(f, chunks[1], state);
        self.input.render(f, chunks[2], state);
        self.status.render(f, chunks[3], state);

        match &state.active_popup {
            ActivePopup::Todo => self.todo_popup.render(f, f.area(), state),
            ActivePopup::Help => self.help_popup.render(f, f.area(), state),
            ActivePopup::SessionPicker => self.session_popup.render(f, f.area(), state),
            ActivePopup::CommandBuilder => self.builder_popup.render(f, f.area(), state),
            ActivePopup::MessageInfo(_) => self.message_popup.render(f, f.area(), state),
            ActivePopup::Config(_) => self.config_popup.render(f, f.area(), state),
            ActivePopup::ThemePicker => self.theme_popup.render(f, f.area(), state),
            ActivePopup::None => {}
        }
    }
}
