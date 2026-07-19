use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::{
    CaptureIdeaCommand, ImportPaintingsCommand, OpenVaultView, TauriCommandState,
    WorkbenchSnapshotCommand,
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
    let open_app_state = temp_path("tauri-open-app-state");

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
            "repair_proposal": null,
            "notice": null
        })
    );

    let mut opening_state = TauriCommandState::with_app_state_dir(&open_app_state);
    let opened = opening_state
        .open_vault(root.display().to_string())
        .expect("open existing vault through command state");
    assert_eq!(
        opened,
        OpenVaultView::Opened {
            vault: gruenes_gewolbe_desktop::ActiveVaultView {
                root: root.display().to_string()
            }
        }
    );

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&app_state).expect("clean app state");
    fs::remove_dir_all(&open_app_state).expect("clean open app state");
}

#[test]
fn tauri_commands_serialize_cancel_and_confirm_a_vault_repair() {
    let root = temp_path("tauri-repair-vault");
    fs::create_dir_all(root.join("subvaults")).expect("create existing structure");
    fs::write(
        root.join("vault.toml"),
        "format_version = 2\nname = \"Archive\"\n",
    )
    .expect("write vault config");
    let mut state = TauriCommandState::default();

    let proposed = state
        .open_vault(root.display().to_string())
        .expect("propose repair through command state");
    assert_eq!(
        serde_json::to_value(&proposed).expect("serialize repair proposal"),
        serde_json::json!({
            "status": "repair_required",
            "proposal": {
                "root": root.display().to_string(),
                "directories": [root.join("collections").display().to_string()]
            }
        })
    );

    state
        .cancel_vault_repair(root.display().to_string())
        .expect("cancel repair through command state");
    assert!(!root.join("collections").exists());

    state
        .open_vault(root.display().to_string())
        .expect("propose repair again");
    let repaired = state
        .confirm_vault_repair(root.display().to_string())
        .expect("confirm repair through command state");
    assert_eq!(repaired.root, root.display().to_string());
    assert!(root.join("collections").is_dir());

    fs::remove_dir_all(&root).expect("clean repaired vault");
}

#[test]
fn tauri_open_command_rejects_a_non_repairable_structural_conflict() {
    let root = temp_path("tauri-unsafe-repair");
    fs::create_dir_all(&root).expect("create vault root");
    fs::write(
        root.join("vault.toml"),
        "format_version = 2\nname = \"Archive\"\n",
    )
    .expect("write vault config");
    fs::write(root.join("subvaults"), b"do not overwrite").expect("write conflicting file");
    let mut state = TauriCommandState::default();

    let error = state
        .open_vault(root.display().to_string())
        .expect_err("unsafe conflict should fail through command state");

    assert_eq!(
        error.to_string(),
        format!(
            "required vault directory conflicts with an existing file: {}",
            root.join("subvaults").display()
        )
    );
    assert_eq!(
        fs::read(root.join("subvaults")).expect("read conflicting file"),
        b"do not overwrite"
    );
    assert!(!root.join("collections").exists());

    fs::remove_dir_all(&root).expect("clean conflicting vault");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
