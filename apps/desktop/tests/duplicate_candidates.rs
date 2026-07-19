use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_does_not_create_review_work_for_repeated_exact_imports() {
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

    assert!(second_import.is_empty());
    let details = shell
        .item_details(first_import[0].id())
        .expect("read preserved details");

    assert!(details.duplicate_candidates().is_empty());

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
