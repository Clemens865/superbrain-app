//! Overlay window management for SuperBrain

use tauri::{AppHandle, Emitter, Manager};

/// Toggle the overlay window
pub fn toggle(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            hide(app);
        } else {
            show(app);
        }
    }
}

/// Show the overlay window
pub fn show(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.center();
        let _ = window.show();
        let _ = window.set_focus();
        let _ = window.emit("overlay-shown", ());
    }
}

/// Hide the overlay window
pub fn hide(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.hide();
        let _ = window.emit("overlay-hidden", ());
    }
}
