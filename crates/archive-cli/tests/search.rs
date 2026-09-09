use std::fs;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn cli_rebuilds_the_metadata_index_and_searches_a_vault() {
    let root = temp_path("cli-search-vault");
    let source_dir = temp_path("cli-search-source");
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

    fs::remove_dir_all(root.join(".gruenesgewolbe")).expect("delete derived state");

    let rebuild = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("rebuild-index")
        .arg(&root)
        .output()
        .expect("run rebuild command");
    assert!(rebuild.status.success());
    assert_eq!(
        String::from_utf8(rebuild.stdout).expect("rebuild stdout"),
        format!("rebuilt-metadata-index\t{}\t1\n", root.display())
    );

    let search = Command::new(env!("CARGO_BIN_EXE_ggvault"))
        .arg("search")
        .arg(&root)
        .arg("jane painter")
        .output()
        .expect("run search command");
    assert!(search.status.success());

    let stdout = String::from_utf8(search.stdout).expect("search stdout");
    assert!(stdout.starts_with("search-result\t"));
    assert!(stdout.contains("\tPaintings\t"));
    assert!(stdout.contains("Jane Painter - 1884 - Nocturne Study"));

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
