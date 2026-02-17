use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    Emitter, Manager,
};

pub fn setup_menu(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let menu = build_app_menu(app)?;
    app.set_menu(menu)?;

    app.on_menu_event(handle_menu_event);

    Ok(())
}

fn build_app_menu(
    app: &tauri::App,
) -> Result<tauri::menu::Menu<tauri::Wry>, Box<dyn std::error::Error>> {
    // File menu
    let new_chat = MenuItemBuilder::with_id("new-chat", "New Chat")
        .accelerator("CmdOrCtrl+T")
        .build(app)?;
    let new_window = MenuItemBuilder::with_id("new-window", "New Window")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open_dir = MenuItemBuilder::with_id("open-directory", "Open Directory...")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;

    // Recent Directories submenu
    let recent_dirs = load_recent_dirs();
    let recent_submenu = {
        let mut builder = SubmenuBuilder::new(app, "Recent Directories");
        if recent_dirs.is_empty() {
            let none_item =
                MenuItemBuilder::with_id("menu-no-recent", "No Recent Directories").build(app)?;
            builder = builder.item(&none_item);
        } else {
            for (i, dir) in recent_dirs.iter().enumerate().take(10) {
                let short = dir.split('/').last().unwrap_or(dir);
                let item = MenuItemBuilder::with_id(&format!("menu-recent-{}", i), short)
                    .build(app)?;
                builder = builder.item(&item);
            }
        }
        builder.build()?
    };

    let settings_item = MenuItemBuilder::with_id("settings", "Settings...")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .items(&[&new_chat, &new_window, &open_dir])
        .item(&recent_submenu)
        .separator()
        .item(&settings_item)
        .build()?;

    // Edit menu
    let find_item = MenuItemBuilder::with_id("find", "Find")
        .accelerator("CmdOrCtrl+F")
        .build(app)?;
    let find_next = MenuItemBuilder::with_id("find-next", "Find Next")
        .accelerator("CmdOrCtrl+G")
        .build(app)?;
    let find_prev = MenuItemBuilder::with_id("find-previous", "Find Previous")
        .accelerator("CmdOrCtrl+Shift+G")
        .build(app)?;

    let edit_menu = SubmenuBuilder::new(app, "Edit")
        .undo()
        .redo()
        .separator()
        .cut()
        .copy()
        .paste()
        .select_all()
        .separator()
        .items(&[&find_item, &find_next, &find_prev])
        .build()?;

    // Window menu
    let always_on_top = MenuItemBuilder::with_id("always-on-top", "Always on Top")
        .accelerator("CmdOrCtrl+Shift+T")
        .build(app)?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .items(&[&always_on_top])
        .build()?;

    // Help menu
    let docs_item = MenuItemBuilder::with_id("help-docs", "Documentation").build(app)?;
    let github_item = MenuItemBuilder::with_id("help-github", "GitHub Repository").build(app)?;
    let about_item = MenuItemBuilder::with_id("help-about", "About Goose").build(app)?;

    let help_menu = SubmenuBuilder::new(app, "Help")
        .items(&[&docs_item, &github_item, &about_item])
        .build()?;

    let menu = MenuBuilder::new(app)
        .items(&[&file_menu, &edit_menu, &window_menu, &help_menu])
        .build()?;

    Ok(menu)
}

fn handle_menu_event(app: &tauri::AppHandle, event: tauri::menu::MenuEvent) {
    let id = event.id();
    let id_str = id.as_ref();
    match id_str {
        "new-chat" | "new-window" | "find" | "find-next" | "find-previous" | "settings"
        | "open-directory" | "always-on-top" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit(id_str, ());
            }
        }
        "help-docs" => {
            let _ = open::that("https://block.github.io/goose/");
        }
        "help-github" => {
            let _ = open::that("https://github.com/block/goose");
        }
        "help-about" => {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.emit("settings", ());
            }
        }
        other if other.starts_with("menu-recent-") => {
            if let Ok(idx) = other.trim_start_matches("menu-recent-").parse::<usize>() {
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
