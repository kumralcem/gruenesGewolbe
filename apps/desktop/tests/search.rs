use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_searches_the_active_vault_and_opens_a_matching_saved_item() {
    let root = temp_path("desktop-search-vault");
    let source_dir = temp_path("desktop-search-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg"),
        b"known painting bytes",
    )
    .expect("write source painting");

    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    shell
        .import_paintings_folder(&source_dir)
        .expect("import paintings");

    fs::remove_dir_all(root.join(".gruenesgewolbe")).expect("delete derived state");
    let results = shell.search_metadata("nocturne").expect("search metadata");

    assert_eq!(results.len(), 1);
    let saved_item = shell
        .open_saved_item(results[0].saved_item().id())
        .expect("open matching item");
    assert_eq!(saved_item.home_subvault(), "Paintings");
    assert!(saved_item
        .item_folder()
        .join("files")
        .join("Jane Painter - 1884 - Nocturne Study.jpg")
        .is_file());

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
