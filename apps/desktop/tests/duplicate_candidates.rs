use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_shows_duplicate_candidate_warnings_without_blocking_import() {
    let root = temp_path("desktop-duplicate-vault");
    let source_dir = temp_path("desktop-duplicate-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg"),
        b"same image bytes",
    )
    .expect("write source image");

    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    let first_import = shell
        .import_paintings_folder(&source_dir)
        .expect("first import");
    let second_import = shell
        .import_paintings_folder(&source_dir)
        .expect("second import");

    assert_eq!(second_import.len(), 1);
    let details = shell
        .item_details(second_import[0].id())
        .expect("read duplicate details");

    assert!(details
        .duplicate_candidates()
        .iter()
        .any(|candidate| candidate.item_id() == first_import[0].id()));

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
