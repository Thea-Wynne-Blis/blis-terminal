fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "active_shortcut",
        "execute_alias",
        "execute_appearance",
        "execute_directory",
        "execute_note",
        "execute_search",
        "execute_open_target",
        "execute_ping",
        "execute_todo",
        "execute_welcome",
        "exit_app",
        "start_timer",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to generate Tauri build metadata");
}
