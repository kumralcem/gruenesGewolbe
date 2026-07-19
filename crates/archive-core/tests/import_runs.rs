use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{AddArtworkItem, ImportRunAction, Vault};

#[test]
fn import_run_recursively_preserves_supported_images_and_returns_a_summary() {
    let root = temp_path("recursive-import-vault");
    let source = temp_path("recursive-import-source");
    let nested = source.join("landscapes/night");
    fs::create_dir_all(&nested).expect("create nested source folders");
    image::RgbImage::from_pixel(8, 6, image::Rgb([20, 40, 60]))
        .save(source.join("Garden - 2024 - Morning.png"))
        .expect("write top-level image");
    image::RgbImage::from_pixel(6, 8, image::Rgb([60, 40, 20]))
        .save(nested.join("Nocturne.jpg"))
        .expect("write nested image");
    let vault = Vault::create(&root).expect("create vault");

    let summary = vault
        .run_paintings_import(&source, |_| ImportRunAction::Continue)
        .expect("run recursive import");

    assert_eq!(summary.imported_count(), 2);
    assert_eq!(summary.failed_count(), 0);
    assert_eq!(summary.imported_items().len(), 2);
    assert!(summary
        .imported_items()
        .iter()
        .all(|item| item.home_subvault() == "Paintings"));

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&source).expect("clean source");
}

#[cfg(unix)]
#[test]
fn import_run_reports_unsupported_files_and_symbolic_links_as_skipped() {
    use std::os::unix::fs::symlink;

    let root = temp_path("skipped-import-vault");
    let source = temp_path("skipped-import-source");
    let external = temp_path("skipped-import-external");
    fs::create_dir_all(&source).expect("create source folder");
    fs::create_dir_all(&external).expect("create external folder");
    fs::write(source.join("notes.txt"), b"not artwork").expect("write unsupported file");
    image::RgbImage::from_pixel(4, 4, image::Rgb([10, 20, 30]))
        .save(external.join("linked.png"))
        .expect("write linked image");
    symlink(external.join("linked.png"), source.join("linked-file.png"))
        .expect("create file symlink");
    symlink(&external, source.join("linked-folder")).expect("create directory symlink");
    let vault = Vault::create(&root).expect("create vault");

    let summary = vault
        .run_paintings_import(&source, |_| ImportRunAction::Continue)
        .expect("run import");

    assert_eq!(summary.imported_count(), 0);
    assert_eq!(summary.skipped_count(), 3);
    assert_eq!(
        summary
            .skipped_entries()
            .iter()
            .map(|entry| entry.reason())
            .collect::<Vec<_>>(),
        vec!["symbolic-link", "symbolic-link", "unsupported-file"]
    );

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&source).expect("clean source");
    fs::remove_dir_all(&external).expect("clean external");
}

#[test]
fn cancellation_between_files_keeps_completed_items_and_marks_the_run_partial() {
    let root = temp_path("cancelled-import-vault");
    let source = temp_path("cancelled-import-source");
    fs::create_dir_all(&source).expect("create source folder");
    for name in ["one.png", "two.png", "three.png"] {
        image::RgbImage::from_pixel(4, 4, image::Rgb([10, 20, 30]))
            .save(source.join(name))
            .expect("write image");
    }
    let vault = Vault::create(&root).expect("create vault");

    let summary = vault
        .run_paintings_import(&source, |progress| {
            if progress.processed() == 1 {
                ImportRunAction::Cancel
            } else {
                ImportRunAction::Continue
            }
        })
        .expect("run cancellable import");

    assert!(summary.was_cancelled());
    assert_eq!(summary.imported_count(), 1);
    assert_eq!(summary.cancelled_count(), 2);
    assert_eq!(summary.cancelled_files().len(), 2);
    assert!(summary
        .cancelled_files()
        .iter()
        .all(|path| path.extension().and_then(|value| value.to_str()) == Some("png")));
    assert_eq!(
        vault
            .browse_artwork_items("Paintings")
            .expect("browse completed import")
            .len(),
        1
    );

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&source).expect("clean source");
}

#[test]
fn import_run_reports_file_progress_in_discovery_order() {
    let root = temp_path("progress-import-vault");
    let source = temp_path("progress-import-source");
    fs::create_dir_all(&source).expect("create source folder");
    for name in ["b.png", "a.png"] {
        image::RgbImage::from_pixel(4, 4, image::Rgb([10, 20, 30]))
            .save(source.join(name))
            .expect("write image");
    }
    let vault = Vault::create(&root).expect("create vault");
    let mut progress = Vec::new();

    vault
        .run_paintings_import(&source, |update| {
            progress.push((
                update.processed(),
                update.total(),
                update
                    .current_file()
                    .file_name()
                    .expect("file name")
                    .to_string_lossy()
                    .into_owned(),
            ));
            ImportRunAction::Continue
        })
        .expect("run import");

    assert_eq!(
        progress,
        vec![(0, 2, "a.png".to_string()), (1, 2, "b.png".to_string())]
    );

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&source).expect("clean source");
}

#[test]
fn a_file_failure_is_audited_without_rolling_back_other_imports() {
    let root = temp_path("partial-failure-import-vault");
    let source = temp_path("partial-failure-import-source");
    fs::create_dir_all(&source).expect("create source folder");
    let failed_file = source.join("a-vanished.png");
    let successful_file = source.join("b-survives.png");
    image::RgbImage::from_pixel(4, 4, image::Rgb([10, 20, 30]))
        .save(&failed_file)
        .expect("write failing image");
    image::RgbImage::from_pixel(4, 4, image::Rgb([30, 20, 10]))
        .save(&successful_file)
        .expect("write successful image");
    let vault = Vault::create(&root).expect("create vault");

    let summary = vault
        .run_paintings_import(&source, |progress| {
            if progress.current_file() == failed_file {
                fs::remove_file(&failed_file).expect("remove discovered file");
            }
            ImportRunAction::Continue
        })
        .expect("complete best-effort import");

    assert_eq!(summary.imported_count(), 1);
    assert_eq!(summary.failed_count(), 1);
    assert_eq!(summary.failed_entries()[0].path(), failed_file);
    let activity = fs::read_to_string(root.join(".gruenesgewolbe/activity-log.tsv"))
        .expect("read activity log");
    assert!(activity.contains("import-file-failed"));
    assert!(activity.contains("a-vanished.png"));

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&source).expect("clean source");
}

#[test]
fn import_run_refreshes_metadata_search_once_after_processing_all_files() {
    let root = temp_path("batched-index-import-vault");
    let source = temp_path("batched-index-import-source");
    fs::create_dir_all(&source).expect("create source folder");
    for name in ["First Study.png", "Second Study.png"] {
        image::RgbImage::from_pixel(4, 4, image::Rgb([10, 20, 30]))
            .save(source.join(name))
            .expect("write image");
    }
    let vault = Vault::create(&root).expect("create vault");

    vault
        .run_paintings_import(&source, |_| ImportRunAction::Continue)
        .expect("run import");

    assert_eq!(
        vault
            .search_metadata("study")
            .expect("search imported items")
            .len(),
        2
    );
    let activity = fs::read_to_string(root.join(".gruenesgewolbe/activity-log.tsv"))
        .expect("read activity log");
    assert_eq!(activity.matches("rebuild-metadata-index").count(), 1);

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&source).expect("clean source");
}

#[test]
fn ambiguous_overlap_is_imported_with_a_duplicate_candidate_review_reason() {
    let root = temp_path("candidate-import-vault");
    let source = temp_path("candidate-import-source");
    let existing_source = temp_path("candidate-existing-source");
    fs::create_dir_all(&source).expect("create import source");
    fs::create_dir_all(&existing_source).expect("create existing source");
    let existing_file = existing_source.join("original.png");
    image::RgbImage::from_pixel(4, 4, image::Rgb([10, 20, 30]))
        .save(&existing_file)
        .expect("write existing image");
    image::RgbImage::from_pixel(5, 5, image::Rgb([30, 20, 10]))
        .save(source.join("Jane Painter - 1884 - Nocturne.png"))
        .expect("write overlapping image");
    let vault = Vault::create(&root).expect("create vault");
    vault
        .add_artwork_item(AddArtworkItem {
            source_file: existing_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Jane Painter".to_string()),
            year: Some("1884".to_string()),
            title: "Nocturne".to_string(),
            saving_reason: None,
        })
        .expect("save existing item");

    let summary = vault
        .run_paintings_import(&source, |_| ImportRunAction::Continue)
        .expect("run overlapping import");

    assert_eq!(summary.imported_count(), 1);
    assert_eq!(summary.duplicate_candidate_count(), 1);
    assert_eq!(
        summary.duplicate_candidate_entries()[0].path(),
        source.join("Jane Painter - 1884 - Nocturne.png")
    );
    let imported = &summary.imported_items()[0];
    let details = vault
        .item_details(imported.id())
        .expect("open imported details");
    assert!(details
        .review_reasons()
        .iter()
        .any(|reason| reason.starts_with("duplicate-candidate |")));

    fs::remove_dir_all(&root).expect("clean vault");
    fs::remove_dir_all(&source).expect("clean source");
    fs::remove_dir_all(&existing_source).expect("clean existing source");
}

fn temp_path(name: &str) -> PathBuf {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock after epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("gruenes-gewolbe-{name}-{unique}"))
}
