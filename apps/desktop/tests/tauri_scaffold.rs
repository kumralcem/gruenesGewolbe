use std::fs;
use std::path::Path;

#[test]
fn tauri_scaffold_points_at_the_workbench_ui_and_commands() {
    let app_root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let config = fs::read_to_string(app_root.join("tauri.conf.json")).expect("read Tauri config");
    assert!(config.contains("\"frontendDist\": \"./ui\""));
    assert!(config.contains("\"identifier\": \"dev.gruenesgewolbe.app\""));

    let html = fs::read_to_string(app_root.join("ui/index.html")).expect("read workbench HTML");
    assert!(html.contains("id=\"subvaults\""));
    assert!(html.contains("id=\"artwork-grid\""));
    assert!(html.contains("id=\"item-details\""));

    let script = fs::read_to_string(app_root.join("ui/main.js")).expect("read workbench script");
    assert!(script.contains("invoke(\"create_vault\""));
    assert!(script.contains("invoke(\"import_paintings\""));
    assert!(script.contains("invoke(\"capture_idea\""));
    assert!(script.contains("invoke(\"workbench_snapshot\""));
}
