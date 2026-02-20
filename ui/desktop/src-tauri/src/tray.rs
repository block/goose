use tauri::{
    image::Image,
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    tray::{MouseButton, TrayIconBuilder, TrayIconEvent},
    Emitter, Manager,
};

const TRAY_ID: &str = "goose-tray";

pub fn create_tray(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let handle = app.handle();
    let menu = build_tray_menu(handle, false)?;

    let icon = load_tray_icon(false);

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

/// Update the tray icon and menu to reflect update availability.
pub fn set_update_available(app: &tauri::AppHandle, available: bool) {
    let Some(tray) = app.tray_by_id(TRAY_ID) else {
        return;
    };

    // Swap icon
    let icon = load_tray_icon(available);
    let _ = tray.set_icon(Some(icon));

    #[cfg(target_os = "macos")]
    {
        let _ = tray.set_icon_as_template(true);
    }

    // Update tooltip
    let tooltip = if available {
        "Goose - Update Available"
    } else {
        "Goose"
    };
    let _ = tray.set_tooltip(Some(tooltip));

    // Rebuild menu with or without the update item
    if let Ok(menu) = build_tray_menu(app, available) {
        let _ = tray.set_menu(Some(menu));
    }
}

fn load_tray_icon(has_update: bool) -> Image<'static> {
    if has_update {
        let bytes = include_bytes!("../icons/iconTemplateUpdate@2x.png");
        Image::from_bytes(bytes).expect("Failed to load update tray icon")
    } else {
        let bytes = include_bytes!("../icons/iconTemplate@2x.png");
        Image::from_bytes(bytes).expect("Failed to load tray icon")
    }
}

fn build_tray_menu(
    app: &tauri::AppHandle,
    has_update: bool,
) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    let mut menu_builder = MenuBuilder::new(app);

    // "Update Available..." item at the top when an update is pending
    if has_update {
        let update_item =
            MenuItemBuilder::with_id("tray-update", "Update Available...").build(app)?;
        menu_builder = menu_builder.item(&update_item).separator();
    }

    let show_item = MenuItemBuilder::with_id("show", "Show Goose").build(app)?;
    let new_window = MenuItemBuilder::with_id("tray-new-window", "New Window").build(app)?;
    let open_dir =
        MenuItemBuilder::with_id("tray-open-directory", "Open Directory...").build(app)?;

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
                let item =
                    MenuItemBuilder::with_id(&format!("tray-recent-{}", i), short).build(app)?;
                builder = builder.item(&item);
            }
        }
        builder.build()?
    };

    let quit_item = MenuItemBuilder::with_id("quit", "Quit").build(app)?;

    let menu = menu_builder
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
        "tray-update" => {
            // Open settings and navigate to the update section
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
                let _ = window.emit("set-view", ("settings", "update"));
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
