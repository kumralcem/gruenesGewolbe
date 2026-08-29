use std::{fs, path::Path};

#[test]
fn tauri_scaffold_points_at_the_vite_ui_and_linux_runtime() {
    let app_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let config = fs::read_to_string(app_root.join("tauri.conf.json")).expect("read Tauri config");
    assert!(config.contains("\"frontendDist\": \"./dist\""));
    assert!(config.contains("\"beforeDevCommand\": \"pnpm dev\""));
    assert!(config.contains("\"identifier\": \"dev.gruenesgewolbe.app\""));
    let config_json: serde_json::Value = serde_json::from_str(&config).expect("parse Tauri config");
    assert_eq!(
        config_json["app"]["security"]["assetProtocol"]["enable"],
        false
    );

    let package = fs::read_to_string(app_root.join("package.json")).expect("read package");
    assert!(package.contains("\"@tauri-apps/cli\""));
    assert!(package.contains("\"vite\""));

    let runtime = fs::read_to_string(app_root.join("src/main.rs")).expect("read runtime");
    assert!(runtime.contains("tauri_plugin_dialog::init()"));
    assert!(runtime.contains("fn startup("));
    assert!(runtime.contains("fn create_vault("));
    assert!(runtime.contains("fn open_vault("));
    assert!(runtime.contains("fn confirm_vault_repair("));
    assert!(runtime.contains("fn cancel_vault_repair("));
    assert!(runtime.contains("async fn capture_source_link("));
    assert!(runtime.contains("fn capture_manual_fallback("));
    assert!(runtime.contains("register_uri_scheme_protocol(\"vault-media\""));
    assert!(runtime.contains("read_active_vault_thumbnail"));
    assert!(runtime.contains("tauri::generate_handler!["));

    let adapter = fs::read_to_string(app_root.join("src/tauri-adapter.ts")).expect("read adapter");
    assert!(adapter.contains("convertFileSrc(path, \"vault-media\")"));
}
