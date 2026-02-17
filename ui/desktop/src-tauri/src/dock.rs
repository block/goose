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
