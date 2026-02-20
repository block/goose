// The `objc` 0.2 macros (sel!, msg_send!, class!) reference cfg(feature = "cargo-clippy")
// which triggers unexpected_cfgs warnings on newer rustc. Suppress until objc is updated.
#![allow(unexpected_cfgs)]

#[cfg(target_os = "macos")]
use tauri::Manager;

/// Show or hide the macOS dock icon at runtime.
///
/// On non-macOS platforms this is a no-op.
pub fn set_dock_visible(#[allow(unused)] app: &tauri::AppHandle, #[allow(unused)] visible: bool) {
    #[cfg(target_os = "macos")]
    {
        if visible {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
        } else {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            // Re-focus the window after hiding dock icon
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }
    }
}

/// Set up a custom dock menu with a "New Window" item on macOS.
///
/// On non-macOS platforms this is a no-op.
pub fn setup_dock_menu(#[allow(unused)] app: &tauri::AppHandle) {
    #[cfg(target_os = "macos")]
    {
        use objc::declare::ClassDecl;
        use objc::runtime::{Class, Object, Sel};
        use objc::{class, msg_send, sel, sel_impl};

        use std::sync::OnceLock;

        static APP_HANDLE: OnceLock<tauri::AppHandle> = OnceLock::new();
        APP_HANDLE.get_or_init(|| app.clone());

        extern "C" fn new_window_action(_this: &Object, _cmd: Sel, _sender: *mut Object) {
            if let Some(app) = APP_HANDLE.get() {
                let handle = app.clone();
                tauri::async_runtime::spawn(async move {
                    let _ = crate::commands::create_chat_window(
                        handle, None, None, None, None, None, None,
                    )
                    .await;
                });
            }
        }

        unsafe {
            let superclass = Class::get("NSObject").expect("NSObject class not found");
            if Class::get("GooseDockMenuDelegate").is_none() {
                let mut decl = ClassDecl::new("GooseDockMenuDelegate", superclass)
                    .expect("Failed to create GooseDockMenuDelegate class");
                decl.add_method(
                    sel!(newWindowAction:),
                    new_window_action
                        as extern "C" fn(&Object, Sel, *mut Object),
                );
                decl.register();
            }

            let delegate_class =
                Class::get("GooseDockMenuDelegate").expect("GooseDockMenuDelegate not registered");
            let delegate: *mut Object = msg_send![delegate_class, new];

            // Build NSMenu
            let menu: *mut Object = msg_send![class!(NSMenu), new];

            // Build NSMenuItem "New Window"
            let title = cocoa_string("New Window");
            let key_equiv = cocoa_string("");
            let item: *mut Object = msg_send![class!(NSMenuItem), alloc];
            let item: *mut Object = msg_send![
                item,
                initWithTitle: title
                action: sel!(newWindowAction:)
                keyEquivalent: key_equiv
            ];
            let _: () = msg_send![item, setTarget: delegate];
            let _: () = msg_send![menu, addItem: item];

            // Set dock menu on NSApplication
            let ns_app: *mut Object = msg_send![class!(NSApplication), sharedApplication];
            let _: () = msg_send![ns_app, setDockMenu: menu];
        }
    }
}

/// Create an NSString from a Rust &str (macOS only).
#[cfg(target_os = "macos")]
unsafe fn cocoa_string(s: &str) -> *mut objc::runtime::Object {
    use objc::{class, msg_send, sel, sel_impl};
    let ns_string: *mut objc::runtime::Object = msg_send![class!(NSString), alloc];
    let ns_string: *mut objc::runtime::Object = msg_send![
        ns_string,
        initWithBytes: s.as_ptr()
        length: s.len()
        encoding: 4u64 // NSUTF8StringEncoding
    ];
    ns_string
}
