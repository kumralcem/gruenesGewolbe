use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn cli_imports_a_paintings_folder_into_a_specified_vault() {
    let root = temp_path("cli-import-vault");
    let source_dir = temp_path("cli-import-source");
    fs::create_dir_all(&source_dir).expect("create source directory");
    fs::write(
        source_dir.join("Jane Painter - 1884 - Nocturne Study.jpg"),
        b"known painting bytes",
    )
    .expect("write source painting");

    let create = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("create")
        .arg(&root)
        .output()
        .expect("run create command");
    assert!(create.status.success());

    let import = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("import-paintings")
        .arg(&root)
        .arg(&source_dir)
        .output()
        .expect("run import command");

    assert!(import.status.success());
    assert_eq!(
        String::from_utf8(import.stdout).expect("import stdout"),
        format!("imported-paintings\t{}\t1\n", root.display())
    );

    assert!(root
        .join("subvaults")
        .join("Paintings")
        .join("items")
        .join("Jane Painter - 1884 - Nocturne Study")
        .join("record.md")
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
