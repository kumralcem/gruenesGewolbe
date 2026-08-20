use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, Vault};

#[test]
fn malformed_item_records_are_localized_across_browsing_and_metadata_rebuilds() {
    let root = temp_path("localized-malformed-records-vault");
    let source_dir = temp_path("localized-malformed-records-source");
    fs::create_dir_all(&source_dir).expect("create source directory");

    let vault = Vault::create(&root).expect("create vault");
    let mut saved_items = Vec::new();
    for title in ["Nocturne", "Garden", "Harbor", "Orchard"] {
        let source_file = source_dir.join(format!("{title}.png"));
        image::RgbImage::from_pixel(4, 4, image::Rgb([20, 30, 40]))
            .save(&source_file)
            .expect("write source image");
        saved_items.push(
            vault
                .add_artwork_item(AddArtworkItem {
                    source_file,
                    home_subvault: "Paintings".to_string(),
                    creator: Some("Archive Artist".to_string()),
                    year: Some("2024".to_string()),
                    title: title.to_string(),
                    saving_reason: None,
                })
                .expect("save artwork"),
        );
    }

    let malformed_paths = [
        saved_items[1].item_folder().join("record.md"),
        saved_items[3].item_folder().join("record.md"),
    ];
    let valid_records = [
        fs::read(&malformed_paths[0]).expect("read first valid record before corruption"),
        fs::read(&malformed_paths[1]).expect("read second valid record before corruption"),
    ];
    let malformed_record = b"---\nid: [not valid yaml\n---\n\n# Broken\n";
    for path in &malformed_paths {
        fs::write(path, malformed_record).expect("write malformed record");
    }

    let reopened = Vault::open(&root).expect("open around malformed Item Record");
    let artwork = reopened
        .browse_artwork_items("Paintings")
        .expect("browse valid Saved Items");
    assert_eq!(artwork.len(), 2);
    assert!(artwork.iter().any(|item| item.title() == "Nocturne"));
    assert!(artwork.iter().any(|item| item.title() == "Harbor"));

    let problems = reopened.vault_problems().expect("list localized Vault Problems");
    assert_eq!(problems.len(), 2);
    for problem in &problems {
        assert!(malformed_paths.iter().any(|path| path == problem.path()));
        assert!(problem.error().contains("YAML"));
        assert_eq!(
            fs::read(problem.path()).expect("read malformed Item Record after scan"),
            malformed_record
        );
    }

    let rebuilt = reopened
        .rebuild_metadata_index()
        .expect("rebuild around malformed Item Record");
    assert_eq!(rebuilt.indexed_items(), 2);
    assert_eq!(rebuilt.omitted_paths(), malformed_paths.as_slice());
    assert_eq!(
        reopened
            .search_metadata("archive artist")
            .expect("search valid Saved Items")
            .len(),
        2
    );

    for (path, record) in malformed_paths.iter().zip(valid_records) {
        fs::write(path, record).expect("correct Item Record externally");
    }
    let rebuilt = reopened
        .rebuild_metadata_index()
        .expect("refresh corrected Item Record");
    assert_eq!(rebuilt.indexed_items(), 4);
    assert!(rebuilt.omitted_paths().is_empty());
    assert!(reopened.vault_problems().expect("rescan Vault").is_empty());
    assert_eq!(
        reopened
            .search_metadata("garden")
            .expect("search corrected Saved Item")
            .len(),
        1
    );

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
