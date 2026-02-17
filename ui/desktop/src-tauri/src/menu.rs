use tauri::{
    menu::{MenuBuilder, MenuItemBuilder, SubmenuBuilder},
    Emitter, Manager,
};

pub fn setup_menu(app: &tauri::App) -> Result<(), Box<dyn std::error::Error>> {
    let new_chat = MenuItemBuilder::with_id("new-chat", "New Chat")
        .accelerator("CmdOrCtrl+T")
        .build(app)?;
    let new_window = MenuItemBuilder::with_id("new-window", "New Window")
        .accelerator("CmdOrCtrl+N")
        .build(app)?;
    let open_dir = MenuItemBuilder::with_id("open-directory", "Open Directory...")
        .accelerator("CmdOrCtrl+O")
        .build(app)?;
    let settings_item = MenuItemBuilder::with_id("settings", "Settings...")
        .accelerator("CmdOrCtrl+,")
        .build(app)?;

    let file_menu = SubmenuBuilder::new(app, "File")
        .items(&[&new_chat, &new_window, &open_dir, &settings_item])
        .build()?;

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

    let always_on_top = MenuItemBuilder::with_id("always-on-top", "Always on Top")
        .accelerator("CmdOrCtrl+Shift+T")
        .build(app)?;

    let window_menu = SubmenuBuilder::new(app, "Window")
        .minimize()
        .items(&[&always_on_top])
        .build()?;

    let menu = MenuBuilder::new(app)
        .items(&[&file_menu, &edit_menu, &window_menu])
        .build()?;

    app.set_menu(menu)?;

    app.on_menu_event(move |app, event| {
        let id = event.id().as_ref();
        match id {
            "new-chat" | "new-window" | "find" | "find-next" | "find-previous" | "settings"
            | "open-directory" | "always-on-top" => {
                if let Some(window) = app.get_webview_window("main") {
                    let _ = window.emit(id, ());
                }
            }
            _ => {}
        }
    });

    Ok(())
}
