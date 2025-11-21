pub mod markdown;
pub mod theme;

use crate::app::{App, InputMode};
use crate::ui::markdown::MarkdownParser;
use goose::conversation::message::MessageContent;
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation,
};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    Frame,
};
use std::time::{Duration, Instant};

use ratatui::text::Text;

pub fn draw(f: &mut Frame, app: &mut App) {
    // Calculate input height based on content, with a minimum of 3 (1 line + borders) and max of 50% screen
    let input_lines = app.input.lines().len() as u16;
    let max_input_height = (f.area().height / 2).max(3);
    let input_height = (input_lines + 2).clamp(3, max_input_height);

    // Always show the todo/status line
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),               // Chat
            Constraint::Length(1),            // Todo/Loading/Version indicator
            Constraint::Length(input_height), // Input
            Constraint::Length(1),            // Status bar at bottom
        ])
        .split(f.area());

    draw_chat(f, app, chunks[0]);
    draw_todo_line(f, app, chunks[1]);
    draw_input(f, app, chunks[2]);
    draw_status(f, app, chunks[3]);

    // Slash Command Popup
    if app.input_mode == InputMode::Editing {
        if let Some(first_line) = app.input.lines().first() {
            if first_line.starts_with('/') {
                draw_slash_commands_popup(f, app, chunks[2], first_line);
            }
        }
    }

    if let Some(idx) = app.focused_message_index {
        draw_message_popup(f, app, idx);
    }

    if app.showing_todo_popup {
        draw_todo_popup_window(f, app);
    }

    if app.showing_help_popup {
        draw_help_popup(f, app);
    }

    if app.showing_about_popup {
        draw_about_popup(f, app);
    }

    if app.showing_command_builder {
        draw_command_builder_popup(f, app);
    }

    if app.showing_session_popup {
        draw_session_popup(f, app);
    }
}

fn draw_session_popup(f: &mut Frame, app: &mut App) {
    let area = centered_rect(60, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .title("Sessions (Enter to Resume, Esc to Close)")
        .style(Style::default().bg(app.config.theme.base.background));

    let items: Vec<ListItem> = app
        .available_sessions
        .iter()
        .map(|session| {
            let id_part = Span::styled(
                format!("{:<15}", session.id),
                Style::default().fg(Color::Cyan),
            );
            let count_part = Span::styled(
                format!("({} msgs)", session.message_count),
                Style::default().fg(Color::DarkGray),
            );
            let name_part = Span::styled(
                format!(" {}", session.name),
                Style::default().fg(Color::White),
            );

            ListItem::new(Line::from(vec![id_part, count_part, name_part]))
        })
        .collect();

    let list = List::new(items).block(block).highlight_style(
        Style::default()
            .bg(app.config.theme.base.selection)
            .add_modifier(Modifier::BOLD),
    );

    f.render_stateful_widget(list, area, &mut app.session_list_state);
}

fn draw_command_builder_popup(f: &mut Frame, app: &mut App) {
    let area = centered_rect(70, 60, f.area());
    f.render_widget(Clear, area);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(app.config.theme.base.background));

    match &app.builder_state {
        crate::app::BuilderState::SelectTool => {
            let title = "Create Command: Select Tool (Enter to pick)";
            let items: Vec<ListItem> = app
                .available_tools
                .iter()
                .map(|t| {
                    ListItem::new(vec![
                        Line::from(Span::styled(
                            &t.name,
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )),
                        Line::from(Span::styled(
                            &t.description,
                            Style::default().fg(Color::Gray),
                        )),
                    ])
                })
                .collect();

            let list = List::new(items).block(block.title(title)).highlight_style(
                Style::default()
                    .bg(app.config.theme.base.selection)
                    .add_modifier(Modifier::BOLD),
            );

            f.render_stateful_widget(list, area, &mut app.builder_list_state);
        }
        crate::app::BuilderState::ConfigureArgs {
            tool_idx,
            field_values,
            current_field_idx,
        } => {
            let tool = &app.available_tools[*tool_idx];
            let arg_name = &tool.parameters[*current_field_idx];

            let title = format!(
                "Configure '{}' (Arg {}/{})",
                tool.name,
                current_field_idx + 1,
                tool.parameters.len()
            );

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // History
                    Constraint::Length(3), // Input
                ])
                .margin(1)
                .split(area);

            f.render_widget(block.title(title), area);

            // 1. Render previous values
            let mut history_text = Vec::new();
            for (i, param) in tool.parameters.iter().enumerate() {
                if i < *current_field_idx {
                    if let Some(val) = field_values.get(param) {
                        history_text.push(Line::from(vec![
                            Span::styled(format!("{}: ", param), Style::default().fg(Color::Green)),
                            Span::styled(val, Style::default().fg(Color::White)),
                        ]));
                    }
                } else if i == *current_field_idx {
                    history_text.push(Line::from(vec![Span::styled(
                        format!("> {}: (Enter value below)", param),
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )]));
                } else {
                    history_text.push(Line::from(vec![Span::styled(
                        format!("{}: (pending)", param),
                        Style::default().fg(Color::DarkGray),
                    )]));
                }
            }
            f.render_widget(Paragraph::new(history_text), chunks[0]);

            // 2. Render Input
            app.builder_input.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title(format!("Value for '{}'", arg_name)),
            );
            f.render_widget(&app.builder_input, chunks[1]);
        }
        crate::app::BuilderState::NameCommand {
            tool_idx,
            field_values,
        } => {
            let tool = &app.available_tools[*tool_idx];
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(1),    // Summary
                    Constraint::Length(3), // Input
                ])
                .margin(1)
                .split(area);

            f.render_widget(block.title("Final Step: Name your Command"), area);

            // Summary
            let mut summary = vec![
                Line::from(Span::styled(
                    "Command Summary:",
                    Style::default().add_modifier(Modifier::UNDERLINED),
                )),
                Line::from(format!("Tool: {}", tool.name)),
                Line::from(""),
                Line::from("Arguments:"),
            ];
            for (k, v) in field_values {
                summary.push(Line::from(format!("  {}: {}", k, v)));
            }

            f.render_widget(Paragraph::new(summary), chunks[0]);

            // Input
            app.builder_input.set_block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_type(BorderType::Rounded)
                    .title("Slash Command Name (e.g. myls)"),
            );
            f.render_widget(&app.builder_input, chunks[1]);
        }
    }
}

fn draw_help_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 60, f.area());
    f.render_widget(Clear, area);

    let text = vec![
        Line::from(Span::styled(
            "Goose TUI Help",
            Style::default()
                .fg(Color::Yellow)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from(Span::styled(
            "Keybindings:",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from("  Enter       Send message / Select item"),
        Line::from("  Ctrl+J      Insert newline"),
        Line::from("  Esc         Switch to Normal mode / Close popups"),
        Line::from("  i           Switch to Editing mode"),
        Line::from("  j / k       Scroll / Navigate"),
        Line::from("  Ctrl+T      Toggle Todo List"),
        Line::from("  Ctrl+C      Interrupt / Clear / Quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Slash Commands:",
            Style::default().add_modifier(Modifier::UNDERLINED),
        )),
        Line::from("  /help       Show this help"),
        Line::from("  /about      About Goose TUI"),
        Line::from("  /theme      Change theme (e.g. /theme light)"),
        Line::from("  /clear      Clear chat history"),
        Line::from("  /exit       Quit"),
        Line::from(""),
        Line::from("  Custom commands can be defined in config."),
    ];

    let block = Block::default()
        .title("Help (Esc to Close)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(app.config.theme.base.background));

    f.render_widget(Paragraph::new(Text::from(text)).block(block), area);
}

fn draw_about_popup(f: &mut Frame, app: &App) {
    let area = centered_rect(40, 30, f.area());
    f.render_widget(Clear, area);

    let text = vec![
        Line::from(Span::styled(
            "Goose TUI",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("Version: 0.1.0"),
        Line::from("Powered by: Goose Agent & Ratatui"),
        Line::from(""),
        Line::from("A Gemini-inspired terminal interface"),
        Line::from("for the Goose AI agent."),
        Line::from(""),
        Line::from(Span::styled("Honk!", Style::default().fg(Color::Yellow))),
    ];

    let block = Block::default()
        .title("About (Esc to Close)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(app.config.theme.base.background));

    f.render_widget(
        Paragraph::new(Text::from(text))
            .block(block)
            .alignment(ratatui::layout::Alignment::Center),
        area,
    );
}

fn draw_slash_commands_popup(f: &mut Frame, app: &App, input_area: Rect, query: &str) {
    let mut commands = vec![
        "/exit", "/quit", "/help", "/about", "/theme", "/clear", "/alias", "/session",
    ];

    // Add custom commands
    let custom: Vec<String> = app
        .config
        .custom_commands
        .iter()
        .map(|c| format!("/{}", c.name))
        .collect();
    let custom_refs: Vec<&str> = custom.iter().map(|s| s.as_str()).collect();
    commands.extend(custom_refs);

    commands.sort();

    let filtered: Vec<&str> = commands
        .iter()
        .filter(|c| c.starts_with(query))
        .cloned()
        .collect();

    if filtered.is_empty() {
        return;
    }

    let height = (filtered.len() as u16 + 2).min(8); // Box height
    let width = 30;
    let area = Rect::new(
        input_area.x,
        input_area.y.saturating_sub(height),
        width,
        height,
    );

    f.render_widget(Clear, area);

    let items: Vec<ListItem> = filtered
        .iter()
        .map(|c| {
            ListItem::new(Span::styled(
                *c,
                Style::default().fg(app.config.theme.base.foreground),
            ))
        })
        .collect();

    let list = List::new(items)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .title("Commands"),
        )
        .style(Style::default().bg(app.config.theme.base.background)); // Match chat bg

    f.render_widget(list, area);
}

fn draw_todo_popup_window(f: &mut Frame, app: &App) {
    let area = centered_rect(60, 60, f.area());
    f.render_widget(Clear, area);

    let mut lines = Vec::new();
    for (text, done) in &app.todos {
        let (prefix, style) = if *done {
            ("[x] ", Style::default().fg(Color::DarkGray))
        } else {
            (
                "[ ] ",
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
        };

        lines.push(Line::from(vec![
            Span::styled(prefix, style),
            Span::styled(text, style),
        ]));
    }

    if lines.is_empty() {
        lines.push(Line::from(Span::styled(
            "No tasks yet.",
            Style::default().fg(Color::DarkGray),
        )));
    }

    let block = Block::default()
        .title("Todos (Ctrl+T/Esc to Close)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(app.config.theme.base.background));

    let paragraph = Paragraph::new(Text::from(lines))
        .block(block)
        .scroll((app.todo_scroll as u16, 0));

    f.render_widget(paragraph, area);
}

fn draw_message_popup(f: &mut Frame, app: &mut App, msg_idx: usize) {
    let area = centered_rect(80, 80, f.area());
    f.render_widget(Clear, area); // Clear background

    let message = &app.messages[msg_idx];
    let mut text_lines = Vec::new();

    match message.role {
        rmcp::model::Role::User => {
            for content in &message.content {
                if let MessageContent::Text(t) = content {
                    for line in t.text.lines() {
                        text_lines.push(Line::from(line));
                    }
                }
                // Handle ToolResponse in User role (standard MCP behavior)
                if let MessageContent::ToolResponse(resp) = content {
                    text_lines.push(Line::from(Span::styled(
                        "Tool Output:",
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )));
                    if let Ok(contents) = &resp.tool_result {
                        for content in contents {
                            if let rmcp::model::Content {
                                raw: rmcp::model::RawContent::Text(text_content),
                                ..
                            } = content
                            {
                                let text = &text_content.text;
                                // Try to pretty-print if it's JSON
                                let display_text = if let Ok(json_val) =
                                    serde_json::from_str::<serde_json::Value>(text)
                                {
                                    serde_json::to_string_pretty(&json_val)
                                        .unwrap_or_else(|_| text.to_string())
                                } else {
                                    text.to_string()
                                };

                                for line in display_text.lines() {
                                    text_lines.push(Line::from(line.to_string()));
                                }
                            }
                        }
                    }
                }
            }
        }
        rmcp::model::Role::Assistant => {
            for content in &message.content {
                match content {
                    MessageContent::Text(t) => {
                        text_lines.push(Line::from("Assistant Message:"));
                        for line in t.text.lines() {
                            text_lines.push(Line::from(line));
                        }
                    }
                    MessageContent::ToolRequest(req) => {
                        if let Ok(call) = &req.tool_call {
                            text_lines.push(Line::from(Span::styled(
                                format!("Tool Request: {}", call.name),
                                Style::default()
                                    .fg(Color::Yellow)
                                    .add_modifier(Modifier::BOLD),
                            )));
                            if let Some(args) = &call.arguments {
                                // Use to_string_pretty for arguments
                                let args_str =
                                    serde_json::to_string_pretty(args).unwrap_or_default();
                                for line in args_str.lines() {
                                    text_lines.push(Line::from(line.to_string()));
                                }
                            }
                        }
                    }
                    MessageContent::ToolResponse(resp) => {
                        text_lines.push(Line::from(Span::styled(
                            "Tool Output:",
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        )));
                        if let Ok(contents) = &resp.tool_result {
                            for content in contents {
                                if let rmcp::model::Content {
                                    raw: rmcp::model::RawContent::Text(text_content),
                                    ..
                                } = content
                                {
                                    let text = &text_content.text;
                                    // Try to pretty-print if it's JSON
                                    let display_text = if let Ok(json_val) =
                                        serde_json::from_str::<serde_json::Value>(text)
                                    {
                                        serde_json::to_string_pretty(&json_val)
                                            .unwrap_or_else(|_| text.to_string())
                                    } else {
                                        text.to_string()
                                    };

                                    for line in display_text.lines() {
                                        text_lines.push(Line::from(line.to_string()));
                                    }
                                }
                            }
                        }
                    }
                    MessageContent::Thinking(t) => {
                        text_lines.push(Line::from("Thinking:"));
                        for line in t.thinking.lines() {
                            text_lines.push(Line::from(line));
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    let block = Block::default()
        .title("Detailed View (Esc to Close, j/k to Scroll)")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .style(Style::default().bg(app.config.theme.base.background));

    // Calculate wrapped lines manually to determine total height
    // The area available for text is area.width - 2 (borders)
    // And area.height - 2 (borders)
    let text_width = area.width.saturating_sub(2) as usize;
    let mut total_lines = 0;

    // We need to iterate over text_lines and wrap them
    for line in &text_lines {
        // Ratatui doesn't easily expose wrapped height without rendering.
        // But we can estimate/calculate using textwrap or similar logic.
        // However, `text_lines` here are ratatui::text::Line which may contain multiple spans.
        // A simple approximation is sufficient or re-using the logic used elsewhere.
        // Paragraph wraps by words.

        // Simple approach: Sum the length of content string and divide by width.
        // This is imperfect for rich text but good enough for scroll limits.
        let content: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
        if content.is_empty() {
            total_lines += 1;
        } else {
            let wrapped = textwrap::wrap(&content, text_width);
            total_lines += wrapped.len();
        }
    }

    app.popup_content_height = total_lines;
    app.popup_area_height = area.height.saturating_sub(2) as usize;

    let paragraph = Paragraph::new(Text::from(text_lines))
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: false }) // Enable wrapping
        .scroll((app.popup_scroll as u16, 0));

    f.render_widget(paragraph, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}

fn draw_todo_line(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();

    // Check for flash message
    let mut showing_flash = false;
    if let (Some(msg), Some(expiry)) = (&app.flash_message, app.flash_message_expiry) {
        if Instant::now() < expiry {
            showing_flash = true;
            spans.push(Span::styled(
                msg,
                Style::default()
                    .fg(app.config.theme.status.warning)
                    .add_modifier(Modifier::BOLD),
            ));
        }
    }

    if !showing_flash {
        if app.waiting_for_response {
            // Animated spinner when working
            let spinner_frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
            let spinner = spinner_frames[(app.animation_frame / 4) % spinner_frames.len()];

            spans.push(Span::styled(
                format!("{} ", spinner),
                Style::default()
                    .fg(app.config.theme.status.thinking)
                    .add_modifier(Modifier::BOLD),
            ));

            if !app.todos.is_empty() {
                // Show active todo while working
                let active_task = app
                    .todos
                    .iter()
                    .find(|(_, done)| !*done)
                    .map(|(text, _)| text.as_str())
                    .unwrap_or("Done!");
                let total = app.todos.len();
                let completed = app.todos.iter().filter(|(_, done)| *done).count();

                spans.push(Span::styled(
                    format!("{} ({}/{}) ", active_task, completed, total),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            } else {
                // Show goose puns when no todos
                let puns = [
                    "Honking at the mainframe...",
                    "Chasing bugs (and breadcrumbs)...",
                    "Migrating data south...",
                    "Deploying the golden egg...",
                    "Flapping wings at warp speed...",
                    "Waddling through the code...",
                    "Goose is loose in the system...",
                    "Compiling feathers...",
                    "Synthesizing honks...",
                    "Calculating flight path...",
                    "Optimizing the gaggle...",
                    "Hacking the breadbox...",
                    "In a wild goose chase for answers...",
                    "Syncing with the flock...",
                    "Preening the pixels...",
                    "Navigating the digital pond...",
                    "Gathering intelligence (and seeds)...",
                    "Formatting the V-formation...",
                    "Decoding the Matrix (it's all corn)...",
                    "System Status: HONK.",
                ];
                // Change pun every ~10 seconds (10s / 30ms tick ~= 333 frames)
                let pun_idx = (app.animation_frame / 333) % puns.len();
                spans.push(Span::styled(
                    puns[pun_idx],
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        } else {
            // Static loading icon when not working
            spans.push(Span::styled(
                "⠿ ",
                Style::default()
                    .fg(app.config.theme.status.thinking)
                    .add_modifier(Modifier::BOLD),
            ));

            if !app.todos.is_empty() {
                // Show todos when not working
                let total = app.todos.len();
                let completed = app.todos.iter().filter(|(_, done)| *done).count();
                let active_task = app
                    .todos
                    .iter()
                    .find(|(_, done)| !*done)
                    .map(|(text, _)| text.as_str())
                    .unwrap_or("All tasks completed!");

                spans.push(Span::styled(
                    format!("{} ({}/{}) ", active_task, completed, total),
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            } else if !app.has_worked {
                // Show version initially before goose has worked
                spans.push(Span::styled(
                    "goose 1.14.0",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            } else {
                // Show waiting message after goose has worked
                spans.push(Span::styled(
                    "Waiting for user input...",
                    Style::default()
                        .fg(Color::Gray)
                        .add_modifier(Modifier::ITALIC),
                ));
            }
        }
    }

    let block = Block::default().style(Style::default());
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

fn draw_status(f: &mut Frame, app: &App, area: Rect) {
    let mut spans = Vec::new();
    let text_color = app.config.theme.base.foreground; // Common color for session, tokens, and CWD

    // 1. Mode Indicator
    let mode_bg_color = if app.waiting_for_response {
        app.config.theme.status.thinking
    } else {
        app.config.theme.base.border_active
    };

    if app.waiting_for_response {
        spans.push(Span::styled(
            " WORKING ",
            Style::default()
                .bg(mode_bg_color)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ));
    } else {
        let mode_str = match app.input_mode {
            InputMode::Normal => "NORMAL",
            InputMode::Editing => "EDITING",
        };
        spans.push(Span::styled(
            format!(" {} ", mode_str),
            Style::default()
                .bg(mode_bg_color)
                .fg(Color::Black)
                .add_modifier(Modifier::BOLD),
        ));
    }

    spans.push(Span::raw(" "));

    // 2. Session ID (no "Session:" prefix, same color as CWD)
    spans.push(Span::styled(
        &app.session_id,
        Style::default().fg(text_color),
    ));
    spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));

    // 3. Token usage (always show, same color as CWD, using model_context_limit for denominator)
    // Format tokens with K suffix if > 1000
    let format_tokens = |tokens: i32| -> String {
        if tokens >= 1000 {
            format!("{}k", tokens / 1000)
        } else {
            tokens.to_string()
        }
    };

    spans.push(Span::styled(
        format!(
            "{}/{}",
            format_tokens(app.token_state.total_tokens),
            format_tokens(app.model_context_limit as i32)
        ),
        Style::default().fg(text_color),
    ));
    spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));

    // 4. Current Working Directory (less aggressive truncation)
    if let Ok(cwd) = std::env::current_dir() {
        if let Some(cwd_str) = cwd.to_str() {
            // Increase max path length to be less aggressive with truncation
            let max_path_len = 50;
            let display_path = if cwd_str.len() > max_path_len {
                format!("...{}", &cwd_str[cwd_str.len() - max_path_len + 3..])
            } else {
                cwd_str.to_string()
            };
            spans.push(Span::styled(display_path, Style::default().fg(text_color)));
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }
    }

    // 5. Dynamic Key Hints
    let mut hints = Vec::new();

    // Ctrl+C Hint
    if !app.input.is_empty() {
        hints.push(Span::styled(
            "Ctrl+C Clear",
            Style::default().fg(Color::DarkGray),
        ));
    } else if app.waiting_for_response {
        hints.push(Span::styled(
            "Ctrl+C Interrupt",
            Style::default().fg(app.config.theme.status.warning),
        ));
    } else {
        hints.push(Span::styled(
            "Ctrl+C Quit",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Esc Hint
    if app.focused_message_index.is_some() || app.showing_todo_popup {
        hints.push(Span::styled(
            "Esc Close",
            Style::default().fg(Color::DarkGray),
        ));
    } else if app.input_mode == InputMode::Editing {
        hints.push(Span::styled(
            "Esc Normal",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // 'i' Hint
    if app.input_mode == InputMode::Normal {
        hints.push(Span::styled("i Edit", Style::default().fg(Color::DarkGray)));
        hints.push(Span::styled(
            "j/k Scroll",
            Style::default().fg(Color::DarkGray),
        ));
    }

    if app.focused_message_index.is_some() || app.showing_todo_popup {
        hints.push(Span::styled(
            "j/k Scroll",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Enter Hint
    if app.input_mode == InputMode::Editing {
        hints.push(Span::styled(
            "Enter Send",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Ctrl+J Hint
    if app.input_mode == InputMode::Editing {
        hints.push(Span::styled(
            "Ctrl+J Newline",
            Style::default().fg(Color::DarkGray),
        ));
    }

    // Ctrl+T Hint
    hints.push(Span::styled(
        "Ctrl+T Todos",
        Style::default().fg(Color::DarkGray),
    ));

    // Combine hints with separators
    for (i, hint_span) in hints.into_iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" | ", Style::default().fg(Color::DarkGray)));
        }
        spans.push(hint_span);
    }

    let block = Block::default().style(Style::default().bg(app.config.theme.base.selection));
    f.render_widget(Paragraph::new(Line::from(spans)).block(block), area);
}

fn draw_chat(f: &mut Frame, app: &mut App, area: Rect) {
    app.visual_line_to_message_index.clear(); // Clear mapping
    app.selectable_indices.clear(); // Clear selectables
    let mut list_items = Vec::new();
    // Calculate available width for text (Area width - 2 for borders)
    let content_width = area.width.saturating_sub(2) as usize;

    // Track the last seen tool call to provide context for the response
    let mut last_tool_call: Option<(String, String)> = None;

    for (msg_idx, message) in app.messages.iter().enumerate() {
        match message.role {
            rmcp::model::Role::User => {
                for content in &message.content {
                    match content {
                        MessageContent::Text(t) => {
                            // User message with left border line (cleaner, less boxy)
                            let border_color = Color::DarkGray;

                            // Wrap text, accounting for the "│ " prefix
                            let wrapped_lines =
                                textwrap::wrap(&t.text, content_width.saturating_sub(2));

                            for line_str in wrapped_lines {
                                let line = Line::from(vec![
                                    Span::styled("│ ", Style::default().fg(border_color)),
                                    Span::styled(
                                        line_str.to_string(),
                                        Style::default()
                                            .fg(app.config.theme.base.foreground)
                                            .add_modifier(Modifier::ITALIC),
                                    ),
                                ]);
                                list_items.push(ListItem::new(line));
                                app.visual_line_to_message_index.push(msg_idx);
                            }
                        }
                        // Handle other content types if needed (e.g. ToolResponse if they appear here)
                        MessageContent::ToolResponse(resp) => {
                            // Mark header as selectable
                            app.selectable_indices.push(list_items.len());

                            // Determine Header Info
                            let (tool_name, tool_args) = last_tool_call
                                .clone()
                                .unwrap_or(("Unknown".to_string(), "".to_string()));

                            let is_success = resp.tool_result.is_ok();
                            let color = if is_success { Color::Green } else { Color::Red };

                            // Format args: truncate if too long
                            let max_args_len = 50;
                            let display_args = if tool_args.len() > max_args_len {
                                format!("{}...", &tool_args[..max_args_len])
                            } else {
                                tool_args
                            };

                            let header_content = format!("{} {}", tool_name, display_args);

                            // Calculate padding for the right side to make it a full width box header
                            // Width - 2 (borders) - 2 (prefix) - content_len - 1 (space) - 1 (┐) ??
                            // Actually: "┌─ CONTENT ─...─┐"
                            // We want: "┌─ " + content + " " + "─"*padding + "┐"
                            let fixed_chars = 5; // "┌─ " + " " + "┐"
                            let padding_len =
                                content_width.saturating_sub(header_content.len() + fixed_chars);

                            let header_spans = vec![
                                Span::styled("╭─ ", Style::default().fg(Color::DarkGray)),
                                Span::styled(
                                    tool_name,
                                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                                ),
                                Span::styled(
                                    format!(" {} ", display_args),
                                    Style::default().fg(Color::Gray),
                                ),
                                Span::styled(
                                    format!("{:─<width$}╮", "", width = padding_len),
                                    Style::default().fg(Color::DarkGray),
                                ),
                            ];
                            list_items.push(ListItem::new(Line::from(header_spans)));
                            app.visual_line_to_message_index.push(msg_idx);

                            // Content
                            if let Ok(contents) = &resp.tool_result {
                                let mut line_count = 0;
                                let max_lines = 10;

                                for content in contents {
                                    if let rmcp::model::Content {
                                        raw: rmcp::model::RawContent::Text(text_content),
                                        ..
                                    } = content
                                    {
                                        for line in text_content.text.lines() {
                                            if line_count >= max_lines {
                                                break;
                                            }
                                            let line_width = content_width.saturating_sub(4);
                                            let truncated = if line.len() > line_width {
                                                &line[..line_width]
                                            } else {
                                                line
                                            };
                                            let content_str = format!("│ {}", truncated);
                                            let padding = content_width
                                                .saturating_sub(content_str.chars().count() + 1);
                                            let box_line = format!(
                                                "{}{: <width$}│",
                                                content_str,
                                                "",
                                                width = padding
                                            );

                                            list_items.push(ListItem::new(Line::from(
                                                Span::styled(
                                                    box_line,
                                                    Style::default().fg(Color::Gray),
                                                ),
                                            )));
                                            app.visual_line_to_message_index.push(msg_idx);
                                            line_count += 1;
                                        }
                                    }
                                }

                                if line_count >= max_lines {
                                    let content = "│ ... (output truncated)";
                                    let padding =
                                        content_width.saturating_sub(content.chars().count() + 1);
                                    let box_line =
                                        format!("{}{: <width$}│", content, "", width = padding);
                                    list_items.push(ListItem::new(Line::from(Span::styled(
                                        box_line,
                                        Style::default()
                                            .fg(Color::DarkGray)
                                            .add_modifier(Modifier::ITALIC),
                                    ))));
                                    app.visual_line_to_message_index.push(msg_idx);
                                }
                            }

                            // Footer with rounded corners
                            let footer = format!(
                                "╰{:─<width$}╯",
                                "",
                                width = content_width.saturating_sub(2)
                            );
                            list_items.push(ListItem::new(Line::from(Span::styled(
                                footer,
                                Style::default().fg(Color::DarkGray),
                            ))));
                            app.visual_line_to_message_index.push(msg_idx);
                        }
                        // For now, falling back to nothing or we can add ToolResponse handler here if it's missing.
                        _ => {}
                    }
                }
            }
            rmcp::model::Role::Assistant => {
                // Assistant messages: Render Markdown directly, no prefix
                for content in &message.content {
                    match content {
                        MessageContent::Text(t) => {
                            let markdown = MarkdownParser::parse(&t.text, content_width);
                            for line in markdown.lines {
                                list_items.push(ListItem::new(line));
                                app.visual_line_to_message_index.push(msg_idx);
                            }
                        }
                        MessageContent::ToolRequest(req) => {
                            if let Ok(call) = &req.tool_call {
                                // Capture info for next response
                                let name = call.name.clone();
                                let args = if let Some(a) = &call.arguments {
                                    // Flatten: key=val
                                    let mut pairs = Vec::new();
                                    for (k, v) in a {
                                        let val_str = match v {
                                            serde_json::Value::String(s) => s.clone(),
                                            _ => v.to_string(),
                                        };
                                        pairs.push(format!("{}={}", k, val_str));
                                    }
                                    pairs.join(", ")
                                } else {
                                    "".to_string()
                                };
                                last_tool_call = Some((name.to_string(), args));

                                // Mark header as selectable
                                app.selectable_indices.push(list_items.len());

                                // Collapsed view
                                let tool_line = Line::from(vec![
                                    Span::styled("▶ Tool: ", Style::default().fg(Color::Yellow)),
                                    Span::styled(
                                        format!("{} (args hidden)", name),
                                        Style::default().fg(Color::White),
                                    ),
                                ]);
                                list_items.push(ListItem::new(tool_line));
                                app.visual_line_to_message_index.push(msg_idx);
                            }
                        }
                        MessageContent::ToolResponse(_) => {
                            // Should not be here in Assistant role usually, or handled in User.
                            // Ignoring to avoid duplication.
                        }
                        MessageContent::Thinking(t) => {
                            list_items.push(ListItem::new(Line::from(vec![
                                Span::styled("🤔 ", Style::default()),
                                Span::styled(
                                    &t.thinking,
                                    Style::default()
                                        .fg(Color::DarkGray)
                                        .add_modifier(Modifier::ITALIC),
                                ),
                            ])));
                            app.visual_line_to_message_index.push(msg_idx);
                        }
                        _ => {}
                    }
                }
            }
        }

        // Spacer
        list_items.push(ListItem::new(Line::from("")));
        app.visual_line_to_message_index.push(msg_idx); // Spacer belongs to the message
    }

    if app.has_user_input_pending {
        let blink_char = if (app.animation_frame / 10) % 2 == 0 {
            "_"
        } else {
            " "
        };
        list_items.push(ListItem::new(Line::from(Span::styled(
            format!(">{}", blink_char),
            Style::default().fg(Color::DarkGray),
        ))));
        // No message mapping for cursor line, or map to last message?
        // It doesn't matter much as it's not selectable.
        if !app.messages.is_empty() {
            app.visual_line_to_message_index
                .push(app.messages.len() - 1);
        }
    }

    // Use thinking color for border when working, otherwise use normal border
    let border_color = if app.waiting_for_response {
        app.config.theme.status.thinking
    } else {
        app.config.theme.base.border
    };

    let mut messages_list = List::new(list_items.clone())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::default().fg(border_color)),
        )
        .style(
            Style::default().fg(app.config.theme.base.foreground).bg(app
                .config
                .theme
                .base
                .background),
        );

    if app.input_mode == InputMode::Normal {
        messages_list = messages_list.highlight_style(
            Style::default()
                .bg(app.config.theme.base.selection)
                .add_modifier(Modifier::BOLD),
        );
    }

    if app.auto_scroll {
        app.scroll_state
            .select(Some(list_items.len().saturating_sub(1)));
    }

    f.render_stateful_widget(messages_list, area, &mut app.scroll_state);

    // Render scrollbar
    app.vertical_scroll_state = app
        .vertical_scroll_state
        .content_length(list_items.len())
        .position(app.scroll_state.selected().unwrap_or(0));

    if let Some(last) = app.last_scroll_time {
        if last.elapsed() < Duration::from_secs(1) {
            f.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight)
                    .begin_symbol(None)
                    .end_symbol(None),
                area,
                &mut app.vertical_scroll_state,
            );
        }
    }
}

fn draw_input(f: &mut Frame, app: &mut App, area: Rect) {
    let (r, g, b) = app.current_border_color;
    let border_style = Style::default().fg(Color::Rgb(r, g, b));

    app.input.set_block(
        Block::default()
            .borders(Borders::ALL)
            .border_type(BorderType::Rounded)
            .title("Message")
            .border_style(border_style),
    );

    app.input.set_style(
        Style::default()
            .fg(app.config.theme.base.foreground)
            .bg(app.config.theme.base.background),
    );

    f.render_widget(&app.input, area);
}
