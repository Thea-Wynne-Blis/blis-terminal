#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod features;
mod platform;

use app::lifecycle;
use features::{
    aliases,
    appearance::{background, welcome},
    files::{directories, open_targets},
    notes, search_settings,
    tasks::{timers, todos},
};

use std::sync::Mutex;
use tauri::{AppHandle, Manager, State, WindowEvent};
use tauri_plugin_global_shortcut::{Builder as ShortcutPlugin, ShortcutState};

struct RegisteredShortcut(Option<&'static str>);

#[tauri::command]
fn active_shortcut(shortcut: State<'_, RegisteredShortcut>) -> Option<&'static str> {
    shortcut.0
}

#[tauri::command]
fn exit_app(app: AppHandle) {
    app.exit(0);
}

fn main() {
    let start_hidden = std::env::args_os().any(|arg| arg == "--background");

    tauri::Builder::default()
        // The single-instance plugin must see secondary launches before other plugins initialize.
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            lifecycle::wake(app);
        }))
        .plugin(
            ShortcutPlugin::new()
                .with_handler(|app, _, event| {
                    if event.state() == ShortcutState::Pressed {
                        lifecycle::toggle(app);
                    }
                })
                .build(),
        )
        .manage(Mutex::new(()))
        .invoke_handler(tauri::generate_handler![
            active_shortcut,
            aliases::execute_alias,
            background::execute_appearance,
            directories::execute_directory,
            exit_app,
            notes::execute_note,
            open_targets::execute_open_target,
            search_settings::execute_search,
            timers::start_timer,
            todos::execute_todo,
            welcome::execute_welcome
        ])
        .on_window_event(|window, event| {
            if window.label() == lifecycle::TERMINAL_LABEL {
                if let WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    if let Err(error) = lifecycle::hide_terminal(window.app_handle()) {
                        eprintln!("Failed to hide terminal: {error}");
                    }
                }
            }
        })
        .setup(move |app| {
            let home = match app.path().home_dir() {
                Ok(home) => home,
                Err(_) => std::env::current_dir()?,
            };
            app.manage(directories::DirectoryStore::new(home));
            let config_directory = app.path().app_config_dir()?;
            app.manage(notes::NoteStore::new(config_directory.join("notes")));
            app.manage(aliases::AliasStore::new(
                config_directory.join("aliases.json"),
            ));
            app.manage(open_targets::OpenTargets::new(
                config_directory.join("open-targets.json"),
            ));
            app.manage(search_settings::SearchStore::new(
                config_directory.join("search-settings.json"),
            ));
            app.manage(todos::TodoStore::new(config_directory.join("todos.json")));
            app.manage(welcome::WelcomeStore::new(
                config_directory.join("welcome.json"),
            ));
            app.manage(timers::TimerNotices::default());
            app.manage(background::AppearanceStore::new(config_directory));
            lifecycle::create_tray(app.handle())?;
            app.manage(RegisteredShortcut(lifecycle::register_shortcut(
                app.handle(),
            )));
            if !start_hidden {
                lifecycle::wake(app.handle());
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("failed to run Blis Terminal");
}
