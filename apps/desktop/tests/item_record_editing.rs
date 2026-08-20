use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::{
    ConfirmItemFolderRenameCommand, ImportPaintingsCommand, ItemRecordSaveView,
    SaveItemRecordCommand, TauriCommandState, WorkbenchSnapshotCommand,
};

#[test]
fn tauri_commands_save_conflict_reload_overwrite_and_confirm_rename() {
    let root = temp_path("desktop-record-edit-vault");
    let source = temp_path("desktop-record-edit-source");
    fs::create_dir_all(&source).expect("create sources");
    fs::write(source.join("draft.jpg"), b"image bytes").expect("write source");
    let mut state = TauriCommandState::default();
    state
        .create_vault(root.display().to_string())
        .expect("create vault");
    let imported = state
        .import_paintings(ImportPaintingsCommand {
            source_folder: source.display().to_string(),
        })
        .expect("import artwork");
    let item_id = imported[0].id.clone();
    let loaded = selected_item(&state, &item_id);

    let first = state
        .save_item_record(SaveItemRecordCommand {
            id: item_id.clone(),
            expected_revision: loaded.record_revision,
            overwrite_conflict: false,
            title: "Nocturne Study".to_string(),
            creator: "Jane Painter".to_string(),
            year: "1884".to_string(),
            saving_reason: "Palette reference".to_string(),
            summary: "Blue-black night tones.".to_string(),
            tags: vec!["night".to_string()],
        })
        .expect("save through command");
    let ItemRecordSaveView::Saved { item: saved } = first else {
        panic!("expected saved result");
    };
    assert_eq!(saved.title, "Nocturne Study");
    assert_eq!(saved.summary, Some("Blue-black night tones.".to_string()));

    let record_path = PathBuf::from(&saved.item_folder).join("record.md");
    let externally_changed = fs::read_to_string(&record_path)
        .expect("read record")
        .replace("title: Nocturne Study", "title: External Title");
    fs::write(&record_path, externally_changed).expect("edit record externally");
    let conflict = state
        .save_item_record(SaveItemRecordCommand {
            id: item_id.clone(),
            expected_revision: saved.record_revision,
            overwrite_conflict: false,
            title: "Editor Title".to_string(),
            creator: saved.creator,
            year: saved.year,
            saving_reason: saved.saving_reason.unwrap_or_default(),
            summary: saved.summary.unwrap_or_default(),
            tags: saved.tags,
        })
        .expect("return conflict as command outcome");
    let ItemRecordSaveView::Conflict { external_item } = conflict else {
        panic!("expected conflict result");
    };
    assert_eq!(external_item.title, "External Title");

    let overwritten = state
        .save_item_record(SaveItemRecordCommand {
            id: item_id.clone(),
            expected_revision: external_item.record_revision,
            overwrite_conflict: true,
            title: "Editor Title".to_string(),
            creator: external_item.creator,
            year: external_item.year,
            saving_reason: external_item.saving_reason.unwrap_or_default(),
            summary: external_item.summary.unwrap_or_default(),
            tags: external_item.tags,
        })
        .expect("deliberately overwrite through command");
    assert!(matches!(overwritten, ItemRecordSaveView::Saved { .. }));
    let proposal = selected_item(&state, &item_id)
        .folder_rename_proposal
        .expect("receive rename proposal");

    let renamed = state
        .confirm_item_folder_rename(ConfirmItemFolderRenameCommand {
            id: item_id.clone(),
            current_path: proposal.current_path,
            proposed_path: proposal.proposed_path,
        })
        .expect("confirm rename through command");
    assert_eq!(renamed.id, item_id);
    assert!(renamed
        .item_folder
        .ends_with("Jane Painter - 1884 - Editor Title"));

    fs::remove_dir_all(root).expect("clean vault");
    fs::remove_dir_all(source).expect("clean sources");
}

fn selected_item(
    state: &TauriCommandState,
    item_id: &str,
) -> gruenes_gewolbe_desktop::ItemDetailsView {
    state
        .workbench_snapshot(WorkbenchSnapshotCommand {
            home_subvault: "Paintings".to_string(),
            artwork_sort: "newest".to_string(),
            search_query: None,
            selected_item_id: Some(item_id.to_string()),
        })
        .expect("load details")
        .selected_item
        .expect("selected item")
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
