use std::sync::Mutex;
use tauri::{Emitter, Listener, Manager};

mod commands;
mod goosed;
mod menu;
mod settings;
mod tray;
mod wakelock;

/// Deep links received before the React frontend signals ready.
pub struct PendingDeepLinks(pub Mutex<Vec<String>>);

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        // Plugins
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_notification::init())
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_single_instance::init(|app, args, _cwd| {
            // Handle second instance — focus existing window or process deep link
            if let Some(url) = args.get(1) {
                commands::handle_deep_link(app, url);
            }
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .plugin(tauri_plugin_updater::Builder::new().build())
        .plugin(tauri_plugin_window_state::Builder::default().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_store::Builder::default().build())
        .plugin(tauri_plugin_process::init())
        .plugin(
            tauri_plugin_log::Builder::default()
                .target(tauri_plugin_log::Target::new(
                    tauri_plugin_log::TargetKind::LogDir {
                        file_name: Some("goose".into()),
                    },
                ))
                .build(),
        )
        .plugin(tauri_plugin_clipboard_manager::init())
        .plugin(tauri_plugin_http::init())
        .plugin(tauri_plugin_autostart::init(
            tauri_plugin_autostart::MacosLauncher::LaunchAgent,
            None,
        ))
        // State
        .manage(goosed::GoosedState::default())
        .manage(settings::SettingsState::default())
        .manage(PendingDeepLinks(Mutex::new(Vec::new())))
        .manage(wakelock::WakelockState::default())
        // Setup
        .setup(|app| {
            // System tray
            tray::create_tray(app)?;

            // Application menu
            menu::setup_menu(app)?;

            // Start goosed backend
            let app_handle = app.handle().clone();

            let external_config = {
                let settings_state = app.state::<settings::SettingsState>();
                let s = settings_state.0.lock().unwrap();
                s.external_goosed.clone()
            };

            tauri::async_runtime::spawn(async move {
                let goosed_state = app_handle.state::<goosed::GoosedState>();
                match goosed::start_goosed(
                    &app_handle,
                    &goosed_state,
                    external_config.as_ref(),
                )
                .await
                {
                    Ok(url) => {
                        log::info!("goosed started at {}", url);
                    }
                    Err(e) => {
                        log::error!("Failed to start goosed: {}", e);
                        let _ = app_handle.emit("fatal-error", &e);
                    }
                }
            });

            // Listen for deep links — queue them until React is ready
            let app_handle2 = app.handle().clone();
            app.listen("deep-link://new-url", move |event: tauri::Event| {
                let payload = event.payload();
                let pending = app_handle2.state::<PendingDeepLinks>();
                let mut links = pending.0.lock().unwrap();
                links.push(payload.to_string());
                // Also try to emit immediately (if React is ready it'll receive it)
                commands::handle_deep_link(&app_handle2, payload);
            });

            // Listen for react-ready and dispatch any pending deep links
            let app_handle3 = app.handle().clone();
            app.listen("react-ready", move |_event: tauri::Event| {
                let pending = app_handle3.state::<PendingDeepLinks>();
                let links: Vec<String> = {
                    let mut guard = pending.0.lock().unwrap();
                    guard.drain(..).collect()
                };
                for url in links {
                    commands::handle_deep_link(&app_handle3, &url);
                }
            });

            // Broadcast theme changes to all windows
            let app_handle4 = app.handle().clone();
            app.listen("broadcast-theme-change", move |event: tauri::Event| {
                let payload = event.payload();
                for (_label, window) in app_handle4.webview_windows() {
                    let _ = window.emit("theme-changed", payload);
                }
            });

            // Register global shortcuts
            setup_global_shortcuts(app)?;

            Ok(())
        })
        // Commands
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::save_settings,
            commands::get_secret_key,
            commands::get_goosed_host_port,
            commands::read_file,
            commands::write_file,
            commands::ensure_directory,
            commands::list_files,
            commands::open_directory_in_explorer,
            commands::check_for_ollama,
            commands::fetch_metadata,
            commands::create_chat_window,
            commands::get_version,
            commands::set_wakelock,
            commands::get_wakelock_state,
            commands::set_spellcheck,
            commands::get_spellcheck_state,
            commands::get_config,
            commands::get_allowed_extensions,
            commands::add_recent_dir,
            commands::has_accepted_recipe_before,
            commands::record_recipe_hash,
            commands::launch_app,
            commands::refresh_app,
            commands::close_app,
            commands::select_file_or_directory,
            commands::restart_app,
            commands::log_from_frontend,
            commands::open_in_chrome,
        ])
        // Cleanup on exit
        .on_window_event(|window, event| {
            if let tauri::WindowEvent::Destroyed = event {
                // Only cleanup goosed when the last window is destroyed
                let app = window.app_handle();
                let windows = app.webview_windows();
                if windows.len() <= 1 {
                    let state = app.state::<goosed::GoosedState>();
                    goosed::stop_goosed(&state);
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Goose");
}

fn setup_global_shortcuts(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    use tauri_plugin_global_shortcut::GlobalShortcutExt;

    let app_handle = app.handle().clone();

    // Register the default focus shortcut
    app.global_shortcut().on_shortcut("CmdOrCtrl+Alt+G", move |_app, _shortcut, _event| {
        if let Some(window) = app_handle.get_webview_window("main") {
            let _ = window.show();
            let _ = window.set_focus();
        }
    })?;

    Ok(())
}
