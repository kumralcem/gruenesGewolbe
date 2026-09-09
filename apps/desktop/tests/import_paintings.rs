use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_imports_a_paintings_folder_into_the_active_vault() {
    let root = temp_path("desktop-import-vault");
    let source_dir = temp_path("desktop-import-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg"),
        b"known painting bytes",
    )
    .expect("write source painting");

    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");
    let imported = shell
        .import_paintings_folder(&source_dir)
        .expect("import paintings folder");

    assert_eq!(imported.len(), 1);
    assert!(root
        .join("subvaults")
        .join("Paintings")
        .join("items")
        .join("Jane Painter - 1884 - Nocturne Study")
        .join("record.md")
        .is_file());
    let grid = shell
        .browse_artwork_items("Paintings")
        .expect("browse imported paintings");
    assert_eq!(grid.len(), 1);
    assert!(grid[0].thumbnail_file().is_file());
    assert!(grid[0]
        .thumbnail_file()
        .starts_with(root.join(".gruenesgewolbe").join("thumbnails")));

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
