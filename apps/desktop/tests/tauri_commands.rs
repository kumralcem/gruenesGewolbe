use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::{
    CaptureIdeaCommand, ImportPaintingsCommand, TauriCommandState, WorkbenchSnapshotCommand,
};

#[test]
fn tauri_commands_build_a_frontend_ready_workbench_snapshot() {
    let root = temp_path("tauri-command-vault");
    let source_dir = temp_path("tauri-command-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg"),
        b"known painting bytes",
    )
    .expect("write source painting");

    let mut state = TauriCommandState::default();
    let active = state
        .create_vault(root.display().to_string())
        .expect("create vault through command");
    assert_eq!(active.root, root.display().to_string());

    let imported = state
        .import_paintings(ImportPaintingsCommand {
            source_folder: source_dir.display().to_string(),
        })
        .expect("import paintings through command");
    assert_eq!(imported.len(), 1);

    state
        .capture_idea(CaptureIdeaCommand {
            source_link: "https://example.com/nocturne-note".to_string(),
            title: "Nocturne note".to_string(),
            saving_reason: Some("Useful framing for night palettes".to_string()),
            copied_text: Some("Main idea text from the page.".to_string()),
        })
        .expect("capture idea through command");

    let snapshot = state
        .workbench_snapshot(WorkbenchSnapshotCommand {
            home_subvault: "Paintings".to_string(),
            search_query: Some("nocturne".to_string()),
            selected_item_id: Some(imported[0].id.clone()),
        })
        .expect("build snapshot through command");

    assert_eq!(snapshot.active_vault.root, root.display().to_string());
    assert!(snapshot
        .subvaults
        .iter()
        .any(|subvault| subvault == "Paintings"));
    assert_eq!(snapshot.artwork_items[0].title, "Nocturne Study");
    assert_eq!(snapshot.idea_sources[0].title, "Nocturne note");
    assert!(snapshot
        .search_results
        .iter()
        .any(|result| result.id == imported[0].id));
    assert_eq!(
        snapshot.selected_item.expect("selected item").title,
        "Nocturne Study"
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn tauri_commands_return_restart_safe_desktop_startup_state() {
    let root = temp_path("tauri-startup-vault");
    let app_state = temp_path("tauri-startup-app-state");

    let mut state = TauriCommandState::with_app_state_dir(&app_state);
    let initial = state.startup().expect("initial startup state");
    assert!(initial.active_vault.is_none());
    assert!(initial.known_vaults.is_empty());
    assert!(initial.notice.is_none());

    state
        .create_vault(root.display().to_string())
        .expect("create through command state");
    drop(state);

    let restored = TauriCommandState::with_app_state_dir(&app_state)
        .startup()
        .expect("restored startup state");
    assert_eq!(
        restored.active_vault.as_ref().expect("active vault").root,
        root.display().to_string()
    );
    assert_eq!(restored.known_vaults[0].root, root.display().to_string());
    assert!(restored.notice.is_none());
    assert_eq!(
        serde_json::to_value(&restored).expect("serialize startup state"),
        serde_json::json!({
            "active_vault": { "root": root.display().to_string() },
            "known_vaults": [{ "root": root.display().to_string() }],
            "notice": null
        })
    );

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&app_state).expect("clean app state");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
