use crate::hidden_blocks::CWD_ANALYSIS_TAG;
use crate::services::events::Event;
use crate::state::action::Action;
use crate::state::{AppState, CwdAnalysisState};
use goose::conversation::message::Message;
use goose_client::Client;
use goose_tui::analysis_target::detect_analysis_target;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

const CWD_ANALYSIS_TIMEOUT: Duration = Duration::from_secs(30);
const CWD_ANALYSIS_GRACE_PERIOD: Duration = Duration::from_secs(2);

pub fn handle_action(
    action: &Action,
    state: &mut AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
    reply_task: &mut Option<tokio::task::JoinHandle<()>>,
) -> bool {
    match action {
        Action::SendMessage(message_to_send) => {
            handle_send_message(message_to_send, state, client, tx, reply_task);
        }
        Action::SendMessageWithFlash { message, .. } => {
            handle_send_message(message, state, client, tx, reply_task);
        }
        Action::ResumeSession(id) => {
            handle_resume_session(id, client, tx);
        }
        Action::CreateNewSession => {
            handle_create_new_session(state, client, tx);
        }
        Action::OpenSessionPicker => {
            handle_open_session_picker(client, tx);
        }
        Action::OpenConfig | Action::OpenMcp => {
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
        Action::OpenSchedulePopup | Action::RefreshSchedules => {
            spawn_schedule_list(client, tx);
        }
        Action::CreateSchedule {
            id,
            recipe_source,
            cron,
        } => {
            spawn_schedule_create(client, tx, id.clone(), recipe_source.clone(), cron.clone());
        }
        Action::UpdateScheduleCron { id, cron } => {
            spawn_schedule_update(client, tx, id.clone(), cron.clone());
        }
        Action::DeleteSchedule(id) => {
            spawn_schedule_delete(client, tx, id.clone());
        }
        Action::RunScheduleNow(id) => {
            spawn_schedule_run(client, tx, id.clone());
        }
        Action::PauseSchedule(id) => {
            spawn_schedule_pause(client, tx, id.clone());
        }
        Action::UnpauseSchedule(id) => {
            spawn_schedule_unpause(client, tx, id.clone());
        }
        Action::KillSchedule(id) => {
            spawn_schedule_kill(client, tx, id.clone());
        }
        Action::FetchScheduleSessions(id) => {
            spawn_schedule_sessions(client, tx, id.clone());
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

pub fn spawn_cwd_analysis(session_id: &str, client: &Client, tx: &mpsc::UnboundedSender<Event>) {
    let client = client.clone();
    let tx = tx.clone();
    let session_id = session_id.to_string();
    tokio::spawn(async move {
        let result = fetch_cwd_analysis(&client, &session_id).await;
        let _ = tx.send(Event::CwdAnalysisComplete(result));
    });
}

pub async fn fetch_cwd_analysis_sync(client: &Client, session_id: &str) -> Option<String> {
    fetch_cwd_analysis(client, session_id).await
}

async fn fetch_cwd_analysis(client: &Client, session_id: &str) -> Option<String> {
    let cwd = std::env::current_dir().ok()?;
    let (target_path, depth) = detect_analysis_target(&cwd);
    let start = std::time::Instant::now();
    tracing::info!(
        "Smart context: analyzing {} (depth={})",
        target_path.display(),
        depth
    );

    let analyze_future = client.call_tool(
        session_id,
        "developer__analyze",
        Some(serde_json::json!({
            "path": target_path.to_string_lossy(),
            "max_depth": depth
        })),
    );

    let result = tokio::time::timeout(CWD_ANALYSIS_TIMEOUT, analyze_future).await;
    let elapsed = start.elapsed();

    match result {
        Ok(Ok(response)) if !response.is_error && !response.output.is_empty() => {
            tracing::info!(
                "Smart context: completed in {:.2}s ({} chars)",
                elapsed.as_secs_f64(),
                response.output.len()
            );
            Some(response.output)
        }
        Ok(Ok(response)) if response.is_error => {
            tracing::warn!(
                "Smart context: failed in {:.2}s: {}",
                elapsed.as_secs_f64(),
                response.output
            );
            None
        }
        Ok(Err(e)) => {
            tracing::warn!(
                "Smart context: error in {:.2}s: {}",
                elapsed.as_secs_f64(),
                e
            );
            None
        }
        Err(_) => {
            tracing::warn!(
                "Smart context: timed out after {:.2}s",
                elapsed.as_secs_f64()
            );
            None
        }
        _ => {
            tracing::debug!(
                "Smart context: empty result in {:.2}s",
                elapsed.as_secs_f64()
            );
            None
        }
    }
}

fn prepend_cwd_analysis(message: &Message, analysis: &str) -> Message {
    let original_text = message.as_concat_text();
    let augmented =
        format!("<{CWD_ANALYSIS_TAG}>\n{analysis}\n</{CWD_ANALYSIS_TAG}>\n\n{original_text}");
    Message::user().with_text(augmented)
}

async fn wait_for_pending_analysis(
    client: &Client,
    session_id: &str,
    tx: &mpsc::UnboundedSender<Event>,
) -> Option<String> {
    let (result_tx, mut result_rx) = mpsc::unbounded_channel();
    let client = client.clone();
    let session_id = session_id.to_string();

    tokio::spawn(async move {
        let result = fetch_cwd_analysis(&client, &session_id).await;
        let _ = result_tx.send(result);
    });

    match tokio::time::timeout(CWD_ANALYSIS_GRACE_PERIOD, result_rx.recv()).await {
        Ok(Some(result)) => {
            let _ = tx.send(Event::CwdAnalysisComplete(result.clone()));
            result
        }
        _ => {
            tracing::info!("Smart context: grace period expired, proceeding without analysis");
            None
        }
    }
}

fn handle_send_message(
    message_to_send: &Message,
    state: &mut AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
    reply_task: &mut Option<tokio::task::JoinHandle<()>>,
) {
    let client = client.clone();
    let tx = tx.clone();
    let mut messages_snapshot = state.messages.clone();
    let session_id = state.session_id.clone();

    let is_first_message = messages_snapshot.is_empty();
    let analysis_result = if is_first_message {
        state.cwd_analysis.take_result()
    } else {
        None
    };
    let analysis_pending = is_first_message && state.cwd_analysis.is_pending();

    let user_message = message_to_send.clone();

    let task = tokio::spawn(async move {
        let analysis = if analysis_result.is_some() {
            analysis_result
        } else if analysis_pending {
            wait_for_pending_analysis(&client, &session_id, &tx).await
        } else {
            None
        };

        let final_user_message = match analysis {
            Some(ref a) => prepend_cwd_analysis(&user_message, a),
            None => user_message,
        };

        messages_snapshot.push(final_user_message);

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

fn handle_create_new_session(
    state: &mut AppState,
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
) {
    let smart_context = state.config.smart_context;
    if smart_context {
        state.cwd_analysis = CwdAnalysisState::Pending;
    }
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        let cwd = std::env::current_dir().unwrap_or_default();
        match client.start_agent(cwd.to_string_lossy().to_string()).await {
            Ok(s) => {
                crate::configure_session_from_global(&client, &s.id).await;
                if smart_context {
                    spawn_cwd_analysis(&s.id, &client, &tx);
                }
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

fn spawn_schedule_list(client: &Client, tx: &mpsc::UnboundedSender<Event>) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.list_schedules().await {
            Ok(jobs) => {
                let _ = tx.send(Event::ScheduleListLoaded(jobs));
            }
            Err(e) => {
                let _ = tx.send(Event::ScheduleOperationFailed(e.to_string()));
            }
        }
    });
}

fn spawn_schedule_create(
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
    id: String,
    recipe_source: String,
    cron: String,
) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.create_schedule(&id, &recipe_source, &cron).await {
            Ok(_) => {
                let _ = tx.send(Event::ScheduleOperationSuccess(format!(
                    "Created schedule '{}'",
                    id
                )));
                if let Ok(jobs) = client.list_schedules().await {
                    let _ = tx.send(Event::ScheduleListLoaded(jobs));
                }
            }
            Err(e) => {
                let _ = tx.send(Event::ScheduleOperationFailed(e.to_string()));
            }
        }
    });
}

fn spawn_schedule_update(
    client: &Client,
    tx: &mpsc::UnboundedSender<Event>,
    id: String,
    cron: String,
) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.update_schedule_cron(&id, &cron).await {
            Ok(_) => {
                let _ = tx.send(Event::ScheduleOperationSuccess(format!(
                    "Updated schedule '{}'",
                    id
                )));
                if let Ok(jobs) = client.list_schedules().await {
                    let _ = tx.send(Event::ScheduleListLoaded(jobs));
                }
            }
            Err(e) => {
                let _ = tx.send(Event::ScheduleOperationFailed(e.to_string()));
            }
        }
    });
}

fn spawn_schedule_delete(client: &Client, tx: &mpsc::UnboundedSender<Event>, id: String) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.delete_schedule(&id).await {
            Ok(_) => {
                let _ = tx.send(Event::ScheduleOperationSuccess(format!(
                    "Deleted schedule '{}'",
                    id
                )));
                if let Ok(jobs) = client.list_schedules().await {
                    let _ = tx.send(Event::ScheduleListLoaded(jobs));
                }
            }
            Err(e) => {
                let _ = tx.send(Event::ScheduleOperationFailed(e.to_string()));
            }
        }
    });
}

fn spawn_schedule_run(client: &Client, tx: &mpsc::UnboundedSender<Event>, id: String) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.run_schedule_now(&id).await {
            Ok(session_id) => {
                let _ = tx.send(Event::ScheduleOperationSuccess(format!(
                    "Started '{}' (session: {})",
                    id, session_id
                )));
                if let Ok(jobs) = client.list_schedules().await {
                    let _ = tx.send(Event::ScheduleListLoaded(jobs));
                }
            }
            Err(e) => {
                let _ = tx.send(Event::ScheduleOperationFailed(e.to_string()));
            }
        }
    });
}

fn spawn_schedule_pause(client: &Client, tx: &mpsc::UnboundedSender<Event>, id: String) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.pause_schedule(&id).await {
            Ok(_) => {
                let _ = tx.send(Event::ScheduleOperationSuccess(format!("Paused '{}'", id)));
                if let Ok(jobs) = client.list_schedules().await {
                    let _ = tx.send(Event::ScheduleListLoaded(jobs));
                }
            }
            Err(e) => {
                let _ = tx.send(Event::ScheduleOperationFailed(e.to_string()));
            }
        }
    });
}

fn spawn_schedule_unpause(client: &Client, tx: &mpsc::UnboundedSender<Event>, id: String) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.unpause_schedule(&id).await {
            Ok(_) => {
                let _ = tx.send(Event::ScheduleOperationSuccess(format!("Resumed '{}'", id)));
                if let Ok(jobs) = client.list_schedules().await {
                    let _ = tx.send(Event::ScheduleListLoaded(jobs));
                }
            }
            Err(e) => {
                let _ = tx.send(Event::ScheduleOperationFailed(e.to_string()));
            }
        }
    });
}

fn spawn_schedule_kill(client: &Client, tx: &mpsc::UnboundedSender<Event>, id: String) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.kill_schedule(&id).await {
            Ok(_) => {
                let _ = tx.send(Event::ScheduleOperationSuccess(format!("Killed '{}'", id)));
                if let Ok(jobs) = client.list_schedules().await {
                    let _ = tx.send(Event::ScheduleListLoaded(jobs));
                }
            }
            Err(e) => {
                let _ = tx.send(Event::ScheduleOperationFailed(e.to_string()));
            }
        }
    });
}

fn spawn_schedule_sessions(client: &Client, tx: &mpsc::UnboundedSender<Event>, id: String) {
    let client = client.clone();
    let tx = tx.clone();
    tokio::spawn(async move {
        match client.get_schedule_sessions(&id, 50).await {
            Ok(sessions) => {
                let _ = tx.send(Event::ScheduleSessionsLoaded {
                    schedule_id: id,
                    sessions,
                });
            }
            Err(e) => {
                let _ = tx.send(Event::ScheduleOperationFailed(e.to_string()));
            }
        }
    });
}
