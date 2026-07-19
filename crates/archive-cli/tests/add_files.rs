use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::Vault;

#[test]
fn cli_adds_selected_artwork_files_and_returns_structured_results() {
    let root = temp_path("cli-add-files-vault");
    let source_dir = temp_path("cli-add-files-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let first = source_dir.join("Jane Painter - 1884 - Nocturne.jpg");
    let second = source_dir.join("mystery.png");
    fs::write(&first, b"first preserved bytes").expect("write first source");
    fs::write(&second, b"second preserved bytes").expect("write second source");
    Vault::create(&root).expect("create vault");

    let output = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .args([
            "add-files",
            root.to_str().expect("vault path"),
            first.to_str().expect("first path"),
            second.to_str().expect("second path"),
        ])
        .output()
        .expect("run add-files command");

    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).expect("UTF-8 stdout");
    let lines = stdout.lines().collect::<Vec<_>>();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("added-artwork\titem-"));
    assert!(lines[0].contains("\tPaintings\t"));
    assert!(lines[1].starts_with("added-artwork\titem-"));
    assert!(root
        .join("subvaults/Paintings/items/Jane Painter - 1884 - Nocturne/files")
        .join(first.file_name().expect("first filename"))
        .is_file());
    assert!(root
        .join("subvaults/Paintings/items/Unknown Creator - Unknown Year - mystery/files")
        .join(second.file_name().expect("second filename"))
        .is_file());

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
