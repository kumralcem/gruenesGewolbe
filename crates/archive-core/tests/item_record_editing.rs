use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Barrier};
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, ItemRecordEdit, TagDefinition, Vault, VaultError};

#[test]
fn structured_edit_saves_all_owned_fields_and_preserves_unknown_content() {
    let (root, source_dir, vault, item_id, record_path) = artwork_fixture("structured-save");
    vault
        .upsert_tag(TagDefinition {
            name: "Night Scenes".to_string(),
            aliases: vec!["nocturne".to_string()],
            meaning: None,
        })
        .expect("register canonical tag");
    let original = fs::read_to_string(&record_path).expect("read record");
    fs::write(
        &record_path,
        format!(
            "{}\n## Curator Notes\n\nKeep this prose exactly.\n",
            original.replace("title: draft", "title: draft\ncustom_field: keep-me")
        ),
    )
    .expect("add user-authored content");

    let loaded = vault.item_details(&item_id).expect("load editor");
    let saved = vault
        .save_item_record_edit(ItemRecordEdit {
            id: item_id.clone(),
            expected_revision: loaded.record_revision().to_string(),
            overwrite_conflict: false,
            title: "Nocturne Study".to_string(),
            creator: "Jane Painter".to_string(),
            year: "1884".to_string(),
            saving_reason: "Palette reference".to_string(),
            summary: "A study of blue-black night tones.".to_string(),
            tags: vec!["nocturne".to_string(), "Atmosphere".to_string()],
        })
        .expect("save complete structured edit");

    assert_eq!(saved.title(), "Nocturne Study");
    assert_eq!(saved.creator(), "Jane Painter");
    assert_eq!(saved.year(), "1884");
    assert_eq!(saved.saving_reason(), Some("Palette reference"));
    assert_eq!(saved.summary(), Some("A study of blue-black night tones."));
    assert_eq!(saved.tags(), vec!["Atmosphere", "Night Scenes"]);
    assert_ne!(saved.record_revision(), loaded.record_revision());

    let written = fs::read_to_string(&record_path).expect("read saved record");
    assert!(written.contains("custom_field: keep-me"));
    assert!(written.contains("## Curator Notes\n\nKeep this prose exactly."));
    assert!(written.contains("## Saving Reason\n\nPalette reference\n"));
    assert!(written.contains("## Summary\n\nA study of blue-black night tones.\n"));
    assert!(!record_path.with_extension("md.tmp").exists());

    let registry = fs::read_to_string(root.join("tag-registry.md")).expect("read registry");
    assert!(registry.contains("## Night Scenes"));
    assert!(registry.contains("## Atmosphere"));

    fs::remove_dir_all(root).expect("clean vault");
    fs::remove_dir_all(source_dir).expect("clean sources");
}

#[test]
fn external_change_requires_reload_or_deliberate_overwrite() {
    let (root, source_dir, vault, item_id, record_path) = artwork_fixture("edit-conflict");
    let loaded = vault.item_details(&item_id).expect("load editor");
    let external = fs::read_to_string(&record_path)
        .expect("read record")
        .replace("title: draft", "title: External File Title");
    fs::write(&record_path, &external).expect("make external edit");

    let edit = ItemRecordEdit {
        id: item_id.clone(),
        expected_revision: loaded.record_revision().to_string(),
        overwrite_conflict: false,
        title: "Structured Editor Title".to_string(),
        creator: "Unknown Creator".to_string(),
        year: "Unknown Year".to_string(),
        saving_reason: "Needs cleanup".to_string(),
        summary: String::new(),
        tags: Vec::new(),
    };
    let error = vault
        .save_item_record_edit(edit.clone())
        .expect_err("refuse stale ordinary save");
    assert!(matches!(error, VaultError::ItemRecordConflict { .. }));
    assert_eq!(
        fs::read_to_string(&record_path).expect("read conflict record"),
        external
    );

    let reloaded = vault
        .item_details(&item_id)
        .expect("reload external version");
    assert_eq!(reloaded.title(), "External File Title");
    assert_ne!(reloaded.record_revision(), loaded.record_revision());

    let second_external = fs::read_to_string(&record_path)
        .expect("read external record")
        .replace("title: External File Title", "title: Second External Title");
    fs::write(&record_path, second_external).expect("make second external edit");
    let second_conflict = vault
        .save_item_record_edit(ItemRecordEdit {
            expected_revision: reloaded.record_revision().to_string(),
            overwrite_conflict: true,
            ..edit.clone()
        })
        .expect_err("refuse overwrite after another unreviewed change");
    assert!(matches!(
        second_conflict,
        VaultError::ItemRecordConflict { .. }
    ));
    let reviewed_again = vault.item_details(&item_id).expect("review second edit");

    let overwritten = vault
        .save_item_record_edit(ItemRecordEdit {
            expected_revision: reviewed_again.record_revision().to_string(),
            overwrite_conflict: true,
            ..edit
        })
        .expect("deliberately overwrite after review");
    assert_eq!(overwritten.title(), "Structured Editor Title");

    fs::remove_dir_all(root).expect("clean vault");
    fs::remove_dir_all(source_dir).expect("clean sources");
}

#[test]
fn complete_edit_is_validated_before_the_record_changes() {
    let (root, source_dir, vault, item_id, record_path) = artwork_fixture("edit-validation");
    let loaded = vault.item_details(&item_id).expect("load editor");
    let original = fs::read_to_string(&record_path).expect("read original record");

    let error = vault
        .save_item_record_edit(ItemRecordEdit {
            id: item_id,
            expected_revision: loaded.record_revision().to_string(),
            overwrite_conflict: false,
            title: " ".to_string(),
            creator: "Jane Painter".to_string(),
            year: "eighteen eighty-four".to_string(),
            saving_reason: String::new(),
            summary: String::new(),
            tags: vec![String::new()],
        })
        .expect_err("reject invalid complete edit");

    let VaultError::InvalidItemRecordEdit(reasons) = error else {
        panic!("expected validation error");
    };
    assert_eq!(
        reasons,
        vec![
            "title must not be empty",
            "year must be four digits or Unknown Year",
            "tags must not contain empty values",
        ]
    );
    assert_eq!(
        fs::read_to_string(record_path).expect("read unchanged record"),
        original
    );

    fs::remove_dir_all(root).expect("clean vault");
    fs::remove_dir_all(source_dir).expect("clean sources");
}

#[test]
fn confirmed_folder_rename_preserves_id_and_uses_a_small_collision_suffix() {
    let (root, source_dir, vault, item_id, _) = artwork_fixture("confirmed-rename");
    let loaded = vault.item_details(&item_id).expect("load editor");
    let saved = vault
        .save_item_record_edit(ItemRecordEdit {
            id: item_id.clone(),
            expected_revision: loaded.record_revision().to_string(),
            overwrite_conflict: false,
            title: "Nocturne Study".to_string(),
            creator: "Jane Painter".to_string(),
            year: "1884".to_string(),
            saving_reason: String::new(),
            summary: String::new(),
            tags: Vec::new(),
        })
        .expect("save improved metadata");
    let old_path = saved.item_folder().to_path_buf();
    let occupied = old_path
        .parent()
        .expect("items directory")
        .join("Jane Painter - 1884 - Nocturne Study");
    fs::create_dir(&occupied).expect("occupy proposed folder");
    let proposal = vault
        .item_details(&item_id)
        .expect("reload collision-aware proposal")
        .folder_rename_proposal()
        .expect("receive rename proposal")
        .clone();
    assert_eq!(proposal.current_path(), old_path);
    assert_eq!(
        proposal.proposed_path(),
        occupied
            .parent()
            .expect("items directory")
            .join("Jane Painter - 1884 - Nocturne Study (2)")
    );

    fs::create_dir(proposal.proposed_path()).expect("occupy reviewed destination concurrently");
    let error = vault
        .confirm_item_folder_rename(&item_id, &proposal)
        .expect_err("refuse a rename whose reviewed destination became occupied");
    assert!(matches!(
        error,
        VaultError::ItemFolderRenameProposalChanged(_)
    ));
    assert!(old_path.exists());
    let refreshed_proposal = vault
        .item_details(&item_id)
        .expect("reload rename proposal")
        .folder_rename_proposal()
        .expect("receive refreshed proposal")
        .clone();
    assert!(refreshed_proposal
        .proposed_path()
        .ends_with("Jane Painter - 1884 - Nocturne Study (3)"));
    let renamed = vault
        .confirm_item_folder_rename(&item_id, &refreshed_proposal)
        .expect("confirm collision-safe rename");

    assert_eq!(renamed.id(), item_id);
    assert!(!old_path.exists());
    assert_eq!(
        renamed
            .item_folder()
            .file_name()
            .and_then(|name| name.to_str()),
        Some("Jane Painter - 1884 - Nocturne Study (3)")
    );
    assert!(renamed.item_folder().join("record.md").is_file());
    assert!(renamed.folder_rename_proposal().is_none());

    fs::remove_dir_all(root).expect("clean vault");
    fs::remove_dir_all(source_dir).expect("clean sources");
}

#[test]
fn atomic_save_never_exposes_a_partial_item_record() {
    let (root, source_dir, vault, item_id, record_path) = artwork_fixture("atomic-save");
    let loaded = vault.item_details(&item_id).expect("load editor");
    let before = fs::read_to_string(&record_path).expect("read before save");
    let started = Arc::new(Barrier::new(2));
    let finished = Arc::new(AtomicBool::new(false));
    let reader_path = record_path.clone();
    let reader_started = Arc::clone(&started);
    let reader_finished = Arc::clone(&finished);
    let reader = std::thread::spawn(move || {
        reader_started.wait();
        let mut observed = Vec::new();
        while !reader_finished.load(Ordering::Acquire) {
            observed.push(fs::read_to_string(&reader_path).expect("read during save"));
        }
        observed.push(fs::read_to_string(reader_path).expect("read after save"));
        observed
    });
    started.wait();
    vault
        .save_item_record_edit(ItemRecordEdit {
            id: item_id,
            expected_revision: loaded.record_revision().to_string(),
            overwrite_conflict: false,
            title: "Atomic Title".to_string(),
            creator: "Jane Painter".to_string(),
            year: "1884".to_string(),
            saving_reason: "Atomic replacement proof".to_string(),
            summary: "A".repeat(2_000_000),
            tags: Vec::new(),
        })
        .expect("atomically save a large record");
    finished.store(true, Ordering::Release);
    let after = fs::read_to_string(&record_path).expect("read final record");
    let observed = reader.join().expect("join concurrent reader");

    assert!(observed
        .iter()
        .all(|record| record == &before || record == &after));
    assert!(observed.iter().any(|record| record == &after));
    assert!(!fs::read_dir(record_path.parent().expect("item folder"))
        .expect("list item folder")
        .filter_map(Result::ok)
        .any(|entry| entry.file_name().to_string_lossy().starts_with(".record-")));

    fs::remove_dir_all(root).expect("clean vault");
    fs::remove_dir_all(source_dir).expect("clean sources");
}

fn artwork_fixture(name: &str) -> (PathBuf, PathBuf, Vault, String, PathBuf) {
    let root = temp_path(&format!("{name}-vault"));
    let source_dir = temp_path(&format!("{name}-source"));
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("draft.jpg");
    fs::write(&source_file, b"image bytes").expect("write source");
    let vault = Vault::create(&root).expect("create vault");
    let saved = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: None,
            year: None,
            title: "draft".to_string(),
            saving_reason: Some("Needs cleanup".to_string()),
        })
        .expect("save fixture");
    let id = saved.id().to_string();
    let record_path = saved.item_folder().join("record.md");
    (root, source_dir, vault, id, record_path)
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
