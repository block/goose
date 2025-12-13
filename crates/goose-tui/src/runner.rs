use crate::action_handler::handle_action;
use crate::app::App;
use crate::components::Component;
use crate::services::events::{Event, EventHandler};
use crate::state::action::Action;
use crate::state::AppState;
use crate::tui;
use anyhow::Result;
use goose_client::Client;
use tokio_stream::StreamExt;

fn needs_redraw(event: &Event, state: &AppState) -> bool {
    match event {
        Event::Tick => state.is_working,
        _ => true,
    }
}

pub async fn run_event_loop(
    mut terminal: tui::Tui,
    mut app: App<'_>,
    mut event_handler: EventHandler,
    mut state: AppState,
    client: Client,
) -> Result<()> {
    let tx = event_handler.sender();
    let c_tools = client.clone();
    let s_id = state.session_id.clone();
    let tx_tools = tx.clone();

    tokio::spawn(async move {
        if let Ok(tools) = c_tools.get_tools(&s_id).await {
            let _ = tx_tools.send(Event::ToolsLoaded(tools));
        }
    });

    if !state.messages.is_empty() {
        app.seed_input_history(&state.messages);
    }

    let mut reply_task: Option<tokio::task::JoinHandle<()>> = None;
    let mut should_redraw = true;

    loop {
        if state.needs_refresh {
            terminal.clear()?;
            state.needs_refresh = false;
            should_redraw = true;
        }

        if should_redraw {
            terminal.draw(|f| {
                app.render(f, f.area(), &state);
            })?;
            should_redraw = false;
        }

        let Some(event) = event_handler.next().await else {
            break;
        };

        if needs_redraw(&event, &state) {
            should_redraw = true;
        }

        if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
            break;
        }

        let mut quit = false;
        while let Some(event) = event_handler.try_next() {
            if needs_redraw(&event, &state) {
                should_redraw = true;
            }
            if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
                quit = true;
                break;
            }
        }
        if quit {
            break;
        }
    }

    Ok(())
}

pub async fn run_recipe_event_loop(
    mut terminal: tui::Tui,
    mut app: App<'_>,
    mut event_handler: EventHandler,
    mut state: AppState,
    client: Client,
    prompt: String,
) -> Result<()> {
    let tx = event_handler.sender();

    let user_message = goose::conversation::message::Message::user().with_text(&prompt);
    state.messages.push(user_message.clone());
    state.is_working = true;

    let client_clone = client.clone();
    let tx_clone = tx.clone();
    let session_id = state.session_id.clone();
    let messages_snapshot = state.messages.clone();

    tokio::spawn(async move {
        match client_clone.reply(messages_snapshot, session_id).await {
            Ok(mut stream) => {
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(msg) => {
                            let _ = tx_clone.send(Event::Server(std::sync::Arc::new(msg)));
                        }
                        Err(e) => {
                            let _ = tx_clone.send(Event::Error(e.to_string()));
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx_clone.send(Event::Error(e.to_string()));
            }
        }
    });

    let mut reply_task: Option<tokio::task::JoinHandle<()>> = None;

    loop {
        if state.needs_refresh {
            terminal.clear()?;
            state.needs_refresh = false;
        }

        terminal.draw(|f| {
            app.render(f, f.area(), &state);
        })?;

        let Some(event) = event_handler.next().await else {
            break;
        };

        if let Event::Input(key) = &event {
            if key.code == crossterm::event::KeyCode::Char('c')
                && key
                    .modifiers
                    .contains(crossterm::event::KeyModifiers::CONTROL)
            {
                break;
            }
        }

        if let Event::Server(msg) = &event {
            if let goose_server::routes::reply::MessageEvent::Finish { .. } = msg.as_ref() {
                crate::state::reducer::update(&mut state, Action::ServerMessage(msg.clone()));
                terminal.draw(|f| {
                    app.render(f, f.area(), &state);
                })?;
                break;
            }
        }

        if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
            break;
        }

        while let Some(event) = event_handler.try_next() {
            if let Event::Server(msg) = &event {
                if let goose_server::routes::reply::MessageEvent::Finish { .. } = msg.as_ref() {
                    crate::state::reducer::update(&mut state, Action::ServerMessage(msg.clone()));
                    terminal.draw(|f| {
                        app.render(f, f.area(), &state);
                    })?;
                    return Ok(());
                }
            }

            if process_event(event, &mut app, &mut state, &client, &tx, &mut reply_task) {
                break;
            }
        }
    }

    Ok(())
}

fn process_event(
    event: Event,
    app: &mut App,
    state: &mut AppState,
    client: &Client,
    tx: &tokio::sync::mpsc::UnboundedSender<Event>,
    reply_task: &mut Option<tokio::task::JoinHandle<()>>,
) -> bool {
    if let Ok(Some(action)) = app.handle_event(&event, state) {
        if handle_action(&action, state, client, tx, reply_task) {
            crate::state::reducer::update(state, action);
            return true;
        }
        let was_copy_mode = state.copy_mode;
        let should_seed_history = matches!(&action, Action::SessionResumed(_));
        crate::state::reducer::update(state, action);
        if should_seed_history {
            app.seed_input_history(&state.messages);
        }
        if state.copy_mode != was_copy_mode {
            let _ = tui::set_mouse_capture(!state.copy_mode);
        }
    }
    false
}
