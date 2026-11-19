fn main() {
    let manifest = tauri_build::AppManifest::new().commands(&[
        "active_shortcut",
        "execute_alias",
        "execute_appearance",
        "execute_search",
        "execute_open_target",
        "exit_app",
    ]);
    tauri_build::try_build(tauri_build::Attributes::new().app_manifest(manifest))
        .expect("failed to generate Tauri build metadata");
}
