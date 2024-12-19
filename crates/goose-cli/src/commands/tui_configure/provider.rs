use std::{
    io,
    sync::{Arc, Mutex},
};

use crate::{
    commands::{
        configure::{get_recommended_model, get_required_keys, send_test_message},
        tui_configure::provider_list,
    },
    profile::Profile,
};
use goose::key_manager::{get_keyring_secret, KeyRetrievalStrategy};
use ratatui::{
    crossterm::event::{KeyCode, KeyEvent},
    layout::{self, Layout, Rect},
    text::{Line, Span},
    widgets::{Block, List, ListState, Paragraph},
    Frame,
};

use super::{
    main_area::{chunks_for_list_and_view_split, render_left_list},
    AppOutcome,
};

pub struct ProviderUi {
    provider_list_state: ListState,
    providers: Vec<ProviderWithState>,
    connection_test: Option<ConnectionTest>,
}

impl ProviderUi {
    pub fn new() -> Self {
        let providers = provider_states(provider_list(), false);
        Self {
            provider_list_state: ListState::default().with_selected(Some(0)),
            providers,
            connection_test: None,
        }
    }

    /// Draw provider ui for the main area
    pub fn render_main_area(&mut self, f: &mut Frame, area: Rect, view_focussed: bool) {
        let provider_names = self.renderable_provider_list();
        let main_area_horizontal_chunks = chunks_for_list_and_view_split(area);

        render_left_list(
            f,
            "Providers".to_string(),
            provider_names.clone(),
            &mut self.provider_list_state,
            view_focussed,
            main_area_horizontal_chunks[0],
        );

        // Render the right side
        let right_chunks = Layout::default()
            .direction(layout::Direction::Vertical)
            .constraints([layout::Constraint::Length(2), layout::Constraint::Min(1)])
            .split(main_area_horizontal_chunks[1]);

        // Render the connection test status
        if let Some(connection_test) = &self.connection_test {
            let current_status = match connection_test.status.lock().unwrap().clone() {
                ConnectionTestStatus::Pending => "Pending",
                ConnectionTestStatus::Success => "Success",
                ConnectionTestStatus::Failed => "Failed",
            };
            let status = Span::raw(format!("   Connection Test: {} ", current_status));
            if "Pending" == current_status {
                let pending_chunks = Layout::default()
                    .direction(layout::Direction::Horizontal)
                    .constraints([
                        layout::Constraint::Length(20),
                        layout::Constraint::Length(3),
                    ])
                    .split(right_chunks[0]);

                // Render status on the right side
                f.render_widget(
                    Paragraph::new(status).block(Block::default()),
                    pending_chunks[0],
                );

                // Render throbber on the left side
                let throbber = throbber_widgets_tui::Throbber::default();
                f.render_widget(throbber, pending_chunks[1]);
            } else {
                if let Some(error_message) = &*connection_test.error_message.lock().unwrap() {
                    let error_message = Paragraph::new(vec![
                        Line::from(status),
                        Line::from(vec![Span::raw("   "), Span::raw(error_message.clone())]),
                    ])
                    .block(Block::default());
                    f.render_widget(error_message, right_chunks[0]);
                } else {
                    f.render_widget(
                        Paragraph::new(status).block(Block::default()),
                        right_chunks[0],
                    );
                }
            }
        }

        let selected_provider = self
            .provider_list_state
            .selected()
            .map(|i| &self.providers[i]);
        if let Some(provider) = selected_provider {
            let provider_attributes: Vec<String> = provider.attributes.iter().map(|attr| {
                let source = match attr.source {
                    AttributeSource::Env => "Present in Env",
                    AttributeSource::Keyring => "Present in Keyring",
                    AttributeSource::Missing => "Missing",
                    AttributeSource::Pending => "Unknown: Press 'c' to check if present in keyring. You may be prompted by a keyring access request.",
                };
                format!("   {}: {}", attr.name, source)
            }).collect();
            let provider_attributes = List::new(provider_attributes).block(Block::default());
            f.render_widget(provider_attributes, right_chunks[1]);
        }
    }

    pub fn handle_events(&mut self, key: KeyEvent) -> io::Result<AppOutcome> {
        if self.connection_test.is_some() {
            // Interrupt with any key
            self.connection_test = None;
            return Ok(AppOutcome::Continue);
        }
        match key.code {
            KeyCode::Esc => return Ok(AppOutcome::UpMenu),
            KeyCode::Char('q') => return Ok(AppOutcome::Exit),
            KeyCode::Char('e') | KeyCode::Enter => {
                // Do stuff
            }
            KeyCode::Char('c') => {
                // TODO: Add some UI feedback that we checked things otherwise its instantaneous.
                let selected_provider = self
                    .provider_list_state
                    .selected()
                    .map(|i| &self.providers[i]);
                if let Some(provider) = selected_provider {
                    let provider_name = provider.name.clone();
                    let provider_attributes = provider_state(provider_name.clone(), true);
                    let provider_index = self
                        .providers
                        .iter()
                        .position(|p| p.name == provider_name)
                        .unwrap();
                    self.providers[provider_index].attributes = provider_attributes;
                }
            }
            KeyCode::Char('t') => {
                // TODO: Allow specifying a model
                self.connection_test = self.provider_list_state.selected().map(|i| {
                    ConnectionTest::new(Profile {
                        provider: self.providers[i].name.clone(),
                        model: get_recommended_model(&self.providers[i].name.clone()).to_string(),
                        additional_systems: vec![],
                        temperature: None,
                        context_limit: None,
                        max_tokens: None,
                        estimate_factor: None,
                    })
                });
                self.connection_test.as_mut().unwrap().start();
            }
            KeyCode::Down => {
                self.provider_list_state.select_next();
            }
            KeyCode::Up => {
                if self.provider_list_state.selected().is_some_and(|v| v == 0) {
                    return Ok(AppOutcome::UpMenu);
                } else {
                    self.provider_list_state.select_previous();
                }
            }
            _ => {}
        }
        Ok(AppOutcome::Continue)
    }

    pub fn action_footer_names(&self) -> Vec<Span> {
        if let Some(_) = &self.connection_test {
            vec![
                Span::raw("Provider"),
                Span::raw("Press any key to end connection test"),
            ]
        } else {
            vec![
                Span::raw("Provider"),
                Span::raw("[C] Check Configuration"),
                Span::raw("[T] Test Connection"),
            ] // TODO: Add edit config
        }
    }

    fn renderable_provider_list(&self) -> Vec<String> {
        self.providers
            .iter()
            .map(|provider| {
                if provider
                    .attributes
                    .iter()
                    .any(|attr| attr.source == AttributeSource::Pending)
                {
                    format!("{} {}", '?', provider.name)
                } else if provider
                    .attributes
                    .iter()
                    .any(|attr| attr.source == AttributeSource::Missing)
                {
                    format!("{} {}", '-', provider.name)
                } else {
                    format!("{} {}", '✔', provider.name)
                }
            })
            .collect()
    }
}

struct ConnectionTest {
    profile: Profile,
    status: Arc<Mutex<ConnectionTestStatus>>,
    error_message: Arc<Mutex<Option<String>>>,
    handle: Option<tokio::task::JoinHandle<()>>,
}

impl ConnectionTest {
    fn new(profile: Profile) -> Self {
        Self {
            profile,
            status: Arc::new(Mutex::new(ConnectionTestStatus::Pending)),
            error_message: Arc::new(Mutex::new(None)),
            handle: None,
        }
    }

    fn start(&mut self) {
        *self.status.lock().unwrap() = ConnectionTestStatus::Pending;
        let profile = self.profile.clone();
        let status = Arc::clone(&self.status);
        let error_message = Arc::clone(&self.error_message);

        self.handle = Some(tokio::spawn({
            async move {
                match send_test_message(profile).await {
                    Ok((message, _usage)) => {
                        let mut status = status.lock().unwrap();
                        let mut error_message = error_message.lock().unwrap();
                        if let Some(content) = message.content.first() {
                            if let Some(_) = content.as_text() {
                                *status = ConnectionTestStatus::Success;
                            } else {
                                *status = ConnectionTestStatus::Failed;
                                *error_message = Some("No response text available".to_string());
                            }
                        } else {
                            *status = ConnectionTestStatus::Failed;
                            *error_message = Some("No response content available".to_string());
                        }
                    }
                    Err(e) => {
                        let mut status = status.lock().unwrap();
                        let mut error_message = error_message.lock().unwrap();
                        *status = ConnectionTestStatus::Failed;
                        *error_message = Some(format!("{:?}", e));
                    }
                }
            }
        }));
    }
}

#[derive(Clone)]
enum ConnectionTestStatus {
    Pending,
    Success,
    Failed,
}

#[derive(PartialEq)]
enum AttributeSource {
    Env,
    Keyring,
    Missing,
    Pending,
}
struct AttributeState {
    name: String, // Attribute name
    source: AttributeSource,
}
struct ProviderWithState {
    name: String, // Provider name
    attributes: Vec<AttributeState>,
}

// We don't always check the keyring as it requires granting access to the keychain which might scare people without forewarning.
fn provider_states(
    provider_names: Vec<String>,
    also_check_keyring: bool,
) -> Vec<ProviderWithState> {
    provider_names
        .into_iter()
        .map(|name| ProviderWithState {
            name: name.clone(),
            attributes: provider_state(name, also_check_keyring),
        })
        .collect()
}

fn provider_state(name: String, also_check_keyring: bool) -> Vec<AttributeState> {
    get_required_keys(&name)
        .into_iter()
        .map(|key| {
            let source = if also_check_keyring
                && get_keyring_secret(key, KeyRetrievalStrategy::KeyringOnly).is_ok()
            {
                AttributeSource::Keyring
            } else if get_keyring_secret(key, KeyRetrievalStrategy::EnvironmentOnly).is_ok() {
                AttributeSource::Env
            } else {
                // If we haven't checked the keyring, we don't know if it's missing.
                if also_check_keyring {
                    AttributeSource::Missing
                } else {
                    AttributeSource::Pending
                }
            };
            AttributeState {
                name: key.to_string(),
                source,
            }
        })
        .collect()
}
