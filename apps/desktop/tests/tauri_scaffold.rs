use std::{fs, path::Path};

#[test]
fn tauri_scaffold_points_at_the_vite_ui_and_linux_runtime() {
    let app_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let config = fs::read_to_string(app_root.join("tauri.conf.json")).expect("read Tauri config");
    assert!(config.contains("\"frontendDist\": \"./dist\""));
    assert!(config.contains("\"beforeDevCommand\": \"npm run dev\""));
    assert!(config.contains("\"identifier\": \"dev.gruenesgewolbe.app\""));

    let package = fs::read_to_string(app_root.join("package.json")).expect("read package");
    assert!(package.contains("\"@tauri-apps/cli\""));
    assert!(package.contains("\"vite\""));

    let runtime = fs::read_to_string(app_root.join("src/main.rs")).expect("read runtime");
    assert!(runtime.contains("tauri_plugin_dialog::init()"));
    assert!(runtime.contains("fn startup("));
    assert!(runtime.contains("fn create_vault("));
    assert!(runtime.contains("fn open_vault("));
    assert!(runtime.contains("tauri::generate_handler![startup, create_vault, open_vault]"));
}
