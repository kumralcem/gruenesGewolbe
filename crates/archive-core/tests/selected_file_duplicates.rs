use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, ExactDuplicatePolicy, ImportRunOptions, Vault};

#[test]
fn selected_file_exact_duplicates_are_skipped_and_can_be_imported_deliberately() {
    let root = temp_path("selected-exact-vault");
    let first_source = temp_path("selected-exact-first");
    let second_source = temp_path("selected-exact-second");
    fs::create_dir_all(&first_source).expect("create first source");
    fs::create_dir_all(&second_source).expect("create second source");
    let original = first_source.join("original.png");
    let renamed = second_source.join("renamed.png");
    write_image(&original, [10, 20, 30]);
    fs::copy(&original, &renamed).expect("copy exact bytes under another name");

    let vault = Vault::create(&root).expect("create vault");
    let existing = vault
        .add_artwork_item(artwork(original, "Existing"))
        .expect("save existing item");

    let skipped = vault
        .add_artwork_files_with_options([renamed.clone()], ImportRunOptions::default())
        .expect("check selected duplicate");
    assert_eq!(skipped.imported_count(), 0);
    assert_eq!(skipped.exact_duplicate_count(), 1);
    assert_eq!(skipped.skipped_entries()[0].path(), renamed);
    assert_eq!(
        skipped.skipped_entries()[0].existing_item_id(),
        Some(existing.id())
    );

    let imported = vault
        .add_artwork_files_with_options(
            [renamed],
            ImportRunOptions {
                exact_duplicate_policy: ExactDuplicatePolicy::ImportAnyway,
                ..ImportRunOptions::default()
            },
        )
        .expect("override selected duplicate");
    assert_eq!(imported.imported_count(), 1);
    assert_eq!(imported.exact_duplicate_count(), 0);
    assert_eq!(imported.duplicate_candidate_count(), 0);

    fs::remove_dir_all(root).expect("clean vault");
    fs::remove_dir_all(first_source).expect("clean first source");
    fs::remove_dir_all(second_source).expect("clean second source");
}

#[test]
fn selected_file_filename_only_overlap_remains_a_duplicate_candidate() {
    let root = temp_path("selected-ambiguous-vault");
    let first_source = temp_path("selected-ambiguous-first");
    let second_source = temp_path("selected-ambiguous-second");
    fs::create_dir_all(&first_source).expect("create first source");
    fs::create_dir_all(&second_source).expect("create second source");
    let first_path = first_source.join("work.png");
    let second_path = second_source.join("work.png");
    write_image(&first_path, [10, 20, 30]);
    write_image(&second_path, [30, 20, 10]);
    let vault = Vault::create(&root).expect("create vault");
    vault
        .add_artwork_item(AddArtworkItem {
            source_file: first_path,
            home_subvault: "Paintings".to_string(),
            creator: None,
            year: None,
            title: "First work".to_string(),
            saving_reason: None,
        })
        .expect("save first version");

    let summary = vault
        .add_artwork_files_with_options([second_path], ImportRunOptions::default())
        .expect("import nonidentical overlap");

    assert_eq!(summary.imported_count(), 1);
    assert_eq!(summary.exact_duplicate_count(), 0);
    assert_eq!(summary.duplicate_candidate_count(), 1);
    let details = vault
        .item_details(summary.imported_items()[0].id())
        .expect("read imported details");
    assert_eq!(details.duplicate_candidates()[0].signal(), "filename");

    fs::remove_dir_all(root).expect("clean vault");
    fs::remove_dir_all(first_source).expect("clean first source");
    fs::remove_dir_all(second_source).expect("clean second source");
}

#[test]
fn unsupported_selected_file_does_not_block_supported_selected_file() {
    let root = temp_path("selected-unsupported-vault");
    let source = temp_path("selected-unsupported-source");
    fs::create_dir_all(&source).expect("create source");
    let supported = source.join("work.png");
    let unsupported = source.join("notes.txt");
    write_image(&supported, [10, 20, 30]);
    fs::write(&unsupported, b"notes").expect("write unsupported file");
    let vault = Vault::create(&root).expect("create vault");

    let summary = vault
        .add_artwork_files_with_options(
            [unsupported.clone(), supported],
            ImportRunOptions::default(),
        )
        .expect("process selected files independently");

    assert_eq!(summary.imported_count(), 1);
    assert_eq!(summary.skipped_count(), 1);
    assert_eq!(summary.skipped_entries()[0].path(), unsupported);
    assert_eq!(summary.skipped_entries()[0].reason(), "unsupported-file");
    let activity_log = fs::read_to_string(root.join(".gruenesgewolbe/activity-log.tsv"))
        .expect("read activity log");
    assert!(activity_log.contains("\tselected-files-import-completed\tselected-files\t"));
    assert!(!activity_log.contains("\timport-run-completed\tselected-files\t"));

    fs::remove_dir_all(root).expect("clean vault");
    fs::remove_dir_all(source).expect("clean source");
}

fn artwork(source_file: PathBuf, title: &str) -> AddArtworkItem {
    AddArtworkItem {
        source_file,
        home_subvault: "Paintings".to_string(),
        creator: Some("Artist".to_string()),
        year: Some("2024".to_string()),
        title: title.to_string(),
        saving_reason: None,
    }
}

fn write_image(path: &std::path::Path, color: [u8; 3]) {
    image::RgbImage::from_pixel(4, 4, image::Rgb(color))
        .save(path)
        .expect("write image");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
