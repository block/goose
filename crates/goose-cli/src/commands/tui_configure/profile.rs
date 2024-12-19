use std::{collections::HashMap, io};

use ratatui::{
    crossterm::event::{Event, KeyCode, KeyEvent},
    layout::{self, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, HighlightSpacing, List, ListState, Paragraph},
    Frame,
};
use tui_input::{backend::crossterm::EventHandler, Input};

use crate::profile::{load_profiles, remove_profile, save_profile, Profile};

use super::{
    main_area::{chunks_for_list_and_view_split, render_left_list},
    provider_list, AppOutcome,
};

pub struct ProfileUI {
    pub profile_ui_mode: ProfileUIMode,
    pub profile_list_state: ListState,
    pub profiles: HashMap<String, Profile>,
    pub edit_profile: Option<EditableProfile>,
}

impl ProfileUI {
    pub fn new() -> Self {
        let mut state = Self {
            profile_ui_mode: ProfileUIMode::ProfileView,
            profile_list_state: ListState::default(),
            profiles: load_profiles().unwrap(),
            edit_profile: None,
        };
        if state.profiles.len() > 0 {
            state.profile_list_state.select_first();
        }
        state
    }

    pub fn handle_events(&mut self, key: KeyEvent) -> io::Result<AppOutcome> {
        match self.profile_ui_mode {
            ProfileUIMode::ProfileView => {
                match key.code {
                    KeyCode::Char('q') => {
                        return Ok(AppOutcome::Exit);
                    }
                    KeyCode::Esc => {
                        return Ok(AppOutcome::UpMenu);
                    }
                    // TODO: Add delete profile with confirmation.
                    KeyCode::Char('n') => {
                        self.profile_ui_mode = ProfileUIMode::ProfileEdit;
                        self.edit_profile = Some(EditableProfile::new(
                            &"".to_string(),
                            &Profile {
                                provider: "".to_string(),
                                model: "".to_string(),
                                additional_systems: vec![],
                                temperature: None,
                                context_limit: None,
                                max_tokens: None,
                                estimate_factor: None,
                            },
                        ));
                    }
                    KeyCode::Char('e') | KeyCode::Enter | KeyCode::Right => {
                        if self.has_profiles() {
                            self.profile_ui_mode = ProfileUIMode::ProfileEdit;
                            let (name, profile) = self.selected_profile().unwrap();
                            self.edit_profile = Some(EditableProfile::new(name, profile));
                        }
                    }
                    KeyCode::Down => {
                        self.profile_list_state.select_next();
                    }
                    KeyCode::Up => {
                        if self.profile_list_state.selected().is_some_and(|v| v == 0) {
                            return Ok(AppOutcome::UpMenu);
                        } else {
                            self.profile_list_state.select_previous();
                        }
                    }
                    _ => {}
                }
            }
            ProfileUIMode::ProfileEdit => {
                if let Some(edit_profile) = self.edit_profile.as_mut() {
                    if edit_profile.focussed_field == InputField::Provider {
                        if edit_profile.provider_drowdown_open {
                            match key.code {
                                KeyCode::Esc => {
                                    edit_profile.focussed_field = InputField::Model;
                                    edit_profile.provider_drowdown_open = false;
                                }
                                KeyCode::Down => {
                                    edit_profile.provider_list_state.select_next();
                                }
                                KeyCode::Up => {
                                    edit_profile.provider_list_state.select_previous();
                                }
                                KeyCode::Enter => {
                                    edit_profile.edited = true;
                                    let selected_provider =
                                        edit_profile.provider_list_state.selected().unwrap_or(0);
                                    let provider = provider_list()[selected_provider].clone();
                                    edit_profile.provider = provider.to_string();
                                    edit_profile.focussed_field = InputField::Model;
                                    edit_profile.provider_drowdown_open = false;
                                }
                                _ => {}
                            }
                        } else {
                            // provider dropdown not open
                            match key.code {
                                KeyCode::Esc => {
                                    self.profile_ui_mode = ProfileUIMode::ProfileView;
                                    self.edit_profile = None;
                                }
                                KeyCode::Down | KeyCode::Tab => {
                                    edit_profile.focussed_field =
                                        next_field(edit_profile.focussed_field.clone());
                                }
                                KeyCode::Up | KeyCode::BackTab => {
                                    edit_profile.focussed_field =
                                        prev_field(edit_profile.focussed_field.clone());
                                }
                                _ => {
                                    edit_profile.provider_drowdown_open = true;
                                    let index = provider_list()
                                        .iter()
                                        .position(|provider| provider == &edit_profile.provider)
                                        .unwrap_or(0);
                                    edit_profile.provider_list_state.select(Some(index));
                                }
                            }
                        }
                    } else {
                        // provider field not focussed.
                        match key.code {
                            KeyCode::Esc => {
                                self.profile_ui_mode = ProfileUIMode::ProfileView;
                                self.edit_profile = None;
                            }
                            KeyCode::Enter => {
                                // Change to save key
                                if let Some(edit_profile) = &self.edit_profile {
                                    // Check if a rename occurred
                                    let (name, _) = self.selected_profile().unwrap().clone();
                                    let name_clone = name.clone();
                                    if edit_profile.name.value() != name_clone {
                                        self.profiles.remove(&name_clone);
                                    }
                                    remove_profile(name_clone.as_str()).unwrap();

                                    let new_profile = Profile {
                                        provider: edit_profile.provider.clone(),
                                        model: edit_profile.model.value().to_string(),
                                        additional_systems: vec![],
                                        temperature: edit_profile.temperature.value().parse().ok(),
                                        context_limit: edit_profile
                                            .context_limit
                                            .value()
                                            .parse()
                                            .ok(),
                                        max_tokens: edit_profile.max_tokens.value().parse().ok(),
                                        estimate_factor: edit_profile
                                            .estimate_factor
                                            .value()
                                            .parse()
                                            .ok(),
                                    };
                                    self.profiles.insert(
                                        edit_profile.name.value().to_string(),
                                        new_profile.clone(),
                                    );
                                    save_profile(edit_profile.name.value(), new_profile).unwrap();

                                    self.profile_ui_mode = ProfileUIMode::ProfileView;
                                    self.edit_profile = None;
                                }
                            }
                            KeyCode::Down | KeyCode::Tab => {
                                edit_profile.focussed_field =
                                    next_field(edit_profile.focussed_field.clone());
                            }
                            KeyCode::Up | KeyCode::BackTab => {
                                edit_profile.focussed_field =
                                    prev_field(edit_profile.focussed_field.clone());
                            }
                            // Add cancel key
                            _ => {
                                edit_profile.edited = true;
                                if let Some(edit_profile) = self.edit_profile.as_mut() {
                                    match edit_profile.focussed_field {
                                        //TODO: validations
                                        InputField::Name => {
                                            edit_profile.name.handle_event(&Event::Key(key));
                                        }
                                        InputField::Provider => {
                                            // edit_profile.provider.handle_event(&Event::Key(key));
                                        }
                                        InputField::Model => {
                                            edit_profile.model.handle_event(&Event::Key(key));
                                        }
                                        InputField::Temperature => {
                                            edit_profile.temperature.handle_event(&Event::Key(key));
                                        }
                                        InputField::ContextLimit => {
                                            edit_profile
                                                .context_limit
                                                .handle_event(&Event::Key(key));
                                        }
                                        InputField::MaxTokens => {
                                            edit_profile.max_tokens.handle_event(&Event::Key(key));
                                        }
                                        InputField::EstimateFactor => {
                                            edit_profile
                                                .estimate_factor
                                                .handle_event(&Event::Key(key));
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
        Ok(AppOutcome::Continue)
    }

    /// Render the main area of the profile view.
    pub fn render_main_area(&mut self, f: &mut Frame, main_area: Rect, view_focussed: bool) {
        let profile_list_names: Vec<String> = self.profile_list_names();
        let has_profiles: bool = profile_list_names.len() > 0;

        let main_area_horizontal_chunks = chunks_for_list_and_view_split(main_area);
        render_left_list(
            f,
            "Profiles".to_string(),
            profile_list_names.clone(),
            &mut self.profile_list_state,
            view_focussed,
            main_area_horizontal_chunks[0],
        );

        // Main - Profile details area
        match self.profile_ui_mode {
            ProfileUIMode::ProfileView => {
                if has_profiles {
                    let (selected_profile_name, selected_profile) =
                        self.selected_profile().unwrap();
                    let profile_view = Paragraph::new(vec![
                        Line::from(vec![Span::styled(
                            "    Profile Details",
                            Style::default().add_modifier(Modifier::ITALIC),
                        )]),
                        Line::from(vec!["".into()]),
                        Line::from(vec![
                            "    Name:             ".into(),
                            selected_profile_name.clone().into(),
                        ]),
                        Line::from(vec![
                            "    Provider:         ".into(),
                            selected_profile.provider.clone().into(),
                        ]),
                        Line::from(vec![
                            "    Model:            ".into(),
                            selected_profile.model.clone().into(),
                        ]),
                        Line::from(vec![
                            "    Temperature:      ".into(),
                            selected_profile
                                .temperature
                                .clone()
                                .map_or("".into(), |temp| temp.to_string().into()),
                        ]),
                        Line::from(vec![
                            "    Context Limit:    ".into(),
                            selected_profile
                                .context_limit
                                .clone()
                                .map_or("".into(), |limit| limit.to_string().into()),
                        ]),
                        Line::from(vec![
                            "    Max Tokens:       ".into(),
                            selected_profile
                                .max_tokens
                                .clone()
                                .map_or("".into(), |tokens| tokens.to_string().into()),
                        ]),
                        Line::from(vec![
                            "    Estimate Factor:  ".into(),
                            selected_profile
                                .estimate_factor
                                .clone()
                                .map_or("".into(), |factor| factor.to_string().into()),
                        ]),
                    ])
                    .block(Block::default().borders(Borders::NONE));
                    f.render_widget(profile_view, main_area_horizontal_chunks[1]);
                } else {
                    let profile_view = Paragraph::new(vec![
                        Line::from(vec![Span::styled(
                            "    Profile Details",
                            Style::default().add_modifier(Modifier::ITALIC),
                        )]),
                        Line::from(vec!["".into()]),
                        Line::from(vec!["    Create a New Profile".into()]),
                    ])
                    .block(Block::default().borders(Borders::NONE));
                    f.render_widget(profile_view, main_area_horizontal_chunks[1]);
                }
            }
            ProfileUIMode::ProfileEdit => {
                let edit_section_chunks = Layout::default()
                    .direction(layout::Direction::Vertical)
                    .constraints([layout::Constraint::Length(2), layout::Constraint::Min(1)])
                    .split(main_area_horizontal_chunks[1]);

                let edit_header = Paragraph::new(vec![
                    Line::from(vec![Span::styled(
                        "    Edit Profile",
                        Style::default().add_modifier(Modifier::ITALIC),
                    )]),
                    Line::from(vec!["".into()]),
                ])
                .block(Block::default().borders(Borders::NONE));

                f.render_widget(edit_header, edit_section_chunks[0]);

                let edit_profile = self.edit_profile.as_ref().unwrap();
                let input_offset = 22;
                let lines = vec![
                    editable_profile_line(
                        "Name",
                        &edit_profile.name,
                        edit_profile
                            .errors
                            .get(&InputField::Name)
                            .cloned()
                            .flatten(),
                        input_offset,
                        edit_profile.focussed_field == InputField::Name,
                    ),
                    if edit_profile.focussed_field == InputField::Provider {
                        non_editable_dropdown_profile_line(
                            "Provider",
                            &edit_profile.provider,
                            None,
                            input_offset,
                            edit_profile.focussed_field == InputField::Provider,
                        )
                    } else {
                        non_editable_profile_line(
                            "Provider",
                            &edit_profile.provider,
                            None,
                            input_offset,
                            edit_profile.focussed_field == InputField::Provider,
                        )
                    },
                    editable_profile_line(
                        "Model",
                        &edit_profile.model,
                        edit_profile
                            .errors
                            .get(&InputField::Model)
                            .cloned()
                            .flatten(),
                        input_offset,
                        edit_profile.focussed_field == InputField::Model,
                    ),
                    editable_profile_line(
                        "Temperature",
                        &edit_profile.temperature,
                        None,
                        input_offset,
                        edit_profile.focussed_field == InputField::Temperature,
                    ),
                    editable_profile_line(
                        "Context Limit",
                        &edit_profile.context_limit,
                        None,
                        input_offset,
                        edit_profile.focussed_field == InputField::ContextLimit,
                    ),
                    editable_profile_line(
                        "Max Tokens",
                        &edit_profile.max_tokens,
                        None,
                        input_offset,
                        edit_profile.focussed_field == InputField::MaxTokens,
                    ),
                    editable_profile_line(
                        "Estimate Factor",
                        &edit_profile.estimate_factor,
                        None,
                        input_offset,
                        edit_profile.focussed_field == InputField::EstimateFactor,
                    ),
                ];
                let edit_profile_area_pos = edit_section_chunks[1].as_position();
                // let mut provider_popup: Option<ProviderPopup> = None;
                match edit_profile.focussed_field {
                    InputField::Name => {
                        f.set_cursor_position((
                            edit_profile_area_pos.x
                                + input_offset
                                + edit_profile.name.visual_cursor() as u16,
                            edit_profile_area_pos.y,
                        ));
                    }
                    InputField::Provider => {
                        f.set_cursor_position((
                            edit_profile_area_pos.x + input_offset + 0,
                            edit_profile_area_pos.y + 1,
                        ));
                        // provider_popup = Some(ProviderPopup{});
                    }
                    InputField::Model => {
                        f.set_cursor_position((
                            edit_profile_area_pos.x
                                + input_offset
                                + edit_profile.model.visual_cursor() as u16,
                            edit_profile_area_pos.y + 2,
                        ));
                    }
                    InputField::Temperature => {
                        f.set_cursor_position((
                            edit_profile_area_pos.x
                                + input_offset
                                + edit_profile.temperature.visual_cursor() as u16,
                            edit_profile_area_pos.y + 3,
                        ));
                    }
                    InputField::ContextLimit => {
                        f.set_cursor_position((
                            edit_profile_area_pos.x
                                + input_offset
                                + edit_profile.context_limit.visual_cursor() as u16,
                            edit_profile_area_pos.y + 4,
                        ));
                    }
                    InputField::MaxTokens => {
                        f.set_cursor_position((
                            edit_profile_area_pos.x
                                + input_offset
                                + edit_profile.max_tokens.visual_cursor() as u16,
                            edit_profile_area_pos.y + 5,
                        ));
                    }
                    InputField::EstimateFactor => {
                        f.set_cursor_position((
                            edit_profile_area_pos.x
                                + input_offset
                                + edit_profile.estimate_factor.visual_cursor() as u16,
                            edit_profile_area_pos.y + 6,
                        ));
                    }
                }
                let edit_profile_form =
                    Paragraph::new(lines).block(Block::default().borders(Borders::NONE));
                f.render_widget(edit_profile_form, edit_section_chunks[1]);

                if edit_profile.focussed_field == InputField::Provider
                    && edit_profile.provider_drowdown_open
                {
                    let target_area = Rect::new(
                        edit_profile_area_pos.x
                            + input_offset
                            + edit_profile.provider.len() as u16
                            + 2,
                        edit_profile_area_pos.y + 1,
                        17,
                        6,
                    );
                    f.render_widget(Clear::default(), target_area);
                    let block = Block::new().borders(Borders::ALL);
                    let provider_list = List::new(provider_list())
                        .highlight_symbol(" > ")
                        .highlight_style(Style::default().add_modifier(Modifier::BOLD))
                        .highlight_spacing(HighlightSpacing::Always)
                        .block(block);
                    f.render_stateful_widget(
                        provider_list,
                        target_area,
                        &mut self.edit_profile.as_mut().unwrap().provider_list_state,
                    );
                }
            }
        }
    }

    pub fn action_footer_names(&self) -> Vec<Span> {
        match self.profile_ui_mode {
            ProfileUIMode::ProfileView => vec![
                Span::raw("Profile"),
                Span::raw("[N] New"),
                Span::raw("[E] Edit"),
            ],
            ProfileUIMode::ProfileEdit => {
                if self.edit_profile.as_ref().unwrap().edited {
                    vec![
                        Span::raw("Profile"),
                        Span::styled(
                            "[Enter] Save",
                            Style::default().add_modifier(Modifier::BOLD),
                        ),
                        Span::raw("[Esc] Cancel"),
                    ]
                } else {
                    vec![
                        Span::raw("Profile"),
                        Span::raw("[Enter] Save"),
                        Span::raw("[Esc] Cancel"),
                    ]
                }
            }
        }
    }

    fn has_profiles(&self) -> bool {
        self.profiles.len() > 0
    }

    fn profile_list_names(&self) -> Vec<String> {
        let mut strs: Vec<String> = self.profiles.iter().map(|(name, _)| name.clone()).collect();
        strs.sort();
        strs
    }

    fn selected_profile(&self) -> Option<(&String, &Profile)> {
        let profile_names = self.profile_list_names();
        let target_profile_name = profile_names
            .get(self.profile_list_state.selected().unwrap_or(0))
            .unwrap();
        Some(
            self.profiles
                .iter()
                .find(|(name, _)| target_profile_name == *name)
                .map(|(name, profile)| (name, profile))
                .unwrap(),
        )
    }
}

/// Within the profile view, which mode the profile is in.
pub enum ProfileUIMode {
    ProfileView,
    ProfileEdit,
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum InputField {
    Name,
    Provider,
    Model,
    Temperature,
    ContextLimit,
    MaxTokens,
    EstimateFactor,
}

#[derive(Clone)]
pub struct EditableProfile {
    pub focussed_field: InputField,
    pub name: Input,
    pub provider: String,
    pub model: Input,
    pub temperature: Input,
    pub context_limit: Input,
    pub max_tokens: Input,
    pub estimate_factor: Input,
    pub errors: HashMap<InputField, Option<String>>,
    pub provider_drowdown_open: bool,
    pub provider_list_state: ListState,
    pub edited: bool,
}

impl EditableProfile {
    pub fn new(name: &String, profile: &Profile) -> Self {
        let temperature = profile.temperature.map_or_else(Input::default, |temp| {
            Input::default().with_value(temp.to_string())
        });
        let context_limit = profile.context_limit.map_or_else(Input::default, |limit| {
            Input::default().with_value(limit.to_string())
        });
        let max_tokens = profile.max_tokens.map_or_else(Input::default, |tokens| {
            Input::default().with_value(tokens.to_string())
        });
        let estimate_factor = profile
            .estimate_factor
            .map_or_else(Input::default, |factor| {
                Input::default().with_value(factor.to_string())
            });

        let mut it = Self {
            focussed_field: InputField::Name,
            name: Input::default().with_value(name.to_string()),
            provider: profile.provider.clone(),
            model: Input::default().with_value(profile.model.clone()),
            temperature,
            context_limit,
            max_tokens,
            estimate_factor,
            errors: HashMap::new(),
            provider_drowdown_open: false,
            provider_list_state: ListState::default(),
            edited: false,
        };

        it.validate();
        it
    }

    fn validate(&mut self) {
        let mut errors: HashMap<InputField, Option<String>> = HashMap::new();
        if self.name.value().is_empty() {
            errors.insert(InputField::Name, Some("Required".to_string()));
        } else {
            errors.insert(InputField::Name, None);
        }

        if self.model.value().is_empty() {
            errors.insert(InputField::Model, Some("Required".to_string()));
        } else {
            errors.insert(InputField::Model, None);
        }

        self.errors = errors;
    }

    fn is_valid(&self) -> bool {
        self.errors.iter().all(|(_, error)| error.is_none())
    }
}

fn next_field(current_field: InputField) -> InputField {
    match current_field {
        InputField::Name => InputField::Provider,
        InputField::Provider => InputField::Model,
        InputField::Model => InputField::Temperature,
        InputField::Temperature => InputField::ContextLimit,
        InputField::ContextLimit => InputField::MaxTokens,
        InputField::MaxTokens => InputField::EstimateFactor,
        InputField::EstimateFactor => InputField::Name,
    }
}

fn prev_field(current_field: InputField) -> InputField {
    match current_field {
        InputField::Name => InputField::EstimateFactor,
        InputField::Provider => InputField::Name,
        InputField::Model => InputField::Provider,
        InputField::Temperature => InputField::Model,
        InputField::ContextLimit => InputField::Temperature,
        InputField::MaxTokens => InputField::ContextLimit,
        InputField::EstimateFactor => InputField::MaxTokens,
    }
}

fn editable_profile_line<'a>(
    label: &'a str,
    input: &'a Input,
    error: Option<String>,
    input_offset: u16,
    focussed: bool,
) -> Line<'a> {
    let err_span = if let Some(err) = error {
        Span::styled(err.clone(), Style::default().fg(Color::Red))
    } else {
        Span::raw("".to_string())
    };
    let label_span = if focussed {
        Span::styled(label, Style::default().add_modifier(Modifier::BOLD))
    } else {
        Span::raw(label)
    };
    let prefix_spaces = " ".repeat(input_offset as usize - label.len() - 5);
    Line::from(vec![
        "    ".into(),
        label_span,
        ":".into(),
        prefix_spaces.into(),
        input.value().into(),
        "       ".into(),
        err_span,
    ])
}

fn non_editable_profile_line<'a>(
    label: &'a str,
    input: &'a str,
    error: Option<String>,
    input_offset: u16,
    focussed: bool,
) -> Line<'a> {
    let err_span = if let Some(err) = error {
        Span::styled(err.clone(), Style::default().fg(Color::Red))
    } else {
        Span::raw("".to_string())
    };
    let label_span = if focussed {
        Span::styled(label, Style::default().add_modifier(Modifier::BOLD))
    } else {
        Span::raw(label)
    };
    let prefix_spaces = " ".repeat(input_offset as usize - label.len() - 5);
    Line::from(vec![
        "    ".into(),
        label_span,
        ":".into(),
        prefix_spaces.into(),
        input.into(),
        "       ".into(),
        err_span,
    ])
}

fn non_editable_dropdown_profile_line<'a>(
    label: &'a str,
    input: &'a str,
    error: Option<String>,
    input_offset: u16,
    focussed: bool,
) -> Line<'a> {
    let err_span = if let Some(err) = error {
        Span::styled(err.clone(), Style::default().fg(Color::Red))
    } else {
        Span::raw("".to_string())
    };
    let label_span = if focussed {
        Span::styled(label, Style::default().add_modifier(Modifier::BOLD))
    } else {
        Span::raw(label)
    };
    let prefix_spaces = " ".repeat(input_offset as usize - label.len() - 5);
    Line::from(vec![
        "    ".into(),
        label_span,
        ":".into(),
        prefix_spaces.into(),
        input.into(),
        " ▼     ".into(),
        err_span,
    ])
}
