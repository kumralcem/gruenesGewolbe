use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, Vault};

#[test]
fn user_can_add_a_local_image_as_an_artwork_saved_item() {
    let root = temp_path("artwork-item");
    let source_dir = temp_path("artwork-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("source-image.jpg");
    fs::write(&source_file, b"original jpeg bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file: source_file.clone(),
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne Study".to_string(),
            saving_reason: Some("Palette reference for night scenes".to_string()),
        })
        .expect("save artwork item");

    uuid::Uuid::parse_str(
        saved_item
            .id()
            .strip_prefix("item-")
            .expect("stable item ID prefix"),
    )
    .expect("collision-resistant UUID item ID");
    assert_eq!(saved_item.home_subvault(), "Paintings");
    assert_eq!(
        saved_item.item_folder(),
        root.join("subvaults")
            .join("Paintings")
            .join("items")
            .join("Jane Painter - 1884 - Nocturne Study")
            .as_path()
    );

    let preserved_file = saved_item
        .item_folder()
        .join("files")
        .join("source-image.jpg");
    assert_eq!(
        fs::read(&preserved_file).expect("read preserved file"),
        b"original jpeg bytes"
    );

    let item_record =
        fs::read_to_string(saved_item.item_folder().join("record.md")).expect("read item record");
    assert!(item_record.contains("item_type: artwork"));
    assert!(item_record.contains("home_subvault: Paintings"));
    assert!(item_record.contains("title: Nocturne Study"));
    assert!(item_record.contains("creator: Jane Painter"));
    assert!(item_record.contains("year: '1884'"));
    assert!(item_record.contains("primary_file: files/source-image.jpg"));
    assert!(item_record.contains("import_original_filename: source-image.jpg"));
    assert!(item_record.contains(&format!("import_source_path: {}", source_file.display())));
    assert!(item_record.contains("review_status: reviewed"));
    assert!(item_record.contains(&format!("id: {}", saved_item.id())));
    assert!(item_record.contains("## Saving Reason\n\nPalette reference for night scenes\n"));

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn saved_artwork_item_can_be_reopened_from_disk_by_stable_id() {
    let root = temp_path("reopen-artwork");
    let source_dir = temp_path("reopen-artwork-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("study.png");
    fs::write(&source_file, b"original png bytes").expect("write source image");

    let vault = Vault::create(&root).expect("create vault");
    let saved_item = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: None,
            year: None,
            title: "Untitled Window Study".to_string(),
            saving_reason: None,
        })
        .expect("save artwork item");

    let reopened_vault = Vault::open(&root).expect("reopen vault");
    let reopened_item = reopened_vault
        .open_saved_item(saved_item.id())
        .expect("reopen saved item");

    assert_eq!(reopened_item.id(), saved_item.id());
    assert_eq!(reopened_item.home_subvault(), "Paintings");
    assert_eq!(reopened_item.item_folder(), saved_item.item_folder());
    assert_eq!(
        fs::read(reopened_item.item_folder().join("files").join("study.png"))
            .expect("read preserved file"),
        b"original png bytes"
    );

    fs::remove_dir_all(&root).expect("clean temp vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

#[test]
fn artwork_item_record_has_parseable_structured_frontmatter() {
    let root = temp_path("structured-artwork-record");
    let source_dir = temp_path("structured-artwork-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("blue-study.png");
    fs::write(&source_file, b"preserved bytes").expect("write source image");
    let vault = Vault::create(&root).expect("create vault");

    let saved = vault
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Painter: Jane".to_string()),
            year: Some("2024".to_string()),
            title: "Blue #2: Study".to_string(),
            saving_reason: Some("Compare blue: green relationships".to_string()),
        })
        .expect("save artwork");

    let record = fs::read_to_string(saved.item_folder().join("record.md")).expect("read record");
    let frontmatter = record
        .strip_prefix("---\n")
        .and_then(|record| record.split_once("\n---\n"))
        .map(|(frontmatter, _)| frontmatter)
        .expect("extract frontmatter");
    let parsed: serde_yaml::Value = serde_yaml::from_str(frontmatter).expect("parse YAML");

    assert_eq!(parsed["title"], "Blue #2: Study");
    assert_eq!(parsed["creator"], "Painter: Jane");
    assert_eq!(parsed["year"], "2024");
    assert_eq!(parsed["home_subvault"], "Paintings");
    assert_eq!(
        vault
            .item_details(saved.id())
            .expect("read details")
            .title(),
        "Blue #2: Study"
    );

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&source_dir).expect("clean source directory");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();

    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
