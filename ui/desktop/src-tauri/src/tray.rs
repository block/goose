use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

const TRAY_ID: &str = "goose-tray";

pub fn create_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_tray_menu(app)?;

    let icon = load_tray_icon();

    let mut builder = TrayIconBuilder::with_id(TRAY_ID)
        .icon(icon)
        .tooltip("Goose")
        .menu(&menu)
        .on_menu_event(handle_tray_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                ..
            } = event
            {
                let app = tray.app_handle();
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.show();
                    let _ = window.set_focus();
                }
            }
        });

    #[cfg(target_os = "macos")]
    {
        builder = builder.icon_as_template(true);
    }

    builder.build(app)?;

    Ok(())
}

/// Show or hide the tray icon at runtime.
pub fn set_tray_visible(app: &tauri::AppHandle, visible: bool) {
    if let Some(tray) = app.tray_by_id(TRAY_ID) {
        let _ = tray.set_visible(visible);
    }
}

fn load_tray_icon() -> Image<'static> {
    // Use the macOS-style template icon (monochrome, supports dark/light mode)
    let icon_bytes = include_bytes!("../icons/iconTemplate@2x.png");
    Image::from_bytes(icon_bytes).expect("Failed to load tray icon")
}

fn build_tray_menu(
    app: &tauri::App,
) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let show_item = MenuItemBuilder::with_id("show", "Show Goose").build(app)?;
    let new_window = MenuItemBuilder::with_id("tray-new-window", "New Window").build(app)?;
    let open_dir = MenuItemBuilder::with_id("tray-open-directory", "Open Directory...").build(app)?;

    // Recent directories submenu
    let recent_dirs = load_recent_dirs();
    let recent_submenu = {
        let mut builder = SubmenuBuilder::new(app, "Recent Directories");
        if recent_dirs.is_empty() {
            let none_item =
                MenuItemBuilder::with_id("tray-no-recent", "No Recent Directories").build(app)?;
            builder = builder.item(&none_item);
        } else {
            for (i, dir) in recent_dirs.iter().enumerate().take(10) {
                let short = dir.split('/').last().unwrap_or(dir);
                let item = MenuItemBuilder::with_id(
                    &format!("tray-recent-{}", i),
                    short,
                )
                .build(app)?;
                builder = builder.item(&item);
            }
        }
        builder.build()?
    };

    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = MenuBuilder::new(app)
        .items(&[&show_item, &new_window, &open_dir])
        .item(&recent_submenu)
        .separator()
        .item(&quit_item)
        .build()?;

    Ok(menu)
}

fn handle_tray_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id();
    let id_str = id.as_ref();
    match id_str {
        "show" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
        "tray-new-window" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("new-window", ());
            }
        }
        "tray-open-directory" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("open-directory", ());
            }
        }
        "quit" => {
            app.exit(0);
        }
        other if other.starts_with("tray-recent-") => {
            if let Ok(idx) = other.trim_start_matches("tray-recent-").parse::<usize>() {
                let dirs = load_recent_dirs();
                if let Some(dir) = dirs.get(idx) {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.emit("open-directory-path", dir.clone());
                    }
                }
            }
        }
        _ => {}
    }
}

fn load_recent_dirs() -> Vec<String> {
    let config_dir = dirs::config_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".config"));
    let recent_file = config_dir.join("Goose").join("recent_dirs.json");

    if recent_file.exists() {
        if let Ok(content) = std::fs::read_to_string(&recent_file) {
            return serde_json::from_str(&content).unwrap_or_default();
        }
    }
    Vec::new()
}
