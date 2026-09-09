use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::{
    AddArtworkFilesCommand, TauriCommandState, WorkbenchSnapshotCommand,
};

#[test]
fn desktop_snapshot_localizes_malformed_records_and_refreshes_after_external_correction() {
    let root = temp_path("desktop-malformed-records-vault");
    let source_dir = temp_path("desktop-malformed-records-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let nocturne = source_dir.join("Nocturne.png");
    let garden = source_dir.join("Garden.png");
    fs::write(&nocturne, b"nocturne fixture").expect("write first source");
    fs::write(&garden, b"garden fixture").expect("write second source");

    let mut state = TauriCommandState::default();
    state
        .create_vault(root.display().to_string())
        .expect("create Vault");
    let imported = state
        .add_artwork_files(AddArtworkFilesCommand {
            source_files: vec![nocturne.display().to_string(), garden.display().to_string()],
            creator: Some("Archive Artist".to_string()),
            year: Some("2024".to_string()),
            saving_reason: None,
            import_exact_duplicates: false,
        })
        .expect("add artwork");
    let malformed_folder = PathBuf::from(&imported.imported_items[1].item_folder);
    let malformed_path = malformed_folder.join("record.md");
    let valid_record = fs::read(&malformed_path).expect("read valid Item Record");
    fs::write(&malformed_path, "---\nid: [broken\n---\n").expect("malform Item Record");

    let snapshot = state
        .workbench_snapshot(WorkbenchSnapshotCommand {
            home_subvault: "Paintings".to_string(),
            artwork_sort: "newest".to_string(),
            search_query: Some("nocturne".to_string()),
            selected_item_id: Some(imported.imported_items[0].id.clone()),
        })
        .expect("build resilient workbench snapshot");
    assert_eq!(snapshot.artwork_items.len(), 1);
    assert_eq!(snapshot.search_results.len(), 1);
    assert_eq!(snapshot.search_results[0].title, "Nocturne");
    assert_eq!(snapshot.vault_problems.len(), 1);
    assert_eq!(snapshot.vault_problems[0].path, malformed_path.display().to_string());
    assert!(snapshot.vault_problems[0].error.contains("YAML parse error"));
    assert!(snapshot.selected_item.is_some());
    assert!(PathBuf::from(state.activity_log_path().expect("Activity Log path")).is_file());

    fs::write(&malformed_path, valid_record).expect("correct Item Record externally");
    let refreshed = state
        .refresh_workbench(WorkbenchSnapshotCommand {
            home_subvault: "Paintings".to_string(),
            artwork_sort: "newest".to_string(),
            search_query: Some("garden".to_string()),
            selected_item_id: Some(imported.imported_items[1].id.clone()),
        })
        .expect("refresh corrected Item Record");
    assert_eq!(refreshed.artwork_items.len(), 2);
    assert_eq!(refreshed.search_results.len(), 1);
    assert_eq!(refreshed.search_results[0].title, "Garden");
    assert!(refreshed.vault_problems.is_empty());
    assert_eq!(refreshed.selected_item.expect("corrected Item Details").title, "Garden");

    fs::remove_dir_all(&root).expect("clean Vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
