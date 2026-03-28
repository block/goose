//! Root application component.
//!
//! All reactive state lives here.  Children are pure render functions.
//!
//! Concurrency model:
//!   • `use_future` #1 — 300 ms spinner/animation tick (tokio::spawn loop)
//!   • `use_future` #2 — agent init + event-channel drain loop
//!   • tokio background task — drives `Agent::reply` stream, sends `AgentMsg`
//!   • `use_terminal_events` — keyboard input

use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;

use iocraft::prelude::*;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

use goose::agents::Agent;
use goose::agents::execute_commands::list_commands as list_agent_commands;
use goose::config::{get_all_extensions, set_extension_enabled, ExtensionEntry, GooseMode};
use goose::slash_commands::list_commands as list_recipe_commands;
use goose::model::ModelConfig;
use goose::providers::base::ModelInfo;
use goose::providers::{create as create_provider, providers as list_providers};

use crate::agent::{build_agent, run_agent_loop};
use crate::components::elicitation_dialog::ElicitationDialog;
use crate::components::extension_dialog::ExtensionDialog;
use crate::components::model_dialog::ModelDialog;
use crate::components::slash_completion::SlashCompletion;
use crate::colors::*;
use crate::components::{
    header::Header,
    input_bar::InputBar,
    permission_dialog::PermissionDialog,
    splash::Splash,
    turn_view::TurnView,
};
use crate::markdown::render as md;
use crate::types::{
    AgentMsg, ElicitationReq, PendingElicitReply, PendingReply,
    PermissionChoice, PermissionReq, Turn,
};

const MAX_QUEUE: usize = 10;

/// TUI-only slash commands (handled before reaching the agent).
const TUI_SLASH_COMMANDS: &[(&str, &str)] = &[
    ("/ext",   "list and toggle extensions"),
    ("/model", "switch model"),
    ("/exit",  "exit goose"),
];

// ── Control messages (keyboard → async futures) ───────────────────────────────

enum CtrlMsg {
    SetMode(GooseMode),
    ToggleExt(ExtensionEntry),
    SwitchModel(String),
}

// ── Props ─────────────────────────────────────────────────────────────────────

#[derive(Default, Props)]
pub struct AppProps {
    pub initial_prompt: Option<String>,
    pub session_id: Option<String>,
}

// ── App ───────────────────────────────────────────────────────────────────────

#[component]
pub fn App(props: &AppProps, mut hooks: Hooks) -> impl Into<AnyElement<'static>> {
    let mut system = hooks.use_context_mut::<SystemContext>();
    let (term_width, term_height) = hooks.use_terminal_size();

    // ── state ─────────────────────────────────────────────────────────────────
    let mut should_exit    = hooks.use_state(|| false);
    let mut turns          = hooks.use_state(Vec::<Turn>::new);
    let mut input          = hooks.use_state(String::new);
    let mut loading        = hooks.use_state(|| true);
    let mut status         = hooks.use_state(|| "connecting…".to_string());
    let mut spin_idx       = hooks.use_state(|| 0usize);
    let mut anim_frame     = hooks.use_state(|| 0usize);
    let mut banner_visible = hooks.use_state(|| true);
    let mut view_turn_idx  = hooks.use_state(|| None::<usize>); // None = latest
    let mut expanded_tc    = hooks.use_state(|| None::<String>);
    let mut scroll_offset  = hooks.use_state(|| 0i32);

    // Permission dialog state
    let mut pending_perm   = hooks.use_state(|| None::<PermissionReq>);
    let mut perm_idx       = hooks.use_state(|| 0usize);
    let pending_reply      = hooks.use_state(PendingReply::default);

    // Elicitation dialog state
    let mut pending_elicit    = hooks.use_state(|| None::<ElicitationReq>);
    let mut elicit_input      = hooks.use_state(String::new);
    let pending_elicit_reply  = hooks.use_state(PendingElicitReply::default);

    // Working directory + token usage (populated after agent init / each turn)
    let mut working_dir = hooks.use_state(|| "".to_string());
    let mut token_total = hooks.use_state(|| 0i64);

    // Prompt sender — set once the agent is ready.
    // Arc so it's Clone + 'static inside State.
    let mut prompt_tx: State<Option<Arc<mpsc::Sender<String>>>> =
        hooks.use_state(|| None);

    // Queued messages (sent while agent is busy).
    let mut queue = hooks.use_state(VecDeque::<String>::new);

    // Cancellation token — replaced each time a new agent turn starts.
    // Arc so it's Clone + 'static in State.
    let cancel_token: State<Arc<CancellationToken>> =
        hooks.use_state(|| Arc::new(CancellationToken::new()));

    // Channel sender for pushing cancel tokens to the agent worker.
    let mut cancel_tx: State<Option<Arc<mpsc::Sender<CancellationToken>>>> =
        hooks.use_state(|| None);

    // Goose mode + extension control state.
    let mut agent_arc:     State<Option<Arc<Agent>>> = hooks.use_state(|| None);
    let mut session_id_st: State<String>             = hooks.use_state(String::new);
    let mut goose_mode:    State<GooseMode>           = hooks.use_state(GooseMode::default);
    let mut ctrl_tx_st: State<Option<Arc<mpsc::Sender<CtrlMsg>>>> =
        hooks.use_state(|| None);

    // Extension dialog state.
    let mut ext_visible  = hooks.use_state(|| false);
    let mut ext_entries  = hooks.use_state(Vec::<ExtensionEntry>::new);
    let mut ext_idx      = hooks.use_state(|| 0usize);

    // Model dialog state.
    let mut model_visible    = hooks.use_state(|| false);
    let mut model_entries    = hooks.use_state(Vec::<ModelInfo>::new);
    let mut model_idx        = hooks.use_state(|| 0usize);
    let mut current_model    = hooks.use_state(String::new);
    let mut provider_name_st = hooks.use_state(String::new);

    // Slash-command completion state.
    let mut completion_idx = hooks.use_state(|| 0usize);

    // ── spinner tick ──────────────────────────────────────────────────────────
    hooks.use_future(async move {
        loop {
            tokio::time::sleep(Duration::from_millis(300)).await;
            spin_idx.set((spin_idx.get() + 1) % 4);
            anim_frame.set(anim_frame.get() + 1);
        }
    });

    // ── control loop (mode toggle + extension toggle) ─────────────────────────
    hooks.use_future(async move {
        let (ctx, mut crx) = mpsc::channel::<CtrlMsg>(8);
        ctrl_tx_st.set(Some(Arc::new(ctx)));
        while let Some(msg) = crx.recv().await {
            let maybe_agent = agent_arc.read().clone();
            let sid = session_id_st.read().clone();
            match msg {
                CtrlMsg::SetMode(next_mode) => {
                    if let Some(agent) = maybe_agent {
                        if agent.update_goose_mode(next_mode, &sid).await.is_ok() {
                            goose_mode.set(next_mode);
                        }
                    }
                }
                CtrlMsg::ToggleExt(entry) => {
                    let new_enabled = !entry.enabled;
                    let key = entry.config.key();
                    let name = entry.config.name();
                    // Persist to config file.
                    set_extension_enabled(&key, new_enabled);
                    // Apply to running agent.
                    if let Some(agent) = maybe_agent {
                        if new_enabled {
                            let _ = agent.add_extension(entry.config.clone(), &sid).await;
                        } else {
                            let _ = agent.remove_extension(&name, &sid).await;
                        }
                    }
                    // Refresh extension list in dialog.
                    ext_entries.set(get_all_extensions());
                }
                CtrlMsg::SwitchModel(model_name) => {
                    let pname = provider_name_st.read().clone();
                    if let Some(agent) = maybe_agent {
                        let extensions = agent.get_extension_configs().await;
                        if let Ok(model_cfg) = ModelConfig::new(&model_name)
                            .map(|c| c.with_canonical_limits(&pname))
                        {
                            if let Ok(new_provider) =
                                create_provider(&pname, model_cfg, extensions).await
                            {
                                if agent.update_provider(new_provider, &sid).await.is_ok() {
                                    current_model.set(model_name);
                                }
                            }
                        }
                    }
                }
            }
        }
    });

    // ── agent init + event loop ───────────────────────────────────────────────
    let session_id_hint = props.session_id.clone();
    let initial_prompt  = props.initial_prompt.clone();

    hooks.use_future(async move {
        // Channel for sending prompts to the agent worker.
        let (ptx, prx) = mpsc::channel::<String>(8);
        // Channel for receiving events from the agent worker.
        let (etx, mut erx) = mpsc::channel::<AgentMsg>(128);
        // Channel for sending cancellation tokens (one per turn).
        let (ctx, crx) = mpsc::channel::<CancellationToken>(4);
        // Store cancel sender so the keyboard handler can reach it.
        cancel_tx.set(Some(Arc::new(ctx)));

        // Store prompt sender so the submit handler can reach it.
        prompt_tx.set(Some(Arc::new(ptx)));

        match build_agent(session_id_hint).await {
            Err(e) => {
                loading.set(false);
                status.set(format!("failed: {e}"));
                return;
            }
            Ok(handle) => {
                loading.set(false);
                status.set("ready".to_string());

                // Show the session's working directory in the header.
                let cwd = &handle.working_dir;
                let home = dirs::home_dir().unwrap_or_default();
                let display = if let Ok(rel) = cwd.strip_prefix(&home) {
                    format!("~/{}", rel.display())
                } else {
                    cwd.display().to_string()
                };
                working_dir.set(display);

                // Read initial goose mode and store agent + session_id for later mode toggles.
                let initial_mode = handle.agent.goose_mode().await;
                goose_mode.set(initial_mode);
                session_id_st.set(handle.session_id.clone());

                // Store provider name + current model; pre-load known models for /model dialog.
                if let Ok(prov) = handle.agent.provider().await {
                    let pname = prov.get_name().to_string();
                    let mname = prov.get_model_config().model_name.clone();
                    let all_providers = list_providers().await;
                    let known = all_providers
                        .into_iter()
                        .find(|(meta, _)| meta.name == pname)
                        .map(|(meta, _)| meta.known_models)
                        .unwrap_or_default();
                    provider_name_st.set(pname);
                    current_model.set(mname);
                    model_entries.set(known);
                }

                agent_arc.set(Some(handle.agent.clone()));

                // Spawn the heavy worker on the tokio threadpool.
                tokio::spawn(run_agent_loop(handle, prx, etx, crx));

                // If an --text prompt was passed, fire it immediately.
                if let Some(ref text) = initial_prompt {
                    send_prompt_to_agent(
                        text.clone(),
                        &prompt_tx,
                        &cancel_tx,
                        cancel_token,
                        &mut turns,
                        &mut loading,
                        &mut status,
                        &mut banner_visible,
                    );
                }
            }
        }

        // Drain agent events and update state.
        while let Some(msg) = erx.recv().await {
            match msg {
                AgentMsg::TextChunk(chunk) => {
                    let mut t = turns.read().clone();
                    if let Some(last) = t.last_mut() {
                        last.agent_raw.push_str(&chunk);
                        last.agent_text = md(&last.agent_raw);
                    }
                    turns.set(t);
                }

                AgentMsg::ToolCallUpdate(info) => {
                    let mut t = turns.read().clone();
                    if let Some(last) = t.last_mut() {
                        if let Some(existing) = last.tool_calls.get_mut(&info.id) {
                            if !info.title.is_empty() { existing.title = info.title; }
                            existing.status = info.status;
                            if info.input_preview.is_some()  { existing.input_preview  = info.input_preview; }
                            if info.output_preview.is_some() { existing.output_preview = info.output_preview; }
                        } else {
                            if !last.tool_call_order.contains(&info.id) {
                                last.tool_call_order.push(info.id.clone());
                            }
                            last.tool_calls.insert(info.id.clone(), info);
                        }
                    }
                    turns.set(t);
                }

                AgentMsg::PermissionRequest(req, reply_tx) => {
                    pending_reply.read().put(reply_tx);
                    perm_idx.set(0);
                    pending_perm.set(Some(req));
                }

                AgentMsg::ElicitationRequest(req, reply_tx) => {
                    pending_elicit_reply.read().put(reply_tx);
                    elicit_input.set(String::new());
                    pending_elicit.set(Some(req));
                }

                AgentMsg::ConversationCleared => {
                    turns.set(Vec::new());
                    token_total.set(0);
                }

                AgentMsg::TokenUsage { total, .. } => {
                    token_total.set(total);
                }

                AgentMsg::Finished { stop_reason } => {
                    // agent_text is already rendered incrementally; nothing to do here.

                    loading.set(false);
                    status.set(if stop_reason == "end_turn" {
                        "ready".to_string()
                    } else {
                        format!("stopped: {stop_reason}")
                    });

                    // Drain the queue — send the next waiting message.
                    let next = queue.read().clone().pop_front();
                    if let Some(text) = next {
                        let mut q = queue.read().clone();
                        q.pop_front();
                        queue.set(q);
                        send_prompt_to_agent(
                            text,
                            &prompt_tx,
                            &cancel_tx,
                            cancel_token,
                            &mut turns,
                            &mut loading,
                            &mut status,
                            &mut banner_visible,
                        );
                    }
                }

                AgentMsg::Error(e) => {
                    loading.set(false);
                    status.set(format!("error: {e}"));
                }
            }
        }
    });

    // ── keyboard handler ──────────────────────────────────────────────────────
    hooks.use_terminal_events(move |event| {
        let TerminalEvent::Key(KeyEvent { code, kind, modifiers, .. }) = event else { return; };
        if kind == KeyEventKind::Release { return; }

        // Escape — dismiss dialogs / cancel turn only; never exits.
        if code == KeyCode::Esc {
            if pending_elicit.read().is_some() {
                if let Some(tx) = pending_elicit_reply.read().take() {
                    let _ = tx.send(String::new());
                }
                pending_elicit.set(None);
            } else if pending_perm.read().is_some() {
                if let Some(tx) = pending_reply.read().take() {
                    let _ = tx.send(PermissionChoice::Cancelled);
                }
                pending_perm.set(None);
            } else if loading.get() {
                cancel_token.read().cancel();
                status.set("stopping…".to_string());
            }
            return;
        }

        // Ctrl-C — cancel turn if loading, otherwise exit.
        if code == KeyCode::Char('c') && modifiers.contains(KeyModifiers::CONTROL) {
            if pending_elicit.read().is_some() {
                if let Some(tx) = pending_elicit_reply.read().take() {
                    let _ = tx.send(String::new());
                }
                pending_elicit.set(None);
            } else if pending_perm.read().is_some() {
                if let Some(tx) = pending_reply.read().take() {
                    let _ = tx.send(PermissionChoice::Cancelled);
                }
                pending_perm.set(None);
            } else if loading.get() {
                cancel_token.read().cancel();
                status.set("stopping…".to_string());
            } else {
                should_exit.set(true);
            }
            return;
        }

        // Extension dialog navigation.
        if ext_visible.get() {
            let n = ext_entries.read().len();
            match code {
                KeyCode::Esc => {
                    ext_visible.set(false);
                }
                KeyCode::Up => {
                    if n > 0 { ext_idx.set((ext_idx.get() + n - 1) % n); }
                }
                KeyCode::Down => {
                    if n > 0 { ext_idx.set((ext_idx.get() + 1) % n); }
                }
                KeyCode::Char(' ') | KeyCode::Enter => {
                    let entries = ext_entries.read().clone();
                    if let Some(entry) = entries.get(ext_idx.get()).cloned() {
                        if let Some(tx) = ctrl_tx_st.read().clone() {
                            let _ = tx.try_send(CtrlMsg::ToggleExt(entry));
                        }
                    }
                }
                _ => {}
            }
            return;
        }

        // Model dialog navigation.
        if model_visible.get() {
            let n = model_entries.read().len();
            match code {
                KeyCode::Esc => {
                    model_visible.set(false);
                }
                KeyCode::Up => {
                    if n > 0 { model_idx.set((model_idx.get() + n - 1) % n); }
                }
                KeyCode::Down => {
                    if n > 0 { model_idx.set((model_idx.get() + 1) % n); }
                }
                KeyCode::Enter => {
                    let entries = model_entries.read().clone();
                    if let Some(info) = entries.get(model_idx.get()).cloned() {
                        if let Some(tx) = ctrl_tx_st.read().clone() {
                            let _ = tx.try_send(CtrlMsg::SwitchModel(info.name));
                        }
                    }
                    model_visible.set(false);
                }
                _ => {}
            }
            return;
        }

        // Shift+Tab — cycle goose mode.
        if code == KeyCode::BackTab {
            let next = cycle_mode(goose_mode.get());
            if let Some(tx) = ctrl_tx_st.read().clone() {
                let _ = tx.try_send(CtrlMsg::SetMode(next));
            }
            return;
        }

        // Permission dialog navigation.
        let perm_clone = pending_perm.read().clone();
        if let Some(req) = perm_clone {
            let n = req.options.len();
            match code {
                KeyCode::Up   => perm_idx.set((perm_idx.get() + n - 1) % n),
                KeyCode::Down => perm_idx.set((perm_idx.get() + 1) % n),
                KeyCode::Enter => {
                    if let Some(opt) = req.options.get(perm_idx.get()) {
                        if let Some(tx) = pending_reply.read().take() {
                            let _ = tx.send(PermissionChoice::Selected(opt.id.clone()));
                        }
                    }
                    pending_perm.set(None);
                }
                KeyCode::Char(c) => {
                    // Direct key shortcuts (y/a/n/N).
                    if let Some(opt) = req.options.iter().find(|o| o.key == c) {
                        let id = opt.id.clone();
                        if let Some(tx) = pending_reply.read().take() {
                            let _ = tx.send(PermissionChoice::Selected(id));
                        }
                        pending_perm.set(None);
                    }
                }
                _ => {}
            }
            return;
        }

        // Elicitation dialog: Enter submits the typed response.
        if code == KeyCode::Enter && !modifiers.contains(KeyModifiers::SHIFT)
            && pending_elicit.read().is_some()
        {
            let text = elicit_input.read().trim().to_string();
            if let Some(tx) = pending_elicit_reply.read().take() {
                let _ = tx.send(text);
            }
            pending_elicit.set(None);
            elicit_input.set(String::new());
            return;
        }

        // Reset completion selection when the user types or deletes (narrows/widens the list).
        if matches!(code, KeyCode::Char(_) | KeyCode::Backspace | KeyCode::Delete) {
            completion_idx.set(0);
        }

        // Slash-completion ↑/↓ navigation (captures before scroll when popup is visible).
        {
            let cur_input = input.read().clone();
            let matches = slash_completions(&cur_input);
            if !matches.is_empty() && !modifiers.contains(KeyModifiers::SHIFT) {
                let n = matches.len();
                if code == KeyCode::Up {
                    completion_idx.set((completion_idx.get() + n - 1) % n);
                    return;
                }
                if code == KeyCode::Down {
                    completion_idx.set((completion_idx.get() + 1) % n);
                    return;
                }
            }
        }

        // Scroll (↑↓ without shift).
        if code == KeyCode::Up && !modifiers.contains(KeyModifiers::SHIFT) {
            scroll_offset.set(scroll_offset.get() + 3);
            return;
        }
        if code == KeyCode::Down && !modifiers.contains(KeyModifiers::SHIFT) {
            scroll_offset.set((scroll_offset.get() - 3).max(0));
            return;
        }

        // History navigation (Shift + ↑↓).
        let total = turns.read().len();
        if code == KeyCode::Up && modifiers.contains(KeyModifiers::SHIFT) && total > 1 {
            let cur = view_turn_idx.get().unwrap_or(total - 1);
            view_turn_idx.set(Some(cur.saturating_sub(1)));
            expanded_tc.set(None);
            scroll_offset.set(0);
            return;
        }
        if code == KeyCode::Down && modifiers.contains(KeyModifiers::SHIFT) {
            let cur = view_turn_idx.get().unwrap_or(total);
            view_turn_idx.set(if cur + 1 >= total { None } else { Some(cur + 1) });
            expanded_tc.set(None);
            scroll_offset.set(0);
            return;
        }

        // Tab — complete highlighted slash command if popup active, else expand/collapse tool call.
        if code == KeyCode::Tab {
            let cur_input = input.read().clone();
            let matches = slash_completions(&cur_input);
            if !matches.is_empty() {
                let idx = completion_idx.get() % matches.len();
                input.set(matches[idx].0.clone());
                // Keep completion_idx so the same item stays highlighted.
                return;
            }
            let t = turns.read();
            let idx = view_turn_idx.get().unwrap_or(t.len().saturating_sub(1));
            if let Some(turn) = t.get(idx) {
                if let Some(feat) = turn.tool_call_order.last() {
                    let next = if expanded_tc.read().as_deref() == Some(feat.as_str()) {
                        None
                    } else {
                        Some(feat.clone())
                    };
                    expanded_tc.set(next);
                }
            }
            return;
        }

        // Enter — submit from splash screen (banner visible).
        // Shift+Enter is handled by TextInput (inserts newline); don't submit.
        if code == KeyCode::Enter && !modifiers.contains(KeyModifiers::SHIFT) && banner_visible.get() {
            let text = input.read().trim().to_string();
            if text.is_empty() { return; }
            input.set(String::new());
            completion_idx.set(0);
            if check_slash_command(&text, &mut should_exit, &mut ext_visible, &mut ext_entries, &mut ext_idx, &mut model_visible, &mut model_idx) {
                return;
            }
            send_prompt_to_agent(
                text,
                &prompt_tx,
                &cancel_tx,
                cancel_token,
                &mut turns,
                &mut loading,
                &mut status,
                &mut banner_visible,
            );
            return;
        }

        // Enter — submit from input bar.
        // Shift+Enter is handled by TextInput (inserts newline); don't submit.
        if code == KeyCode::Enter && !modifiers.contains(KeyModifiers::SHIFT) && !banner_visible.get() {
            let text = input.read().trim().to_string();
            if text.is_empty() { return; }
            input.set(String::new());
            completion_idx.set(0);
            view_turn_idx.set(None);
            expanded_tc.set(None);
            scroll_offset.set(0);

            if check_slash_command(&text, &mut should_exit, &mut ext_visible, &mut ext_entries, &mut ext_idx, &mut model_visible, &mut model_idx) {
                return;
            }

            if loading.get() {
                if queue.read().len() < MAX_QUEUE {
                    let mut q = queue.read().clone();
                    q.push_back(text);
                    queue.set(q);
                }
            } else {
                send_prompt_to_agent(
                    text,
                    &prompt_tx,
                    &cancel_tx,
                    cancel_token,
                    &mut turns,
                    &mut loading,
                    &mut status,
                    &mut banner_visible,
                );
            }
        }
    });

    // ── render ────────────────────────────────────────────────────────────────

    if should_exit.get() {
        system.exit();
    }

    let turns_snap   = turns.read();
    let view_idx     = view_turn_idx.get().unwrap_or(turns_snap.len().saturating_sub(1));
    let cur_turn     = turns_snap.get(view_idx).cloned();
    let is_history   = view_turn_idx.get().is_some_and(|i| i + 1 < turns_snap.len());
    let turn_info    = (turns_snap.len() > 1).then_some((view_idx + 1, turns_snap.len()));
    let turns_total  = turns_snap.len();
    drop(turns_snap); // release read lock before potentially triggering re-render
    let rule         = "─".repeat(term_width as usize);

    let queue_snap    = queue.read().clone();
    let perm_snap     = pending_perm.read().clone();
    let elicit_snap   = pending_elicit.read().clone();
    let expanded_snap = expanded_tc.read().clone();
    let banner        = banner_visible.get();
    let cwd_snap      = working_dir.read().clone();
    let tokens        = token_total.get();
    let mode_str      = goose_mode.get().to_string();
    let ext_snap       = ext_entries.read().clone();
    let ext_open       = ext_visible.get();
    let model_snap     = model_entries.read().clone();
    let model_open     = model_visible.get();
    let cur_model_snap = current_model.read().clone();
    let input_snap     = input.read().clone();
    let completions    = slash_completions(&input_snap);

    // Always return a single root View; use conditional #() blocks inside.
    element! {
        View(
            flex_direction: FlexDirection::Column,
            width: term_width,
            height: term_height,
        ) {
            // ── Splash screen ─────────────────────────────────────────────
            #(banner.then(|| element! {
                Splash(
                    status: status.to_string(),
                    anim_frame: anim_frame.get(),
                    show_input: !loading.get() && props.initial_prompt.is_none(),
                    input: input,
                    width: term_width,
                    height: term_height,
                )
            }))

            // ── Main UI (shown after first prompt) ────────────────────────
            #((!banner).then(|| element! {
                View(
                    flex_direction: FlexDirection::Column,
                    flex_grow: 1.0,
                    padding_left: 2,
                    padding_right: 2,
                ) {
                    Header(
                        status: status.to_string(),
                        loading: loading.get(),
                        spin_idx: spin_idx.get(),
                        turn_info: turn_info,
                        working_dir: cwd_snap,
                        token_total: tokens,
                        goose_mode: mode_str,
                        width: term_width - 4,
                    )

                    // Current turn — user prompt
                    #(cur_turn.as_ref().map(|t| element! {
                        View(flex_direction: FlexDirection::Row, padding_left: 3, margin_top: 1) {
                            Text(content: "❯ ", color: CRANBERRY, weight: Weight::Bold)
                            Text(content: t.user_text.clone(), color: TEXT_PRIMARY, weight: Weight::Bold)
                        }
                    }))

                    // Scroll indicator — content hidden above
                    #((scroll_offset.get() > 0).then(|| element! {
                        View(justify_content: JustifyContent::Center, width: term_width - 4) {
                            Text(content: "▲ scroll up for more ▲", color: TEXT_DIM, italic: true)
                        }
                    }))

                    // Agent response + tool calls (scrollable)
                    View(flex_grow: 1.0, overflow_y: Overflow::Hidden) {
                        TurnView(
                            turn: cur_turn,
                            expanded_tool_call: expanded_snap,
                            active: !is_history && loading.get(),
                            status: status.to_string(),
                            width: term_width - 4,
                            scroll_offset: scroll_offset.get(),
                        )
                    }

                    // Scroll indicator — content hidden below (we've scrolled up)
                    #((scroll_offset.get() > 0).then(|| element! {
                        View(justify_content: JustifyContent::Center, width: term_width - 4) {
                            Text(content: "▼ scroll down for more ▼", color: TEXT_DIM, italic: true)
                        }
                    }))

                    // Permission dialog (injected inline)
                    #(perm_snap.clone().map(|req| element! {
                        PermissionDialog(
                            request: Some(req),
                            selected_idx: perm_idx.get(),
                        )
                    }))

                    // Elicitation dialog (free-text input from agent)
                    #(elicit_snap.clone().map(|req| element! {
                        ElicitationDialog(
                            request: Some(req),
                            value: elicit_input,
                            width: term_width - 4,
                        )
                    }))

                    // Model switcher dialog
                    #(model_open.then(|| element! {
                        ModelDialog(
                            models: model_snap.clone(),
                            current_model: cur_model_snap.clone(),
                            selected_idx: model_idx.get(),
                            width: term_width - 4,
                        )
                    }))

                    // Extension dialog
                    #(ext_open.then(|| element! {
                        ExtensionDialog(
                            extensions: ext_snap.clone(),
                            selected_idx: ext_idx.get(),
                            width: term_width - 4,
                        )
                    }))

                    // History navigation indicator
                    #(is_history.then(|| element! {
                        View(flex_direction: FlexDirection::Column, width: term_width - 4) {
                            Text(content: rule.clone(), color: RULE)
                            View(justify_content: JustifyContent::Center) {
                                Text(content: format!("turn {}/{}", view_idx + 1, turns_total), color: GOLD)
                                Text(content: " — shift+↓ to return", color: TEXT_DIM)
                            }
                        }
                    }))

                    // Queued message hints
                    #(queue_snap.iter().enumerate().map(|(i, msg)| element! {
                        View(key: i.to_string(), flex_direction: FlexDirection::Row, padding_left: 3) {
                            Text(content: "❯ ", color: TEXT_DIM)
                            Text(content: msg.clone(), color: TEXT_DIM)
                            Text(content: "  (queued)", color: GOLD)
                        }
                    }))

                    // Slash-command completion popup (shown above input bar when typing /)
                    #((!is_history && perm_snap.is_none() && elicit_snap.is_none() && !ext_open && !model_open && !completions.is_empty() && props.initial_prompt.is_none()).then(|| {
                        let items: Vec<(String, String)> = completions.iter()
                            .map(|(c, d)| (c.to_string(), d.to_string()))
                            .collect();
                        element! {
                            SlashCompletion(
                                completions: items,
                                selected_idx: completion_idx.get() % completions.len().max(1),
                                width: term_width - 4,
                            )
                        }
                    }))

                    // Input bar (hidden when permission/elicitation/extension dialog is active)
                    #((!is_history && perm_snap.is_none() && elicit_snap.is_none() && !ext_open && !model_open && props.initial_prompt.is_none()).then(|| element! {
                        InputBar(
                            value: input,
                            has_queued: !queue_snap.is_empty(),
                            width: term_width - 4,
                        )
                    }))
                }
            }))
        }
    }
}

// ── Helper: compute slash command completions ─────────────────────────────────

fn slash_completions(input: &str) -> Vec<(String, String)> {
    if !input.starts_with('/') || input.contains(' ') {
        return vec![];
    }
    // TUI-local commands
    let mut out: Vec<(String, String)> = TUI_SLASH_COMMANDS
        .iter()
        .filter(|(cmd, _)| cmd.starts_with(input))
        .map(|(c, d)| (c.to_string(), d.to_string()))
        .collect();
    // Agent built-in commands (/compact, /clear, /prompts, /prompt)
    for cmd in list_agent_commands() {
        let full = format!("/{}", cmd.name);
        if full.starts_with(input) {
            out.push((full, cmd.description.to_string()));
        }
    }
    // User recipe commands
    for mapping in list_recipe_commands() {
        let full = mapping.command.clone();
        let full = if full.starts_with('/') { full } else { format!("/{}", full) };
        if full.starts_with(input) {
            out.push((full, "recipe".to_string()));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

// ── Helper: check slash commands (sync — called from keyboard handler) ────────

fn check_slash_command(
    text: &str,
    should_exit: &mut State<bool>,
    ext_visible: &mut State<bool>,
    ext_entries: &mut State<Vec<ExtensionEntry>>,
    ext_idx: &mut State<usize>,
    model_visible: &mut State<bool>,
    model_idx: &mut State<usize>,
) -> bool {
    match text.trim() {
        "/exit" | "/quit" => {
            should_exit.set(true);
            true
        }
        "/ext" => {
            ext_entries.set(get_all_extensions());
            ext_idx.set(0);
            ext_visible.set(true);
            true
        }
        "/model" => {
            // model_entries was pre-loaded at init; just open the dialog.
            model_idx.set(0);
            model_visible.set(true);
            true
        }
        _ => false,
    }
}

// ── Helper: cycle through GooseMode variants ──────────────────────────────────

fn cycle_mode(current: GooseMode) -> GooseMode {
    match current {
        GooseMode::Auto        => GooseMode::Approve,
        GooseMode::Approve     => GooseMode::SmartApprove,
        GooseMode::SmartApprove => GooseMode::Chat,
        GooseMode::Chat        => GooseMode::Auto,
    }
}

// ── Helper: send a prompt to the agent and update state ───────────────────────

fn send_prompt_to_agent(
    text: String,
    prompt_tx: &State<Option<Arc<mpsc::Sender<String>>>>,
    cancel_tx: &State<Option<Arc<mpsc::Sender<CancellationToken>>>>,
    mut cancel_token: State<Arc<CancellationToken>>,
    turns: &mut State<Vec<Turn>>,
    loading: &mut State<bool>,
    status: &mut State<String>,
    banner_visible: &mut State<bool>,
) {
    let Some(tx) = prompt_tx.read().clone() else { return };

    // Create a fresh cancellation token for this turn and share it with
    // the agent worker so it can be cancelled mid-stream.
    let token = CancellationToken::new();
    if let Some(ctx) = cancel_tx.read().clone() {
        let _ = ctx.try_send(token.clone());
    }
    cancel_token.set(Arc::new(token));

    let mut t = turns.read().clone();
    t.push(Turn { user_text: text.clone(), ..Default::default() });
    turns.set(t);

    banner_visible.set(false);
    loading.set(true);
    status.set("thinking…".to_string());

    // Fire-and-forget: the channel is buffered.
    if tx.try_send(text).is_err() {
        loading.set(false);
        status.set("error: agent channel full".to_string());
    }
}
