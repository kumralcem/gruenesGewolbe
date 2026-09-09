use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::{ManualFallbackCapture, Vault};

#[test]
fn capture_import_and_index_rebuild_append_noncanonical_activity_log_events() {
    let root = temp_path("activity-log-vault");
    let source_dir = temp_path("activity-log-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg"),
        b"known painting bytes",
    )
    .expect("write source painting");

    let vault = Vault::create(&root).expect("create vault");
    let imported = vault
        .import_paintings_folder(&source_dir)
        .expect("import paintings");
    let captured = vault
        .manual_fallback_capture(ManualFallbackCapture {
            source_link: "https://example.com/nocturne-note".to_string(),
            title: "Nocturne note".to_string(),
            saving_reason: Some("Useful framing for night palettes".to_string()),
            copied_text: Some("Main idea text from the page.".to_string()),
            copied_image: None,
        })
        .expect("capture source");
    vault
        .rebuild_metadata_index()
        .expect("rebuild metadata index");

    let activity_log = root.join(".gruenesgewolbe").join("activity-log.tsv");
    let log = fs::read_to_string(&activity_log).expect("read activity log");
    assert!(log.contains(&format!(
        "\timport-paintings\t{}\t1\n",
        source_dir.display()
    )));
    assert!(log.contains(&format!(
        "\tcapture\t{}\tIdea Sources\tmanual-fallback\n",
        captured.id()
    )));
    assert!(log.contains("\trebuild-metadata-index\t2\n"));

    fs::remove_file(&activity_log).expect("delete activity log");
    let reopened = Vault::open(&root).expect("open without activity log");
    let rebuilt = reopened
        .rebuild_metadata_index()
        .expect("rebuild without activity log");
    assert_eq!(rebuilt.indexed_items(), 2);
    let results = reopened
        .search_metadata("jane painter")
        .expect("search without original activity log");
    assert_eq!(results[0].saved_item().id(), imported[0].id());

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
