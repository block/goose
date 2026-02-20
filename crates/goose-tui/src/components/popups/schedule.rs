use super::{navigate_list, popup_block, render_hints, render_scrollbar};
use crate::components::Component;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{ActivePopup, AppState};
use crate::utils::file_completion::{complete_path, derive_job_id_from_path};
use crate::utils::layout::centered_rect;
use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers, MouseEventKind};
use goose_client::{ScheduledJob, SessionDisplayInfo};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, ScrollbarState,
};
use ratatui::Frame;
use ratatui_textarea::TextArea;

const CRON_PRESETS: [(&str, &str, &str); 5] = [
    ("1", "0 * * * *", "Every hour"),
    ("2", "0 9 * * *", "Daily at 9am"),
    ("3", "0 9 * * 1", "Weekly Monday 9am"),
    ("4", "*/30 * * * *", "Every 30 minutes"),
    ("5", "0 0 * * *", "Daily at midnight"),
];

#[derive(Default, Debug, PartialEq)]
pub enum View {
    #[default]
    List,
    Create,
    Edit,
    History,
    ConfirmDelete,
}

#[derive(Default)]
pub enum FormField {
    #[default]
    RecipePath,
    JobId,
    Cron,
}

pub struct SchedulePopup {
    pub view: View,
    pub jobs: Vec<ScheduledJob>,
    pub sessions: Vec<SessionDisplayInfo>,
    pub list_state: ListState,
    pub scroll_state: ScrollbarState,
    pub history_list_state: ListState,
    pub form_field: FormField,
    pub recipe_input: TextArea<'static>,
    pub job_id_input: TextArea<'static>,
    pub cron_input: TextArea<'static>,
    pub editing_job_id: Option<String>,
    pub error_message: Option<String>,
    pub pending_delete_id: Option<String>,
    pub file_completions: Vec<(String, bool)>,
    pub completion_selected: usize,
    pub job_id_auto_generated: bool,
}

impl Default for SchedulePopup {
    fn default() -> Self {
        Self {
            view: View::List,
            jobs: Vec::new(),
            sessions: Vec::new(),
            list_state: ListState::default(),
            scroll_state: ScrollbarState::default(),
            history_list_state: ListState::default(),
            form_field: FormField::RecipePath,
            recipe_input: TextArea::default(),
            job_id_input: TextArea::default(),
            cron_input: TextArea::default(),
            editing_job_id: None,
            error_message: None,
            pending_delete_id: None,
            file_completions: Vec::new(),
            completion_selected: 0,
            job_id_auto_generated: true,
        }
    }
}

impl SchedulePopup {
    pub fn new() -> Self {
        Self::default()
    }

    fn reset(&mut self) {
        self.view = View::List;
        self.form_field = FormField::RecipePath;
        self.recipe_input = TextArea::default();
        self.job_id_input = TextArea::default();
        self.cron_input = TextArea::default();
        self.editing_job_id = None;
        self.error_message = None;
        self.pending_delete_id = None;
        self.sessions.clear();
        self.history_list_state = ListState::default();
        self.file_completions.clear();
        self.completion_selected = 0;
        self.job_id_auto_generated = true;
    }

    fn update_job_id_from_recipe(&mut self) {
        if self.job_id_auto_generated {
            let recipe_path = Self::get_input_text(&self.recipe_input);
            let derived_id = derive_job_id_from_path(&recipe_path);
            self.job_id_input = TextArea::default();
            if !derived_id.is_empty() {
                self.job_id_input.insert_str(&derived_id);
            }
        }
    }

    fn handle_paste(&mut self, text: &str) {
        if self.view != View::Create && self.view != View::Edit {
            return;
        }
        let text = text.replace("\r\n", "\n").replace('\r', "\n");
        let input = match self.form_field {
            FormField::RecipePath => &mut self.recipe_input,
            FormField::JobId => {
                self.job_id_auto_generated = false;
                &mut self.job_id_input
            }
            FormField::Cron => &mut self.cron_input,
        };
        input.insert_str(text);
        if matches!(self.form_field, FormField::RecipePath) {
            self.update_job_id_from_recipe();
        }
    }

    fn selected_job(&self) -> Option<&ScheduledJob> {
        self.list_state.selected().and_then(|i| self.jobs.get(i))
    }

    fn apply_preset(&mut self, key: char) {
        if let Some((_, cron, _)) = CRON_PRESETS.iter().find(|(k, _, _)| k.starts_with(key)) {
            self.cron_input = TextArea::default();
            self.cron_input.insert_str(cron);
        }
    }

    pub fn get_input_text(input: &TextArea) -> String {
        input
            .lines()
            .first()
            .map(|s| s.to_string())
            .unwrap_or_default()
    }

    pub fn handle_list_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(next) = navigate_list(self.list_state.selected(), 1, self.jobs.len()) {
                    self.list_state.select(Some(next));
                    self.scroll_state = self.scroll_state.position(next);
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(next) = navigate_list(self.list_state.selected(), -1, self.jobs.len()) {
                    self.list_state.select(Some(next));
                    self.scroll_state = self.scroll_state.position(next);
                }
                None
            }
            KeyCode::Char('n') => {
                self.view = View::Create;
                self.form_field = FormField::RecipePath;
                None
            }
            KeyCode::Char('e') => {
                if let Some((job_id, cron)) =
                    self.selected_job().map(|j| (j.id.clone(), j.cron.clone()))
                {
                    self.editing_job_id = Some(job_id);
                    self.cron_input = TextArea::default();
                    self.cron_input.insert_str(&cron);
                    self.view = View::Edit;
                }
                None
            }
            KeyCode::Char('d') => {
                if let Some(job) = self.selected_job() {
                    self.pending_delete_id = Some(job.id.clone());
                    self.view = View::ConfirmDelete;
                }
                None
            }
            KeyCode::Char('r') => self
                .selected_job()
                .map(|j| Action::RunScheduleNow(j.id.clone())),
            KeyCode::Char('p') => self.selected_job().map(|j| {
                if j.paused {
                    Action::UnpauseSchedule(j.id.clone())
                } else {
                    Action::PauseSchedule(j.id.clone())
                }
            }),
            KeyCode::Char('K') => self
                .selected_job()
                .filter(|j| j.currently_running)
                .map(|j| Action::KillSchedule(j.id.clone())),
            KeyCode::Char('h') => {
                if let Some(job_id) = self.selected_job().map(|j| j.id.clone()) {
                    self.editing_job_id = Some(job_id.clone());
                    self.view = View::History;
                    return Some(Action::FetchScheduleSessions(job_id));
                }
                None
            }
            KeyCode::Char('R') => Some(Action::RefreshSchedules),
            KeyCode::Esc | KeyCode::Char('q') => Some(Action::ClosePopup),
            _ => None,
        }
    }

    pub fn handle_create_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.reset();
                None
            }
            KeyCode::Tab | KeyCode::Down if self.file_completions.is_empty() => {
                self.form_field = match self.form_field {
                    FormField::RecipePath => FormField::JobId,
                    FormField::JobId => FormField::Cron,
                    FormField::Cron => FormField::RecipePath,
                };
                None
            }
            KeyCode::Tab if !self.file_completions.is_empty() => {
                self.apply_file_completion();
                None
            }
            KeyCode::Down if !self.file_completions.is_empty() => {
                self.completion_selected =
                    (self.completion_selected + 1) % self.file_completions.len();
                None
            }
            KeyCode::Up if !self.file_completions.is_empty() => {
                self.completion_selected = self
                    .completion_selected
                    .checked_sub(1)
                    .unwrap_or(self.file_completions.len() - 1);
                None
            }
            KeyCode::BackTab => {
                self.form_field = match self.form_field {
                    FormField::RecipePath => FormField::Cron,
                    FormField::JobId => FormField::RecipePath,
                    FormField::Cron => FormField::JobId,
                };
                None
            }
            KeyCode::Up if key.modifiers.contains(KeyModifiers::SHIFT) => {
                self.form_field = match self.form_field {
                    FormField::RecipePath => FormField::Cron,
                    FormField::JobId => FormField::RecipePath,
                    FormField::Cron => FormField::JobId,
                };
                None
            }
            KeyCode::Char(c @ '1'..='5') if matches!(self.form_field, FormField::Cron) => {
                self.apply_preset(c);
                None
            }
            KeyCode::Enter if !self.file_completions.is_empty() => {
                self.apply_file_completion();
                None
            }
            KeyCode::Enter => {
                let recipe = Self::get_input_text(&self.recipe_input);
                let id = Self::get_input_text(&self.job_id_input);
                let cron = Self::get_input_text(&self.cron_input);
                if recipe.is_empty() || id.is_empty() || cron.is_empty() {
                    self.error_message = Some("All fields required".into());
                    return None;
                }
                self.reset();
                Some(Action::CreateSchedule {
                    id,
                    recipe_source: recipe,
                    cron,
                })
            }
            _ => {
                match self.form_field {
                    FormField::RecipePath => {
                        self.recipe_input.input(key);
                        self.update_job_id_from_recipe();
                        self.update_file_completions();
                    }
                    FormField::JobId => {
                        self.job_id_auto_generated = false;
                        self.job_id_input.input(key);
                    }
                    FormField::Cron => {
                        self.cron_input.input(key);
                    }
                }
                None
            }
        }
    }

    fn update_file_completions(&mut self) {
        if !matches!(self.form_field, FormField::RecipePath) {
            self.file_completions.clear();
            self.completion_selected = 0;
            return;
        }
        let partial = Self::get_input_text(&self.recipe_input);
        if partial.is_empty() {
            self.file_completions.clear();
            self.completion_selected = 0;
            return;
        }
        let cwd = std::env::current_dir().unwrap_or_default();
        self.file_completions = complete_path(&partial, &cwd);
        if self.completion_selected >= self.file_completions.len() {
            self.completion_selected = 0;
        }
    }

    fn apply_file_completion(&mut self) {
        if let Some((name, is_dir)) = self.file_completions.get(self.completion_selected).cloned() {
            let current = Self::get_input_text(&self.recipe_input);
            let new_path = if let Some(last_slash) = current.rfind('/') {
                format!("{}/{}", &current[..last_slash], name)
            } else {
                name.clone()
            };
            self.recipe_input = TextArea::default();
            self.recipe_input.insert_str(&new_path);
            if is_dir {
                self.recipe_input.insert_char('/');
            }
            self.file_completions.clear();
            self.completion_selected = 0;
            self.update_job_id_from_recipe();
        }
    }

    fn handle_edit_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.reset();
                None
            }
            KeyCode::Char(c @ '1'..='5') => {
                self.apply_preset(c);
                None
            }
            KeyCode::Enter => {
                let cron = Self::get_input_text(&self.cron_input);
                let id = self.editing_job_id.take()?;
                self.reset();
                Some(Action::UpdateScheduleCron { id, cron })
            }
            _ => {
                self.cron_input.input(key);
                None
            }
        }
    }

    pub fn handle_confirm_delete_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('y') | KeyCode::Char('Y') => {
                let id = self.pending_delete_id.take()?;
                self.view = View::List;
                Some(Action::DeleteSchedule(id))
            }
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
                self.pending_delete_id = None;
                self.view = View::List;
                None
            }
            _ => None,
        }
    }

    fn handle_history_key(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc => {
                self.reset();
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                if let Some(next) =
                    navigate_list(self.history_list_state.selected(), 1, self.sessions.len())
                {
                    self.history_list_state.select(Some(next));
                }
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                if let Some(next) =
                    navigate_list(self.history_list_state.selected(), -1, self.sessions.len())
                {
                    self.history_list_state.select(Some(next));
                }
                None
            }
            _ => None,
        }
    }

    fn render_list(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;

        let items: Vec<ListItem> = self
            .jobs
            .iter()
            .map(|job| {
                let (icon, color) = if job.currently_running {
                    ("●", Color::Green)
                } else if job.paused {
                    ("◐", Color::Yellow)
                } else {
                    ("○", Color::DarkGray)
                };
                let status = if job.currently_running {
                    "Running"
                } else if job.paused {
                    "Paused"
                } else {
                    "Idle"
                };
                ListItem::new(Line::from(vec![
                    Span::styled(format!("{icon} "), Style::default().fg(color)),
                    Span::styled(
                        format!("{:<20} ", job.id),
                        Style::default().fg(theme.base.foreground),
                    ),
                    Span::styled(
                        format!("{:<25} ", &job.cron),
                        Style::default().fg(theme.status.info),
                    ),
                    Span::styled(status, Style::default().fg(theme.base.border)),
                ]))
            })
            .collect();

        self.scroll_state = self.scroll_state.content_length(self.jobs.len());

        if items.is_empty() {
            let empty_msg = Paragraph::new("No scheduled jobs. Press 'n' to create one.")
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(theme.base.border));
            f.render_widget(empty_msg, area);
        } else {
            let list = List::new(items)
                .highlight_style(
                    Style::default()
                        .bg(theme.base.selection)
                        .add_modifier(Modifier::BOLD),
                )
                .highlight_symbol("▶ ");

            f.render_stateful_widget(list, area, &mut self.list_state);
            render_scrollbar(f, area, &mut self.scroll_state);
        }
    }

    fn render_create(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" New Schedule ");
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(3),
            Constraint::Length(2),
            Constraint::Length(1),
        ])
        .split(inner);

        let active_style = Style::default().fg(theme.status.warning);
        let inactive_style = Style::default().fg(theme.base.border);

        self.recipe_input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Recipe Path")
                .border_style(if matches!(self.form_field, FormField::RecipePath) {
                    active_style
                } else {
                    inactive_style
                }),
        );
        self.job_id_input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Job ID")
                .border_style(if matches!(self.form_field, FormField::JobId) {
                    active_style
                } else {
                    inactive_style
                }),
        );
        self.cron_input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("Cron")
                .border_style(if matches!(self.form_field, FormField::Cron) {
                    active_style
                } else {
                    inactive_style
                }),
        );

        f.render_widget(&self.recipe_input, chunks[0]);
        f.render_widget(&self.job_id_input, chunks[1]);
        f.render_widget(&self.cron_input, chunks[2]);

        let presets: String = CRON_PRESETS
            .iter()
            .map(|(k, _, d)| format!("[{k}]{d} "))
            .collect();
        f.render_widget(
            Paragraph::new(presets).style(Style::default().fg(theme.base.border)),
            chunks[3],
        );

        if let Some(ref err) = self.error_message {
            f.render_widget(
                Paragraph::new(err.as_str()).style(Style::default().fg(theme.status.error)),
                chunks[4],
            );
        }
    }

    fn render_edit(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let title = format!(" Edit: {} ", self.editing_job_id.as_deref().unwrap_or(""));
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        f.render_widget(block, area);

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Length(3),
            Constraint::Length(2),
        ])
        .split(inner);

        if let Some(job) = self
            .editing_job_id
            .as_ref()
            .and_then(|id| self.jobs.iter().find(|j| &j.id == id))
        {
            f.render_widget(
                Paragraph::new(format!("Current: {}", job.cron))
                    .style(Style::default().fg(theme.base.border)),
                chunks[0],
            );
        }

        self.cron_input.set_block(
            Block::default()
                .borders(Borders::ALL)
                .title("New Cron")
                .border_style(Style::default().fg(theme.status.warning)),
        );
        f.render_widget(&self.cron_input, chunks[1]);

        let presets: String = CRON_PRESETS
            .iter()
            .map(|(k, _, d)| format!("[{k}]{d} "))
            .collect();
        f.render_widget(
            Paragraph::new(presets).style(Style::default().fg(theme.base.border)),
            chunks[2],
        );
    }

    fn render_history(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let title = format!(
            " History: {} ",
            self.editing_job_id.as_deref().unwrap_or("")
        );

        let items: Vec<ListItem> = self
            .sessions
            .iter()
            .map(|s| {
                ListItem::new(Line::from(vec![
                    Span::styled(
                        format!("{:<20} ", s.name),
                        Style::default().fg(theme.base.foreground),
                    ),
                    Span::styled(
                        format!("{:<20} ", &s.created_at),
                        Style::default().fg(theme.status.info),
                    ),
                    Span::styled(
                        format!("{:>4} msgs  ", s.message_count),
                        Style::default().fg(theme.base.border),
                    ),
                    Span::styled(
                        format!("{:>6} tokens", s.total_tokens.unwrap_or(0)),
                        Style::default().fg(theme.base.border),
                    ),
                ]))
            })
            .collect();

        if items.is_empty() {
            let empty_msg = Paragraph::new("No execution history found.")
                .alignment(ratatui::layout::Alignment::Center)
                .style(Style::default().fg(theme.base.border));
            let block = Block::default().borders(Borders::ALL).title(title);
            let inner = block.inner(area);
            f.render_widget(block, area);
            f.render_widget(empty_msg, inner);
        } else {
            let list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title(title))
                .highlight_style(
                    Style::default()
                        .bg(theme.base.selection)
                        .add_modifier(Modifier::BOLD),
                );

            f.render_stateful_widget(list, area, &mut self.history_list_state);
        }
    }

    fn render_confirm_delete(&self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let id = self.pending_delete_id.as_deref().unwrap_or("");
        let text = format!("Delete schedule '{id}'?\n\n[y]es  [n]o");
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Confirm Delete ");
        f.render_widget(
            Paragraph::new(text)
                .block(block)
                .style(Style::default().fg(theme.base.foreground)),
            area,
        );
    }

    fn render_file_completions(&self, f: &mut Frame, area: Rect, state: &AppState) {
        if self.file_completions.is_empty() || !matches!(self.form_field, FormField::RecipePath) {
            return;
        }

        let theme = &state.config.theme;
        let max_height = (area.height / 3).max(5);
        let content_height = (self.file_completions.len() as u16 + 2).min(max_height);
        let width = 50.min(area.width.saturating_sub(4));

        // Position below the recipe input field (first field, ~3 lines down)
        let popup_y = area.y + 4;
        let popup_area = Rect::new(area.x + 1, popup_y, width, content_height);

        f.render_widget(Clear, popup_area);

        let items: Vec<ListItem> = self
            .file_completions
            .iter()
            .enumerate()
            .map(|(i, (name, is_dir))| {
                let is_selected = i == self.completion_selected;
                let display = if *is_dir {
                    format!("{name}/")
                } else {
                    name.clone()
                };
                let prefix = if is_selected { "▶ " } else { "  " };
                let color = if is_selected {
                    theme.status.thinking
                } else if *is_dir {
                    theme.status.info
                } else {
                    theme.base.foreground
                };
                ListItem::new(Span::styled(
                    format!("{prefix}{display}"),
                    Style::default().fg(color),
                ))
            })
            .collect();

        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(" Files ")
                    .border_style(Style::default().fg(theme.base.border)),
            )
            .style(Style::default().bg(theme.base.background));

        f.render_widget(list, popup_area);
    }

    fn render_footer(&self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let help = match self.view {
            View::List => "n:new e:edit d:del r:run p:pause K:kill h:history R:refresh Esc:close",
            View::Create => "Tab:next 1-5:preset Enter:save Esc:cancel",
            View::Edit => "1-5:preset Enter:save Esc:cancel",
            View::History => "j/k:navigate Esc:back",
            View::ConfirmDelete => "y:confirm n:cancel",
        };
        render_hints(f, area, theme, &[(help, "")]);
    }
}

impl Component for SchedulePopup {
    fn handle_event(&mut self, event: &Event, state: &AppState) -> Result<Option<Action>> {
        if state.active_popup != ActivePopup::SchedulePicker {
            self.reset();
            return Ok(None);
        }

        match event {
            Event::ScheduleListLoaded(jobs) => {
                self.jobs = jobs.clone();
                if self.list_state.selected().is_none() && !self.jobs.is_empty() {
                    self.list_state.select(Some(0));
                }
                self.error_message = None;
            }
            Event::ScheduleSessionsLoaded {
                schedule_id,
                sessions,
            } => {
                if self.editing_job_id.as_ref() == Some(schedule_id) {
                    self.sessions = sessions.clone();
                    if !self.sessions.is_empty() {
                        self.history_list_state.select(Some(0));
                    }
                }
            }
            Event::ScheduleOperationSuccess(msg) => {
                self.error_message = None;
                return Ok(Some(Action::ShowFlash(msg.clone())));
            }
            Event::ScheduleOperationFailed(err) => {
                self.error_message = Some(err.clone());
            }
            Event::Paste(text) => {
                self.handle_paste(text);
            }
            Event::Input(key) => {
                return Ok(match self.view {
                    View::List => self.handle_list_key(*key),
                    View::Create => self.handle_create_key(*key),
                    View::Edit => self.handle_edit_key(*key),
                    View::History => self.handle_history_key(*key),
                    View::ConfirmDelete => self.handle_confirm_delete_key(*key),
                });
            }
            Event::Mouse(mouse) => match mouse.kind {
                MouseEventKind::ScrollDown if self.view == View::List => {
                    if let Some(next) =
                        navigate_list(self.list_state.selected(), 1, self.jobs.len())
                    {
                        self.list_state.select(Some(next));
                        self.scroll_state = self.scroll_state.position(next);
                    }
                }
                MouseEventKind::ScrollUp if self.view == View::List => {
                    if let Some(next) =
                        navigate_list(self.list_state.selected(), -1, self.jobs.len())
                    {
                        self.list_state.select(Some(next));
                        self.scroll_state = self.scroll_state.position(next);
                    }
                }
                _ => {}
            },
            _ => {}
        }
        Ok(None)
    }

    fn render(&mut self, f: &mut Frame, area: Rect, state: &AppState) {
        let theme = &state.config.theme;
        let area = centered_rect(70, 60, area);
        f.render_widget(Clear, area);

        let [content_area, hints_area] =
            Layout::vertical([Constraint::Min(1), Constraint::Length(1)])
                .margin(1)
                .areas(area);

        f.render_widget(popup_block(" Schedules ", theme), area);

        match self.view {
            View::List => self.render_list(f, content_area, state),
            View::Create => {
                self.render_create(f, content_area, state);
                self.render_file_completions(f, content_area, state);
            }
            View::Edit => self.render_edit(f, content_area, state),
            View::History => self.render_history(f, content_area, state),
            View::ConfirmDelete => self.render_confirm_delete(f, content_area, state),
        }

        self.render_footer(f, hints_area, state);
    }
}
