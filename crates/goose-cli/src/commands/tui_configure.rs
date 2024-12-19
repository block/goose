use std::{
    cmp::max,
    hash::Hash,
    io::{self, stdout},
    panic::{set_hook, take_hook},
    time::Duration,
    vec,
};

use profile::ProfileUI;
use ratatui::{
    backend::{Backend, CrosstermBackend},
    crossterm::{
        event::{self, poll, Event, KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers},
        execute,
        terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    },
    layout::{self, Layout, Rect},
    style::{Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, Paragraph},
    Frame, Terminal,
};

mod main_area;
mod profile;
mod provider;

pub async fn handle_tui_configure() -> io::Result<()> {
    init_panic_hook();
    let mut tui = init_tui()?;

    run(tui).await
}

fn init_panic_hook() {
    let original_hook = take_hook();
    set_hook(Box::new(move |panic_info| {
        // intentionally ignore errors here since we're already in a panic
        let _ = restore_tui();
        original_hook(panic_info);
    }));
}

fn init_tui() -> io::Result<Terminal<impl Backend>> {
    enable_raw_mode()?;
    execute!(stdout(), EnterAlternateScreen)?;
    Terminal::new(CrosstermBackend::new(stdout()))
}

/// Restore the terminal to its original state in order to not have side effects on the terminal after the program exits.
fn restore_tui() -> io::Result<()> {
    disable_raw_mode()?;
    execute!(stdout(), LeaveAlternateScreen)?;
    Ok(())
}

/// Top level which view is in focus.
#[derive(Clone, PartialEq, Eq, Hash)]
enum UIMode {
    Profile,
    Provider,
}

enum AppOutcome {
    Continue,
    Exit,
    UpMenu, // Go up a menu level
}

struct App {
    main_menu_focussed: bool,
    ui_mode: UIMode,
    profile_ui: ProfileUI,
    provider_ui: provider::ProviderUi,
}

impl App {
    fn new() -> Self {
        Self {
            main_menu_focussed: false,
            ui_mode: UIMode::Profile,
            profile_ui: ProfileUI::new(),
            provider_ui: provider::ProviderUi::new(),
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        // Fit all the profile items and enough room to display their details including systems, just using dummy 14 for now.
        let profile_view_height = max(14, self.profile_ui.profiles.len() + 4) as u16;

        let vertical_chunks = Layout::default()
            .direction(layout::Direction::Vertical)
            // header, context title (Profiles), main display, footer
            .constraints([
                layout::Constraint::Length(1),
                layout::Constraint::Length(3),
                layout::Constraint::Length(profile_view_height),
                layout::Constraint::Min(1),
                layout::Constraint::Min(1),
            ])
            .split(f.area());

        let main_area = vertical_chunks[2];
        let footer_area = vertical_chunks[3];

        render_header(f, vertical_chunks[0]);
        self.render_main_menu(f, vertical_chunks[1]);

        // Main area
        match self.ui_mode {
            UIMode::Profile => {
                self.profile_ui
                    .render_main_area(f, main_area, !self.main_menu_focussed);
            }
            UIMode::Provider => {
                self.provider_ui
                    .render_main_area(f, main_area, !self.main_menu_focussed);
            }
        }

        // Footer
        // TODO: Provide the correct actions for the current mode.
        let actions: Vec<Span<'_>> = if self.main_menu_focussed {
            vec![Span::raw("Main Menu"), Span::raw("[Enter] Select")]
        } else {
            match self.ui_mode {
                UIMode::Profile => self.profile_ui.action_footer_names(),
                UIMode::Provider => self.provider_ui.action_footer_names(),
            }
        };
        render_footer(f, footer_area, &actions, self.main_menu_focussed);
    }

    fn handle_events(&mut self) -> io::Result<AppOutcome> {
        if poll(Duration::from_millis(100))? {
            if let Event::Key(key) = event::read()? {
                match key {
                    KeyEvent {
                        code: KeyCode::Char('c'),
                        modifiers: KeyModifiers::CONTROL,
                        kind: KeyEventKind::Press,
                        state: KeyEventState::NONE,
                    } => {
                        return Ok(AppOutcome::Exit);
                    }
                    _ => {}
                }
                if self.main_menu_focussed {
                    match key.code {
                        KeyCode::Char('q') => {
                            return Ok(AppOutcome::Exit);
                        }
                        KeyCode::Left => match self.ui_mode {
                            UIMode::Profile => {}
                            UIMode::Provider => {
                                self.ui_mode = UIMode::Profile;
                            }
                        },
                        KeyCode::Right => match self.ui_mode {
                            UIMode::Profile => {
                                self.ui_mode = UIMode::Provider;
                            }
                            UIMode::Provider => {}
                        },
                        KeyCode::Down | KeyCode::Enter => {
                            self.main_menu_focussed = false;
                        }
                        _ => {}
                    }
                } else {
                    match self.ui_mode {
                        UIMode::Provider => match self.provider_ui.handle_events(key)? {
                            AppOutcome::UpMenu => {
                                self.main_menu_focussed = true;
                                return Ok(AppOutcome::Continue);
                            }
                            o => {
                                return Ok(o);
                            }
                        },
                        UIMode::Profile => match self.profile_ui.handle_events(key)? {
                            AppOutcome::UpMenu => {
                                self.main_menu_focussed = true;
                                return Ok(AppOutcome::Continue);
                            }
                            o => {
                                return Ok(o);
                            }
                        },
                    }
                }
            }
        } else {
            // No event within polling timeout, continue to render.
            return Ok(AppOutcome::Continue);
        }
        Ok(AppOutcome::Continue)
    }

    fn render_main_menu(&mut self, f: &mut Frame, area: Rect) {
        let profiles_title = Span::styled(
            "Profiles",
            main_menu_item_style(self.main_menu_focussed, self.ui_mode == UIMode::Profile),
        );
        let providers_title = Span::styled(
            "Providers",
            main_menu_item_style(self.main_menu_focussed, self.ui_mode == UIMode::Provider),
        );
        let systems_title = Span::styled(
            "Systems (todo)",
            main_menu_item_style(self.main_menu_focussed, false),
        );
        f.render_widget(
            Paragraph::new(vec![
                Line::from(""),
                Line::from(vec![
                    Span::raw("   "),
                    profiles_title,
                    Span::raw("   "),
                    providers_title,
                    Span::raw("   "),
                    systems_title,
                ]),
                Line::from(vec![Span::raw(
                    "─".repeat((f.area().width as usize).saturating_sub(24)),
                )]),
                Line::from(""),
            ])
            .block(Block::default().borders(Borders::BOTTOM)),
            area,
        );
    }
}

async fn run(mut tui: Terminal<impl Backend>) -> io::Result<()> {
    let mut app = App::new();
    loop {
        tui.draw(|f| app.draw(f))?;
        match app.handle_events() {
            Ok(AppOutcome::Continue) | Ok(AppOutcome::UpMenu) => continue,
            Ok(AppOutcome::Exit) => break,
            Err(_) => break,
        }
    }
    restore_tui()?;
    Ok(())
}

fn render_header(f: &mut Frame, header_area: layout::Rect) {
    let title = Line::from(vec![
        Span::raw("─".repeat(10)),
        Span::styled(
            " Configure Goose ",
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]);
    f.render_widget(
        Block::default().borders(Borders::TOP).title(title),
        header_area,
    );
}

// Main menu functions
fn main_menu_item_style(main_menu_focussed: bool, is_selected: bool) -> Style {
    let mut style = Style::default();
    if main_menu_focussed {
        if is_selected {
            style = style.add_modifier(Modifier::BOLD | Modifier::UNDERLINED);
        } else {
            style = style.add_modifier(Modifier::BOLD);
        }
    } else if is_selected {
        style = style.add_modifier(Modifier::UNDERLINED);
    }
    style
}

// Profile functions

fn render_footer(
    f: &mut Frame,
    footer_area: layout::Rect,
    actions: &Vec<Span>,
    main_menu_focussed: bool,
) {
    let actions_prefix = vec![
        Span::raw(" ".repeat(3)),
        actions[0].clone().into(),
        Span::raw(":"),
        Span::raw(" ".repeat(3)),
    ];
    let actions_suffix = actions.iter().skip(1).fold(Vec::new(), |mut acc, action| {
        acc.push(action.clone());
        acc.push(Span::raw(" ".repeat(3)));
        acc
    });
    let mut app_nav_items = vec![
        Span::raw(" ".repeat(3)),
        Span::styled("App:       ", Style::default()),
        Span::styled("[Ctrl+C] Quit", Style::default()),
    ];
    if !main_menu_focussed {
        app_nav_items.push(Span::raw(" ".repeat(3)));
        app_nav_items.push(Span::styled("[Esc] Close Current Menu", Style::default()));
    }
    let footer = Text::from(vec![
        Line::from([actions_prefix, actions_suffix].concat()),
        Line::from(app_nav_items),
    ]);

    let title_line = Line::from(vec![
        Span::raw("─".repeat(10)),
        Span::styled(" Actions ", Style::default()),
    ]);
    let block = Block::default().borders(Borders::TOP).title(title_line);
    f.render_widget(Paragraph::new(footer).block(block), footer_area);
}

// Shared functions
pub fn provider_list() -> Vec<String> {
    vec![
        "anthropic".to_string(),
        "databricks".to_string(),
        "ollama".to_string(),
        "openai".to_string(),
    ]
}
