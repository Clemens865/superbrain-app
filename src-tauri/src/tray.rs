//! System tray management for SuperBrain

use tauri::{
    menu::{Menu, MenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    AppHandle, Emitter, Manager,
};

/// Set up the system tray icon and menu
pub fn setup_tray(app: &AppHandle) -> Result<(), Box<dyn std::error::Error>> {
    let show = MenuItem::with_id(app, "show", "Show SuperBrain", true, None::<&str>)?;
    let status = MenuItem::with_id(app, "status", "Status: Running", false, None::<&str>)?;
    let separator = MenuItem::with_id(app, "sep1", "---", false, None::<&str>)?;
    let settings = MenuItem::with_id(app, "settings", "Settings...", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "Quit SuperBrain", true, None::<&str>)?;

    let menu = Menu::with_items(app, &[&show, &status, &separator, &settings, &quit])?;

    let _tray = TrayIconBuilder::new()
        .menu(&menu)
        .tooltip("SuperBrain - Cognitive Assistant")
        .icon(app.default_window_icon().cloned().unwrap())
        .on_menu_event(move |app, event| {
            match event.id.as_ref() {
                "show" => {
                    toggle_overlay(app);
                }
                "settings" => {
                    if let Some(window) = app.get_webview_window("main") {
                        let _ = window.show();
                        let _ = window.set_focus();
                        // Emit event to navigate to settings
                        let _ = window.emit("navigate", "settings");
                    }
                }
                "quit" => {
                    // Flush state before quitting
                    if let Some(state) = app.try_state::<crate::state::AppState>() {
                        let _ = state.flush();
                    }
                    app.exit(0);
                }
                _ => {}
            }
        })
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click {
                button: MouseButton::Left,
                button_state: MouseButtonState::Up,
                ..
            } = event
            {
                toggle_overlay(tray.app_handle());
            }
        })
        .build(app)?;

    Ok(())
}

/// Toggle the overlay window visibility
fn toggle_overlay(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        if window.is_visible().unwrap_or(false) {
            let _ = window.hide();
        } else {
            let _ = window.show();
            let _ = window.set_focus();
            let _ = window.center();
        }
    }
}
