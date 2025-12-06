use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::AppState;
use goose_client::Client;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

pub fn handle_action(
    action: &Action,
    state: &AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
    reply_task: &mut Option<tokio::task::JoinHandle<()>>,
) -> bool {
    match action {
        Action::SendMessage(message_to_send) => {
            handle_send_message(message_to_send, state, client, tx, reply_task);
        }
        Action::ResumeSession(id) => {
            handle_resume_session(id, client, tx);
        }
        Action::CreateNewSession => {
            handle_create_new_session(client, tx);
        }
        Action::OpenSessionPicker => {
            handle_open_session_picker(client, tx);
        }
        Action::OpenConfig => {
            handle_open_config(client, tx);
        }
        Action::FetchModels(provider) => {
            handle_fetch_models(provider, client, tx);
        }
        Action::UpdateProvider { provider, model } => {
            handle_update_provider(provider, model, state, client, tx);
        }
        Action::ToggleExtension { name, enabled } => {
            handle_toggle_extension(name, *enabled, state, client, tx);
        }
        Action::ForkFromMessage(msg_idx) => {
            handle_fork_from_message(*msg_idx, state, client, tx);
        }
        Action::SetGooseMode(mode) => {
            handle_set_goose_mode(mode, client, tx);
        }
        Action::ConfirmToolCall { id, approved } => {
            handle_confirm_tool_call(id, *approved, state, client, tx);
        }
        Action::Quit => {
            return true;
        }
        Action::Interrupt => {
            if let Some(task) = reply_task.take() {
                task.abort();
            }
        }
        _ => {}
    }
    false
}

fn handle_send_message(
    message_to_send: &goose::conversation::message::Message,
    state: &AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
    reply_task: &mut Option<tokio::task::JoinHandle<()>>,
) {
    let client = client.clone();
    let tx = tx.clone();
    let mut messages_snapshot = state.messages.clone();
    messages_snapshot.push(message_to_send.clone());
    let session_id = state.session_id.clone();

    let task = tokio::spawn(async move {
        match client.reply(messages_snapshot, session_id).await {
            Ok(mut stream) => {
                while let Some(result) = stream.next().await {
                    match result {
                        Ok(msg) => {
                            let _ = tx.send(Event::Server(std::sync::Arc::new(msg)));
                        }
                        Err(e) => {
                            let _ = tx.send(Event::Error(e.to_string()));
                        }
                    }
                }
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e.to_string()));
            }
        }
    });
    *reply_task = Some(task);
}

fn handle_resume_session(id: &str, client: &Client, tx: &mpsc::UnboundedSender<Event>) {
    let client = client.clone();
    let tx = tx.clone();
    let id = id.to_string();
    tokio::spawn(async move {
        match client.resume_agent(&id).await {
            Ok(s) => {
                let _ = tx.send(Event::SessionResumed(Box::new(s)));
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e.to_string()));
            }
        }
    });
}

fn handle_create_new_session(client: &Client, tx: &mpsc::UnboundedSender<Event>) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let cwd = std::env::current_dir().unwrap_or_default();
        match client.start_agent(cwd.to_string_lossy().to_string()).await {
            Ok(s) => {
                crate::configure_session_from_global(&client, &s.id).await;
                let _ = tx.send(Event::SessionResumed(Box::new(s)));
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e.to_string()));
            }
        }
    });
}

fn handle_open_session_picker(client: &Client, tx: &mpsc::UnboundedSender<Event>) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.list_sessions().await {
            Ok(sessions) => {
                let _ = tx.send(Event::SessionsList(sessions));
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e.to_string()));
            }
        }
    });
}

fn handle_open_config(client: &Client, tx: &mpsc::UnboundedSender<Event>) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.get_providers().await {
            Ok(providers) => {
                let _ = tx.send(Event::ProvidersLoaded(providers));
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e.to_string()));
            }
        }
        match client.get_extensions().await {
            Ok(extensions) => {
                let _ = tx.send(Event::ExtensionsLoaded(extensions));
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e.to_string()));
            }
        }
        match client.read_config().await {
            Ok(config) => {
                let _ = tx.send(Event::ConfigLoaded(config));
            }
            Err(e) => {
                let _ = tx.send(Event::Error(e.to_string()));
            }
        }
    });
}

fn handle_fetch_models(provider: &str, client: &Client, tx: &mpsc::UnboundedSender<Event>) {
    let client = client.clone();
    let tx = tx.clone();
    let p = provider.to_string();
    tokio::spawn(async move {
        match client.get_provider_models(&p).await {
            Ok(models) => {
                let _ = tx.send(Event::ModelsLoaded {
                    provider: p,
                    models,
                });
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models for {}: {}", p, e);
            }
        }
    });
}

fn handle_update_provider(
    provider: &str,
    model: &str,
    state: &AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
) {
    let client = client.clone();
    let tx = tx.clone();
    let session_id = state.session_id.clone();
    let p = provider.to_string();
    let m = model.to_string();
    tokio::spawn(async move {
        if let Err(e) = client
            .update_provider(&session_id, p.clone(), Some(m.clone()))
            .await
        {
            let _ = tx.send(Event::Error(format!(
                "Failed to update session provider: {e}"
            )));
            return;
        }
        if let Err(e) = client
            .upsert_config("GOOSE_PROVIDER", serde_json::json!(p), false)
            .await
        {
            let _ = tx.send(Event::Error(format!(
                "Failed to update config provider: {e}"
            )));
        }
        if let Err(e) = client
            .upsert_config("GOOSE_MODEL", serde_json::json!(m), false)
            .await
        {
            let _ = tx.send(Event::Error(format!("Failed to update config model: {e}")));
        }
    });
}

fn handle_toggle_extension(
    name: &str,
    enabled: bool,
    state: &AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
) {
    let client = client.clone();
    let tx = tx.clone();
    let session_id = state.session_id.clone();
    let ext_name = name.to_string();

    let ext_config = state
        .extensions
        .iter()
        .find(|e| e.config.name() == ext_name)
        .map(|e| e.config.clone());

    if let Some(config) = ext_config {
        tokio::spawn(async move {
            if enabled {
                if let Err(e) = client.add_extension(&session_id, config.clone()).await {
                    let _ = tx.send(Event::Error(format!(
                        "Failed to enable extension in session: {e}"
                    )));
                }
                if let Err(e) = client.add_config_extension(ext_name, config, true).await {
                    let _ = tx.send(Event::Error(format!(
                        "Failed to enable extension in config: {e}"
                    )));
                }
            } else {
                if let Err(e) = client.remove_extension(&session_id, &ext_name).await {
                    let _ = tx.send(Event::Error(format!(
                        "Failed to disable extension in session: {e}"
                    )));
                }
                if let Err(e) = client.add_config_extension(ext_name, config, false).await {
                    let _ = tx.send(Event::Error(format!(
                        "Failed to disable extension in config: {e}"
                    )));
                }
            }

            match client.get_extensions().await {
                Ok(extensions) => {
                    let _ = tx.send(Event::ExtensionsLoaded(extensions));
                }
                Err(e) => {
                    let _ = tx.send(Event::Error(e.to_string()));
                }
            }
        });
    }
}

fn handle_fork_from_message(
    msg_idx: usize,
    state: &AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
) {
    let client = client.clone();
    let tx = tx.clone();
    let session_id = state.session_id.clone();

    tokio::spawn(async move {
        let exported = match client.export_session(&session_id).await {
            Ok(json) => {
                tracing::debug!(
                    "Fork: exported session (first 500 chars): {}",
                    &json[..json.len().min(500)]
                );
                json
            }
            Err(e) => {
                let _ = tx.send(Event::Error(format!("Export failed: {e}")));
                return;
            }
        };

        let mut session: serde_json::Value = match serde_json::from_str(&exported) {
            Ok(v) => v,
            Err(e) => {
                tracing::error!("Fork: failed to parse exported JSON: {e}");
                let _ = tx.send(Event::Error(format!("Parse failed: {e}")));
                return;
            }
        };

        let original_count = session
            .get("conversation")
            .and_then(|c| c.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        if let Some(conv) = session.get_mut("conversation") {
            if let Some(messages) = conv.as_array_mut() {
                messages.truncate(msg_idx + 1);
            }
        }

        let new_count = session
            .get("conversation")
            .and_then(|c| c.as_array())
            .map(|a| a.len())
            .unwrap_or(0);

        tracing::info!(
            "Fork: truncating from {} to {} messages (up to index {})",
            original_count,
            new_count,
            msg_idx
        );

        if let Some(name) = session.get("name").and_then(|n| n.as_str()) {
            session["name"] = serde_json::json!(format!("Fork: {}", name));
        }

        let modified_json = serde_json::to_string(&session).unwrap_or_default();
        tracing::info!(
            "Fork: sending import request with {} bytes",
            modified_json.len()
        );
        let forked = match client.import_session(&modified_json).await {
            Ok(s) => {
                tracing::info!("Fork: import succeeded, new session id: {}", s.id);
                s
            }
            Err(e) => {
                tracing::error!("Fork: import failed: {e}");
                let _ = tx.send(Event::Error(format!("Import failed: {e}")));
                return;
            }
        };

        tracing::info!(
            "Fork: imported session {} with {} messages",
            forked.id,
            forked
                .conversation
                .as_ref()
                .map(|c| c.messages().len())
                .unwrap_or(0)
        );

        match client.resume_agent(&forked.id).await {
            Ok(s) => {
                let _ = tx.send(Event::SessionResumed(Box::new(s)));
            }
            Err(e) => {
                let _ = tx.send(Event::Error(format!("Resume failed: {e}")));
            }
        }
    });
}

fn handle_set_goose_mode(mode: &str, client: &Client, tx: &mpsc::UnboundedSender<Event>) {
    use goose::config::GooseMode;
    use std::str::FromStr;

    let mode_str = mode.to_string();
    let client = client.clone();
    let tx = tx.clone();

    match GooseMode::from_str(&mode_str) {
        Ok(mode) => {
            let mode_name = match mode {
                GooseMode::Auto => "auto",
                GooseMode::Approve => "approve",
                GooseMode::Chat => "chat",
                GooseMode::SmartApprove => "smart_approve",
            };

            tokio::spawn(async move {
                if let Err(e) = client
                    .upsert_config("GOOSE_MODE", serde_json::json!(mode_name), false)
                    .await
                {
                    let _ = tx.send(Event::Error(format!("Failed to set mode: {e}")));
                } else {
                    let _ = tx.send(Event::Flash(format!("Mode set to: {mode_name}")));
                }
            });
        }
        Err(_) => {
            let _ = tx.send(Event::Error(
                "Invalid mode. Use: auto, approve, chat, smart_approve".to_string(),
            ));
        }
    }
}

fn handle_confirm_tool_call(
    id: &str,
    approved: bool,
    state: &AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
) {
    let client = client.clone();
    let tx = tx.clone();
    let session_id = state.session_id.clone();
    let request_id = id.to_string();
    let action = if approved { "allow_once" } else { "deny" };

    tokio::spawn(async move {
        if let Err(e) = client
            .confirm_tool_permission(&session_id, &request_id, action)
            .await
        {
            let _ = tx.send(Event::Error(format!("Failed to confirm tool: {e}")));
        }
    });
}
