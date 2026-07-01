use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, ExtractedTextCapture, UpdateItemRecord, Vault};

#[test]
fn user_can_browse_artwork_items_and_open_item_details_from_records() {
    let root = temp_path("workbench-browse-vault");
    let source_dir = temp_path("workbench-browse-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let nocturne_source = source_dir.join("nocturne.jpg");
    let garden_source = source_dir.join("garden.png");
    fs::write(&nocturne_source, b"nocturne bytes").expect("write nocturne");
    fs::write(&garden_source, b"garden bytes").expect("write garden");

    let vault = Vault::create(&root).expect("create vault");
    let nocturne = vault
        .add_artwork_item(AddArtworkItem {
            source_file: nocturne_source.clone(),
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: Some("Palette reference for night scenes".to_string()),
        })
        .expect("save nocturne");
    vault
        .add_artwork_item(AddArtworkItem {
            source_file: garden_source,
            home_subvault: "Paintings".to_string(),
            creator: Some("Lee Artist".to_string()),
            year: Some("2024".to_string()),
            title: "Garden Window".to_string(),
            saving_reason: Some("Composition reference".to_string()),
        })
        .expect("save garden");

    let grid = vault
        .browse_artwork_items("Paintings")
        .expect("browse artwork items");
    assert_eq!(grid.len(), 2);
    assert_eq!(grid[0].title(), "Garden Window");
    assert_eq!(grid[0].creator(), "Lee Artist");
    assert_eq!(grid[0].year(), "2024");
    assert_eq!(grid[0].review_status(), "needs-review");
    assert!(grid[0].primary_file().ends_with("files/garden.png"));
    assert_eq!(grid[1].title(), "Nocturne Study");

    let details = vault
        .item_details(nocturne.id())
        .expect("open item details");
    assert_eq!(details.id(), nocturne.id());
    assert_eq!(details.home_subvault(), "Paintings");
    assert_eq!(details.title(), "Nocturne Study");
    assert_eq!(details.creator(), "Jane Painter");
    assert_eq!(details.year(), "1884");
    assert_eq!(details.review_status(), "needs-review");
    assert_eq!(
        details.saving_reason(),
        Some("Palette reference for night scenes")
    );
    assert_eq!(details.import_original_filename(), Some("nocturne.jpg"));
    assert_eq!(
        details.import_source_path(),
        Some(nocturne_source.as_path())
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn browsing_artwork_items_rebuilds_cached_thumbnail_previews_from_preserved_files() {
    let root = temp_path("workbench-thumbnail-vault");
    let source_dir = temp_path("workbench-thumbnail-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("nocturne.jpg");
    fs::write(&source_file, b"nocturne image bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: Some("Palette reference".to_string()),
        })
        .expect("save artwork");

    let grid = vault
        .browse_artwork_items("Paintings")
        .expect("browse artwork grid");
    assert_eq!(grid.len(), 1);
    assert!(grid[0].thumbnail_file().is_file());
    assert!(grid[0]
        .thumbnail_file()
        .starts_with(root.join(".gruenesgewolbe").join("thumbnails")));
    assert_eq!(
        fs::read(grid[0].thumbnail_file()).expect("read cached thumbnail"),
        b"nocturne image bytes"
    );

    fs::remove_dir_all(root.join(".gruenesgewolbe")).expect("delete derived state");
    let rebuilt_grid = vault
        .browse_artwork_items("Paintings")
        .expect("browse artwork grid after deleting cache");
    assert_eq!(rebuilt_grid[0].saved_item().id(), saved_item.id());
    assert_eq!(
        fs::read(rebuilt_grid[0].thumbnail_file()).expect("read rebuilt thumbnail"),
        b"nocturne image bytes"
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn user_can_browse_idea_sources_as_a_text_list_and_open_source_link_details() {
    let root = temp_path("workbench-idea-sources-vault");
    let vault = Vault::create(&root).expect("create vault");
    let archive_note = vault
        .capture_extracted_text(ExtractedTextCapture {
            source_link: "https://example.com/archive-note".to_string(),
            title: "Archive note".to_string(),
            saving_reason: Some("Useful source material".to_string()),
            cleaned_text: "Cleaned article text.".to_string(),
        })
        .expect("capture archive note");
    vault
        .capture_extracted_text(ExtractedTextCapture {
            source_link: "https://example.com/zettelkasten".to_string(),
            title: "Zettelkasten reference".to_string(),
            saving_reason: Some("Note-taking source".to_string()),
            cleaned_text: "Another cleaned article.".to_string(),
        })
        .expect("capture zettelkasten note");

    let ideas = vault.browse_idea_sources().expect("browse idea sources");

    assert_eq!(ideas.len(), 2);
    assert_eq!(ideas[0].saved_item().id(), archive_note.id());
    assert_eq!(ideas[0].title(), "Archive note");
    assert_eq!(ideas[0].source_link(), "https://example.com/archive-note");
    assert_eq!(ideas[0].review_status(), "needs-review");
    assert_eq!(ideas[0].reason(), Some("Useful source material"));
    assert_eq!(
        ideas[0].source_copy(),
        Some(Path::new("source-copies/cleaned-text.md"))
    );
    assert_eq!(ideas[1].title(), "Zettelkasten reference");

    let details = vault
        .item_details(archive_note.id())
        .expect("open idea details");
    assert_eq!(
        details.source_link(),
        Some("https://example.com/archive-note")
    );
    assert_eq!(
        details.source_copy(),
        Some(Path::new("source-copies/cleaned-text.md"))
    );
    assert_eq!(details.saving_reason(), Some("Useful source material"));

    fs::remove_dir_all(&root).expect("clean temp vault");
}

#[test]
fn user_can_edit_item_record_fields_and_clear_review_status() {
    let root = temp_path("workbench-edit-vault");
    let source_dir = temp_path("workbench-edit-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("draft.jpg");
    fs::write(&source_file, b"draft bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: None,
            year: None,
            title: "draft".to_string(),
            saving_reason: Some("Needs cleanup".to_string()),
        })
        .expect("save draft");

    vault
        .update_item_record(UpdateItemRecord {
            id: saved_item.id().to_string(),
            title: Some("Nocturne Study".to_string()),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            saving_reason: Some("Palette reference for night scenes".to_string()),
            review_status: Some("reviewed".to_string()),
        })
        .expect("update item record");

    let details = vault
        .item_details(saved_item.id())
        .expect("open updated details");
    assert_eq!(details.title(), "Nocturne Study");
    assert_eq!(details.creator(), "Jane Painter");
    assert_eq!(details.year(), "1884");
    assert_eq!(details.review_status(), "reviewed");
    assert_eq!(
        details.saving_reason(),
        Some("Palette reference for night scenes")
    );

    let record = fs::read_to_string(saved_item.item_folder().join("record.md"))
        .expect("read updated record");
    assert!(record.contains("title: Nocturne Study"));
    assert!(record.contains("creator: Jane Painter"));
    assert!(record.contains("year: \"1884\""));
    assert!(record.contains("review_status: reviewed"));
    assert!(record.contains("## Saving Reason\n\nPalette reference for night scenes\n"));

    vault
        .rebuild_metadata_index()
        .expect("refresh derived index");
    let results = vault
        .search_metadata("jane painter")
        .expect("search updated metadata");
    assert_eq!(results[0].saved_item().id(), saved_item.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn metadata_cleanup_suggests_a_folder_rename_without_moving_the_item_folder() {
    let root = temp_path("workbench-folder-rename-vault");
    let source_dir = temp_path("workbench-folder-rename-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("draft.jpg");
    fs::write(&source_file, b"draft bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: None,
            year: None,
            title: "draft".to_string(),
            saving_reason: Some("Needs cleanup".to_string()),
        })
        .expect("save draft");
    let original_folder = saved_item.item_folder().to_path_buf();

    let updated = vault
        .update_item_record(UpdateItemRecord {
            id: saved_item.id().to_string(),
            title: Some("Nocturne Study".to_string()),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            saving_reason: None,
            review_status: None,
        })
        .expect("update item metadata");

    assert_eq!(updated.item_folder(), original_folder.as_path());
    assert!(original_folder.is_dir());
    assert!(!original_folder
        .parent()
        .expect("items directory")
        .join("Jane Painter - 1884 - Nocturne Study")
        .exists());
    assert_eq!(
        updated.folder_rename_suggestion(),
        Some("Jane Painter - 1884 - Nocturne Study")
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn direct_item_record_edits_are_reflected_after_rebuild() {
    let root = temp_path("workbench-file-edit-vault");
    let source_dir = temp_path("workbench-file-edit-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("draft.jpg");
    fs::write(&source_file, b"draft bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Unknown Creator".to_string()),
            year: Some("Unknown Year".to_string()),
            title: "draft".to_string(),
            saving_reason: None,
        })
        .expect("save draft");

    let record_path = saved_item.item_folder().join("record.md");
    let record = fs::read_to_string(&record_path).expect("read record");
    fs::write(
        &record_path,
        record
            .replace("title: draft", "title: File Edited Title")
            .replace("review_status: needs-review", "review_status: reviewed"),
    )
    .expect("write direct file edit");

    vault
        .rebuild_metadata_index()
        .expect("rebuild metadata index");

    let details = vault.item_details(saved_item.id()).expect("read details");
    assert_eq!(details.title(), "File Edited Title");
    assert_eq!(details.review_status(), "reviewed");

    let results = vault
        .search_metadata("file edited title")
        .expect("search edited title");
    assert_eq!(results[0].saved_item().id(), saved_item.id());

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn user_can_browse_a_cross_subvault_review_queue_for_uncertain_items() {
    let root = temp_path("workbench-review-queue-vault");
    let source_dir = temp_path("workbench-review-queue-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let artwork_source = source_dir.join("unknown.jpg");
    fs::write(&artwork_source, b"unknown artwork bytes").expect("write artwork source");

    let vault = Vault::create(&root).expect("create vault");
    let artwork = vault
        .add_artwork_item(AddArtworkItem {
            source_file: artwork_source,
            home_subvault: "Paintings".to_string(),
            creator: None,
            year: None,
            title: "unknown".to_string(),
            saving_reason: Some("Needs attribution".to_string()),
        })
        .expect("save artwork");
    let idea = vault
        .capture_extracted_text(ExtractedTextCapture {
            source_link: "https://example.com/archive-note".to_string(),
            title: "Archive note".to_string(),
            saving_reason: Some("Useful source material".to_string()),
            cleaned_text: "Cleaned article text.".to_string(),
        })
        .expect("capture idea");

    vault
        .update_item_record(UpdateItemRecord {
            id: artwork.id().to_string(),
            title: None,
            creator: None,
            year: None,
            saving_reason: None,
            review_status: Some("reviewed".to_string()),
        })
        .expect("clear artwork review");

    let queue = vault.review_queue().expect("browse review queue");

    assert_eq!(queue.len(), 1);
    assert_eq!(queue[0].saved_item().id(), idea.id());
    assert_eq!(queue[0].home_subvault(), "Idea Sources");
    assert_eq!(queue[0].title(), "Archive note");
    assert_eq!(queue[0].item_type(), "idea");
    assert_eq!(queue[0].review_status(), "needs-review");
    assert_eq!(queue[0].reason(), Some("Useful source material"));

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
