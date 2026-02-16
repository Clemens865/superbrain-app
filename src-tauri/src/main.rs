// Prevents additional console window on Windows in release
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code)]

mod ai;
mod brain;
mod commands;
mod context;
mod indexer;
mod overlay;
mod state;
mod tray;
mod workflows;

use state::AppState;
use tauri::Manager;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "superbrain_app=info".into()),
        )
        .init();

    tracing::info!("SuperBrain starting...");

    tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_shell::init())
        .setup(|app| {
            // Set activation policy to accessory (menu bar only, no dock icon)
            #[cfg(target_os = "macos")]
            {
                app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            }

            // Initialize application state
            let app_state = AppState::new().expect("Failed to initialize SuperBrain");

            // Try to initialize Ollama embeddings in background
            let embeddings = app_state.embeddings.clone();
            tauri::async_runtime::spawn(async move {
                embeddings.try_init_ollama().await;
            });

            app.manage(app_state);

            // Setup system tray
            tray::setup_tray(app.handle())?;

            // Setup global shortcut (Cmd+Shift+Space)
            use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut};
            let shortcut: Shortcut = "CmdOrCtrl+Shift+Space"
                .parse()
                .expect("Failed to parse shortcut");

            let handle = app.handle().clone();
            app.global_shortcut().on_shortcut(shortcut, move |_app, _shortcut, _event| {
                overlay::toggle(&handle);
            })?;

            // Start file watcher for indexed directories
            let indexer_ref = app.state::<AppState>().indexer.clone();
            let watch_dirs = indexer::watcher::default_watch_dirs();
            indexer_ref.add_watch_dirs(watch_dirs.clone());
            match indexer::watcher::start_watcher(watch_dirs) {
                Ok((_watcher, mut rx)) => {
                    let idx = indexer_ref;
                    tauri::async_runtime::spawn(async move {
                        // Keep _watcher alive by moving it into the task
                        let _keep_alive = _watcher;
                        while let Some(change) = rx.recv().await {
                            let path = match &change {
                                indexer::watcher::FileChange::Created(p)
                                | indexer::watcher::FileChange::Modified(p) => Some(p.clone()),
                                indexer::watcher::FileChange::Deleted(_) => None,
                            };
                            if let Some(path) = path {
                                tracing::debug!("File changed, re-indexing: {:?}", path);
                                let _ = idx.index_file(&path).await;
                            }
                        }
                    });
                    tracing::info!("File watcher started");
                }
                Err(e) => {
                    tracing::warn!("Failed to start file watcher: {}", e);
                }
            }

            // Start background cognitive cycle task
            let engine = app
                .state::<AppState>()
                .engine
                .clone();
            let persistence = app
                .state::<AppState>()
                .persistence
                .clone();

            tauri::async_runtime::spawn(async move {
                let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(60));
                loop {
                    interval.tick().await;
                    // Run a cognitive cycle
                    let _ = engine.cycle();
                    // Periodic flush
                    let nodes = engine.memory.all_nodes();
                    let _ = persistence.store_memories_batch(&nodes);
                    tracing::debug!("Background cycle completed");
                }
            });

            // Start overlay hidden
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.hide();
            }

            tracing::info!("SuperBrain initialized successfully");
            Ok(())
        })
        .on_window_event(|window, event| {
            // Hide window on blur (click outside)
            if let tauri::WindowEvent::Focused(false) = event {
                let _ = window.hide();
            }
        })
        .invoke_handler(tauri::generate_handler![
            commands::think,
            commands::remember,
            commands::recall,
            commands::get_status,
            commands::get_settings,
            commands::update_settings,
            commands::get_thoughts,
            commands::get_stats,
            commands::evolve,
            commands::cycle,
            commands::search_files,
            commands::index_files,
            commands::run_workflow,
            commands::check_ollama,
            commands::flush,
        ])
        .run(tauri::generate_context!())
        .expect("Error while running SuperBrain");
}

fn main() {
    run();
}
