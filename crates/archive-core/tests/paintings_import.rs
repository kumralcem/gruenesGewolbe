use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use gruenes_gewolbe_core::Vault;

#[test]
fn user_can_import_a_paintings_folder_copying_supported_images_by_default() {
    let root = temp_path("paintings-import-vault");
    let source_dir = temp_path("paintings-import-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    let known_source = source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg");
    let unknown_source = source_dir.join("mystery.png");
    let ignored_source = source_dir.join("notes.txt");
    fs::write(&known_source, b"known painting bytes").expect("write known painting");
    fs::write(&unknown_source, b"unknown painting bytes").expect("write unknown painting");
    fs::write(&ignored_source, b"not an image").expect("write ignored source");

    let vault = Vault::create(&root).expect("create vault");
    let imported = vault
        .import_paintings_folder(&source_dir)
        .expect("import paintings folder");

    assert_eq!(imported.len(), 2);
    assert_eq!(
        fs::read(&known_source).expect("read known source"),
        b"known painting bytes"
    );
    assert_eq!(
        fs::read(&unknown_source).expect("read unknown source"),
        b"unknown painting bytes"
    );
    assert!(ignored_source.is_file());

    let known_folder = root
        .join("subvaults")
        .join("Paintings")
        .join("items")
        .join("Jane Painter - 1884 - Nocturne Study");
    assert!(known_folder
        .join("files")
        .join("Jane Painter - 1884 - Nocturne Study.jpg")
        .is_file());
    assert_eq!(
        fs::read(
            known_folder
                .join("files")
                .join("Jane Painter - 1884 - Nocturne Study.jpg")
        )
        .expect("read preserved known file"),
        b"known painting bytes"
    );

    let known_record = fs::read_to_string(known_folder.join("record.md")).expect("read record");
    assert!(known_record.contains("home_subvault: Paintings"));
    assert!(known_record.contains("title: Nocturne Study"));
    assert!(known_record.contains("creator: Jane Painter"));
    assert!(known_record.contains("year: '1884'"));
    assert!(
        known_record.contains("import_original_filename: Jane Painter - 1884 - Nocturne Study.jpg")
    );
    assert!(known_record.contains(&format!("import_source_folder: {}", source_dir.display())));
    assert!(known_record.contains("imported_at: "));
    assert!(known_record.contains("review_status: reviewed"));

    let unknown_folder = root
        .join("subvaults")
        .join("Paintings")
        .join("items")
        .join("Unknown Creator - Unknown Year - mystery");
    let unknown_record =
        fs::read_to_string(unknown_folder.join("record.md")).expect("read unknown record");
    assert!(unknown_record.contains("creator: Unknown Creator"));
    assert!(unknown_record.contains("year: Unknown Year"));
    assert!(unknown_record.contains("title: mystery"));
    assert!(unknown_record.contains("review_status: needs-review"));

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
