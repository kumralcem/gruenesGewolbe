use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::AddArtworkItem;
use gruenes_gewolbe_desktop::DesktopShell;

#[test]
fn desktop_shell_adds_a_local_image_to_the_active_vault() {
    let root = temp_path("desktop-artwork-vault");
    let source_dir = temp_path("desktop-artwork-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let source_file = source_dir.join("window-study.webp");
    fs::write(&source_file, b"original webp bytes").expect("write source image");

    let mut shell = DesktopShell::default();
    shell.create_vault(&root).expect("create active vault");

    let saved_item = shell
        .add_artwork_item(AddArtworkItem {
            source_file,
            home_subvault: "Paintings".to_string(),
            creator: Some("Lee Artist".to_string()),
            year: Some("2024".to_string()),
            title: "Window Study".to_string(),
            saving_reason: Some("Composition reference".to_string()),
        })
        .expect("add artwork item");

    assert_eq!(saved_item.home_subvault(), "Paintings");
    assert!(saved_item
        .item_folder()
        .join("files")
        .join("window-study.webp")
        .is_file());
    assert!(saved_item.item_folder().join("record.md").is_file());

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
