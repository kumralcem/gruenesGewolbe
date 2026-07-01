use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{CollectionDefinition, ManualFallbackCapture, UpdateItemRecord};
use gruenes_gewolbe_desktop::{DesktopShell, WorkbenchRequest};

#[test]
fn desktop_shell_supports_workbench_browse_inspect_edit_and_review() {
    let root = temp_path("desktop-workbench-vault");
    let source_dir = temp_path("desktop-workbench-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(source_dir.join("mystery.jpg"), b"known painting bytes")
        .expect("write source painting");

    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    shell
        .import_paintings_folder(&source_dir)
        .expect("import paintings");

    let grid = shell
        .browse_artwork_items("Paintings")
        .expect("browse artwork grid");
    assert_eq!(grid.len(), 1);
    assert_eq!(grid[0].title(), "mystery");
    assert_eq!(grid[0].review_status(), "needs-review");

    let details = shell
        .item_details(grid[0].saved_item().id())
        .expect("open item details");
    assert_eq!(details.creator(), "Unknown Creator");
    assert_eq!(details.import_original_filename(), Some("mystery.jpg"));

    let review_queue = shell.review_queue().expect("browse review queue");
    assert_eq!(review_queue.len(), 1);
    assert_eq!(review_queue[0].saved_item().id(), details.id());
    assert_eq!(review_queue[0].home_subvault(), "Paintings");
    assert_eq!(review_queue[0].title(), "mystery");
    assert_eq!(review_queue[0].review_status(), "needs-review");

    let updated = shell
        .update_item_record(UpdateItemRecord {
            id: details.id().to_string(),
            title: Some("Nocturne Study Revised".to_string()),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            saving_reason: Some("Palette reference".to_string()),
            review_status: Some("reviewed".to_string()),
        })
        .expect("update item record");

    assert_eq!(updated.title(), "Nocturne Study Revised");
    assert_eq!(updated.review_status(), "reviewed");
    assert_eq!(
        updated.folder_rename_suggestion(),
        Some("Jane Painter - 1884 - Nocturne Study Revised")
    );
    assert!(shell
        .review_queue()
        .expect("browse cleared review queue")
        .is_empty());

    let results = shell
        .search_metadata("revised")
        .expect("search updated metadata");
    assert_eq!(results[0].saved_item().id(), updated.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn desktop_shell_builds_a_first_screen_workbench_snapshot() {
    let root = temp_path("desktop-workbench-snapshot-vault");
    let source_dir = temp_path("desktop-workbench-snapshot-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg"),
        b"known painting bytes",
    )
    .expect("write source painting");

    let mut shell = DesktopShell::default();
    let active = shell.create_vault(&root).expect("create active vault");
    let imported = shell
        .import_paintings_folder(&source_dir)
        .expect("import paintings");
    let item_id = imported[0].id().to_string();

    let collection = shell
        .create_collection(CollectionDefinition {
            name: "Night References".to_string(),
            purpose: "Visual research".to_string(),
            description: None,
        })
        .expect("create collection");
    shell
        .add_item_to_collection(collection.id(), &item_id)
        .expect("add item to collection");

    let captured = shell
        .manual_fallback_capture(ManualFallbackCapture {
            source_link: "https://example.com/nocturne-note".to_string(),
            title: "Nocturne note".to_string(),
            saving_reason: Some("Useful framing for night palettes".to_string()),
            copied_text: Some("Main idea text from the page.".to_string()),
            copied_image: None,
        })
        .expect("capture idea");

    let snapshot = shell
        .workbench_snapshot(WorkbenchRequest {
            home_subvault: "Paintings".to_string(),
            search_query: Some("nocturne".to_string()),
            selected_item_id: Some(item_id.clone()),
        })
        .expect("build workbench snapshot");

    assert_eq!(snapshot.active_vault().root(), active.root());
    assert!(snapshot
        .subvaults()
        .iter()
        .any(|subvault| subvault == "Paintings"));
    assert!(snapshot
        .subvaults()
        .iter()
        .any(|subvault| subvault == "Idea Sources"));
    assert_eq!(snapshot.collections()[0].name(), "Night References");
    assert_eq!(snapshot.artwork_items()[0].saved_item().id(), item_id);
    assert_eq!(snapshot.idea_sources()[0].title(), "Nocturne note");
    assert!(snapshot
        .review_queue()
        .iter()
        .any(|item| item.saved_item().id() == captured.id()));
    assert!(snapshot
        .search_results()
        .iter()
        .any(|result| result.saved_item().id() == item_id));
    assert_eq!(
        snapshot
            .selected_item()
            .expect("selected item")
            .collections(),
        vec!["Night References"]
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
